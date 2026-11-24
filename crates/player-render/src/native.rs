use libloading::Library;
use std::{
    ffi::{CStr, CString, c_char, c_int, c_ulong, c_void},
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, SyncSender},
    },
    thread,
    time::Duration,
};

pub(crate) use crate::render_contract::{Handle, Param};
#[repr(C)]
struct Event {
    kind: c_int,
    error: c_int,
    userdata: u64,
    data: *mut c_void,
}
#[repr(C)]
struct End {
    reason: c_int,
    error: c_int,
}
#[repr(C)]
struct LogMessage {
    prefix: *const c_char,
    level: *const c_char,
    text: *const c_char,
    log_level: c_int,
}

// Minimal mpv_node ABI used only to set one script option without list parsing.
#[repr(C)]
union NodeValue {
    string: *mut c_char,
    int64: i64,
    double: f64,
    list: *mut NodeList,
}
#[repr(C)]
struct Node {
    value: NodeValue,
    format: c_int,
}
#[repr(C)]
struct NodeList {
    num: c_int,
    values: *mut Node,
    keys: *mut *mut c_char,
}

pub(crate) struct Api {
    pub handle: Handle,
    _library: Library,
    command: unsafe extern "C" fn(Handle, *const *const c_char) -> c_int,
    property: unsafe extern "C" fn(Handle, *const c_char) -> *mut c_char,
    set_property: unsafe extern "C" fn(Handle, *const c_char, c_int, *mut c_void) -> c_int,
    free: unsafe extern "C" fn(*mut c_void),
    wait_event: unsafe extern "C" fn(Handle, f64) -> *const Event,
    destroy: unsafe extern "C" fn(Handle),
    destroy_uninitialized: unsafe extern "C" fn(Handle),
    initialized: bool,
    pub render_create: unsafe extern "C" fn(*mut Handle, Handle, *mut Param) -> c_int,
    pub render: unsafe extern "C" fn(Handle, *mut Param) -> c_int,
    pub render_update: unsafe extern "C" fn(Handle) -> u64,
    pub render_free: unsafe extern "C" fn(Handle),
    pub render_callback:
        unsafe extern "C" fn(Handle, Option<extern "C" fn(*mut c_void)>, *mut c_void),
}
// SAFETY: libmpv documents the client interface as thread-safe. Only the control
// worker calls client/property/event functions after initialization. Render calls
// are confined to Slint's current-context callback. Arc retains library+client
// until the render context is freed and the control worker has exited.
unsafe impl Send for Api {}
unsafe impl Sync for Api {}

// Windows dependencies resolve only in trusted system/application directories;
// LOAD_LIBRARY_SEARCH_DEFAULT_DIRS excludes cwd and works with the local runtime.
unsafe fn load_library(path: &std::path::Path) -> Result<Library, String> {
    #[cfg(windows)]
    {
        use libloading::os::windows::{
            LOAD_LIBRARY_SEARCH_DEFAULT_DIRS, Library as WindowsLibrary,
        };
        let path = path.canonicalize().map_err(|error| error.to_string())?;
        // SAFETY: caller retains the library; the absolute path is selected by
        // executable-relative discovery or an explicit FOXIDE_LIBMPV override.
        unsafe {
            WindowsLibrary::load_with_flags(path, LOAD_LIBRARY_SEARCH_DEFAULT_DIRS)
                .map(Into::into)
                .map_err(|error| error.to_string())
        }
    }
    #[cfg(not(windows))]
    // SAFETY: native package-manager library lookup; ownership is retained by Api.
    unsafe {
        Library::new(path).map_err(|error| error.to_string())
    }
}

impl Api {
    fn load(
        helper: Option<&player_services::VideoSiteTool>,
        thumbnail: bool,
    ) -> Result<Arc<Self>, String> {
        Self::load_with_options(helper, thumbnail, &[])
    }

    fn load_with_options(
        helper: Option<&player_services::VideoSiteTool>,
        thumbnail: bool,
        extra_options: &[(&CStr, &CStr)],
    ) -> Result<Arc<Self>, String> {
        let candidates: Vec<PathBuf> = if let Some(path) = std::env::var_os("FOXIDE_LIBMPV") {
            vec![path.into()]
        } else if cfg!(windows) {
            crate::dependency_paths::windows_libraries(
                &std::env::current_exe().map_err(|e| e.to_string())?,
            )
        } else if cfg!(target_os = "macos") {
            vec![
                "/opt/homebrew/lib/libmpv.2.dylib".into(),
                "/usr/local/lib/libmpv.2.dylib".into(),
                "libmpv.2.dylib".into(),
            ]
        } else {
            vec!["libmpv.so.2".into()]
        };
        let mut errors = Vec::new();
        #[cfg(windows)]
        let mut runtime_file_found = false;
        for path in candidates {
            #[cfg(windows)]
            {
                runtime_file_found |= path.is_file();
            }
            // SAFETY: library stays owned by Api; every symbol has the documented
            // C signature from the pinned mpv client/render headers. No unload
            // occurs until the last client/render Arc has been released.
            let library = match unsafe { load_library(&path) } {
                Ok(value) => value,
                Err(error) => {
                    errors.push(error.to_string());
                    continue;
                }
            };
            let loaded = unsafe { Self::from_library(library, helper, thumbnail, extra_options) };
            match loaded {
                Ok(api) => return Ok(Arc::new(api)),
                Err(error) => errors.push(error),
            }
        }
        #[cfg(windows)]
        return Err(windows_runtime_error(
            runtime_file_found,
            &errors.join("; "),
        ));
        #[cfg(not(windows))]
        Err(format!(
            "Playback engine unavailable: {}",
            errors.join("; ")
        ))
    }

    unsafe fn from_library(
        library: Library,
        helper: Option<&player_services::VideoSiteTool>,
        thumbnail: bool,
        extra_options: &[(&CStr, &CStr)],
    ) -> Result<Self, String> {
        // SAFETY: caller supplies an owned libmpv library. All functions are loaded
        // before creating a client so missing symbols cannot leak the client.
        unsafe {
            macro_rules! symbol {
                ($name:literal, $ty:ty) => {
                    *library
                        .get::<$ty>(concat!($name, "\0").as_bytes())
                        .map_err(|e| e.to_string())?
                };
            }
            let version = symbol!("mpv_client_api_version", unsafe extern "C" fn() -> c_ulong);
            if version() >> 16 != 2 {
                return Err("Unsupported libmpv client ABI; expected major version 2".into());
            }
            let create = symbol!("mpv_create", unsafe extern "C" fn() -> Handle);
            let initialize = symbol!("mpv_initialize", unsafe extern "C" fn(Handle) -> c_int);
            let option = symbol!(
                "mpv_set_option_string",
                unsafe extern "C" fn(Handle, *const c_char, *const c_char) -> c_int
            );
            let node_option = symbol!(
                "mpv_set_option",
                unsafe extern "C" fn(Handle, *const c_char, c_int, *mut c_void) -> c_int
            );
            let observe = symbol!(
                "mpv_observe_property",
                unsafe extern "C" fn(Handle, u64, *const c_char, c_int) -> c_int
            );
            let mut api = Self {
                handle: std::ptr::null_mut(),
                command: symbol!(
                    "mpv_command",
                    unsafe extern "C" fn(Handle, *const *const c_char) -> c_int
                ),
                property: symbol!(
                    "mpv_get_property_string",
                    unsafe extern "C" fn(Handle, *const c_char) -> *mut c_char
                ),
                set_property: symbol!(
                    "mpv_set_property",
                    unsafe extern "C" fn(Handle, *const c_char, c_int, *mut c_void) -> c_int
                ),
                free: symbol!("mpv_free", unsafe extern "C" fn(*mut c_void)),
                wait_event: symbol!(
                    "mpv_wait_event",
                    unsafe extern "C" fn(Handle, f64) -> *const Event
                ),
                destroy: symbol!("mpv_terminate_destroy", unsafe extern "C" fn(Handle)),
                destroy_uninitialized: symbol!("mpv_destroy", unsafe extern "C" fn(Handle)),
                initialized: false,
                render_create: symbol!(
                    "mpv_render_context_create",
                    unsafe extern "C" fn(*mut Handle, Handle, *mut Param) -> c_int
                ),
                render: symbol!(
                    "mpv_render_context_render",
                    unsafe extern "C" fn(Handle, *mut Param) -> c_int
                ),
                render_update: symbol!(
                    "mpv_render_context_update",
                    unsafe extern "C" fn(Handle) -> u64
                ),
                render_free: symbol!("mpv_render_context_free", unsafe extern "C" fn(Handle)),
                render_callback: symbol!(
                    "mpv_render_context_set_update_callback",
                    unsafe extern "C" fn(Handle, Option<extern "C" fn(*mut c_void)>, *mut c_void)
                ),
                _library: library,
            };
            api.handle = create();
            if api.handle.is_null() {
                return Err("libmpv could not allocate a player".into());
            }
            for (name, value) in [
                (c"config", c"no"),
                (c"load-scripts", c"no"),
                (c"vo", c"libmpv"),
                (c"hwdec", c"auto-safe"),
                (c"keep-open", c"yes"),
                (c"terminal", c"no"),
                (c"input-default-bindings", c"no"),
                (c"audio", if thumbnail { c"no" } else { c"auto" }),
                (c"gapless-audio", if thumbnail { c"no" } else { c"yes" }),
            ] {
                if option(api.handle, name.as_ptr(), value.as_ptr()) < 0 {
                    return Err("libmpv rejected a required playback option".into());
                }
            }
            for (name, value) in extra_options {
                if option(api.handle, name.as_ptr(), value.as_ptr()) < 0 {
                    return Err(format!(
                        "libmpv rejected the requested output option {}",
                        name.to_string_lossy()
                    ));
                }
            }
            let enabled = if helper.is_some() { c"yes" } else { c"no" };
            let helper_status = option(api.handle, c"ytdl".as_ptr(), enabled.as_ptr());
            // A Lua-disabled build has no ytdl option (-5). With no selected
            // helper, that already guarantees the hook cannot run.
            if helper_status < 0 && (helper.is_some() || helper_status != -5) {
                return Err(
                    "This libmpv build cannot enable video sites; a Lua-enabled engine is required"
                        .into(),
                );
            }
            if let Some(helper) = helper {
                let value = CString::new(
                    helper
                        .path()
                        .to_str()
                        .ok_or("Helper path must be Unicode")?,
                )
                .map_err(|_| "Invalid helper path")?;
                let mut entries = [Node {
                    value: NodeValue {
                        string: value.as_ptr().cast_mut(),
                    },
                    format: 1,
                }];
                let mut keys = [c"ytdl_hook-ytdl_path".as_ptr().cast_mut()];
                let mut list = NodeList {
                    num: 1,
                    values: entries.as_mut_ptr(),
                    keys: keys.as_mut_ptr(),
                };
                let mut node = Node {
                    value: NodeValue { list: &mut list },
                    format: 8,
                };
                // SAFETY: MPV_FORMAT_NODE (6) points at a NODE_MAP (8) containing
                // one STRING (1). mpv copies the value synchronously. All C strings,
                // entries and arrays live through the call; no borrowed data escapes.
                if node_option(
                    api.handle,
                    c"script-opts".as_ptr(),
                    6,
                    (&mut node as *mut Node).cast(),
                ) < 0
                {
                    return Err("libmpv rejected the explicit video-site helper path".into());
                }
            }
            if initialize(api.handle) < 0 {
                return Err("libmpv initialization failed".into());
            }
            api.initialized = true;
            if observe(api.handle, 1, c"track-list".as_ptr(), 0) < 0 {
                return Err("Cannot observe playback track metadata".into());
            }
            {
                let logs = api
                    ._library
                    .get::<unsafe extern "C" fn(Handle, *const c_char) -> c_int>(
                        b"mpv_request_log_messages\0",
                    )
                    .map_err(|error| error.to_string())?;
                logs(
                    api.handle,
                    if cfg!(test) {
                        c"debug".as_ptr()
                    } else {
                        c"warn".as_ptr()
                    },
                );
            }
            Ok(api)
        }
    }

    fn set_shaders(&self, shaders: &[player_services::ShaderFile]) -> Result<(), String> {
        let strings = shaders
            .iter()
            .map(|shader| CString::new(shader.path().to_string_lossy().as_bytes()))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| "Invalid shader path")?;
        let mut values: Vec<Node> = strings
            .iter()
            .map(|value| Node {
                value: NodeValue {
                    string: value.as_ptr().cast_mut(),
                },
                format: 1,
            })
            .collect();
        let mut list = NodeList {
            num: values.len() as c_int,
            values: values.as_mut_ptr(),
            keys: std::ptr::null_mut(),
        };
        let mut node = Node {
            value: NodeValue { list: &mut list },
            format: 7,
        };
        // SAFETY: the STRING nodes in this NODE_ARRAY and all C strings live through
        // this synchronous copy. An empty list clears all shaders. Native path-list
        // parsing is bypassed, preserving commas, semicolons and drive letters.
        let code = unsafe {
            (self.set_property)(
                self.handle,
                c"glsl-shaders".as_ptr(),
                6,
                (&mut node as *mut Node).cast(),
            )
        };
        if code < 0 {
            Err(format!("Shader configuration rejected ({code})"))
        } else {
            Ok(())
        }
    }
    fn get(&self, name: &CStr) -> String {
        // SAFETY: only the control worker retrieves properties. Returned strings
        // are copied before mpv_free, using the same library's allocator.
        unsafe {
            let value = (self.property)(self.handle, name.as_ptr());
            if value.is_null() {
                return String::new();
            }
            let text = CStr::from_ptr(value).to_string_lossy().into_owned();
            (self.free)(value.cast());
            text
        }
    }
    fn track_property(&self, index: u32, suffix: &str) -> String {
        // Both components are internal ASCII values; there can be no interior NUL.
        let name = CString::new(format!("track-list/{index}/{suffix}")).unwrap();
        self.get(&name)
    }
    fn number(&self, name: &CStr) -> f64 {
        self.get(name)
            .parse::<f64>()
            .ok()
            .filter(|v| v.is_finite())
            .unwrap_or(0.0)
    }
    fn send(&self, args: &[String]) -> Result<(), String> {
        let strings = args
            .iter()
            .map(|s| CString::new(s.as_str()))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| "Playback input contains a NUL byte")?;
        let mut pointers = Vec::with_capacity(strings.len() + 1);
        pointers.extend(strings.iter().map(|s| s.as_ptr()));
        pointers.push(std::ptr::null());
        // SAFETY: strings and null-terminated pointer array live through the call.
        let code = unsafe { (self.command)(self.handle, pointers.as_ptr()) };
        if code < 0 {
            Err(format!("Playback command failed ({code})"))
        } else {
            Ok(())
        }
    }
}

#[cfg(windows)]
fn windows_runtime_error(runtime_file_found: bool, details: &str) -> String {
    if runtime_file_found {
        format!(
            "Playback runtime was found, but Windows could not initialize it. A required runtime DLL may be missing or blocked. Re-run Start Menu > Foxide > Install playback runtime, then restart Foxide. See Help > Open diagnostic log. Details: {details}"
        )
    } else {
        format!(
            "Playback runtime is not installed. Use Start Menu > Foxide > Install playback runtime, then restart Foxide. See Help > Open diagnostic log for details. Details: {details}"
        )
    }
}
impl Drop for Api {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            // SAFETY: the final Arc is released only after renderer teardown and
            // control-worker exit. No callback/command can access the client now.
            unsafe {
                if self.initialized {
                    (self.destroy)(self.handle);
                } else {
                    (self.destroy_uninitialized)(self.handle);
                }
            }
        }
    }
}

fn renderer_warning(prefix: &str, level: c_int, text: &str) -> Option<String> {
    if level > 30 || !(prefix.starts_with("vo/") || prefix.starts_with("libmpv_render")) {
        return None;
    }
    Some(format!(
        "Renderer warning: {}",
        text.trim().chars().take(2048).collect::<String>()
    ))
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Track {
    pub id: u32,
    pub label: String,
    pub kind: String,
    pub selected: bool,
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Snapshot {
    pub path: String,
    /// Original user selection, retained across helper redirects without per-tick allocation.
    pub requested_path: Arc<str>,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub loaded: bool,
    pub paused: bool,
    pub position: f64,
    pub duration: f64,
    pub playlist_position: usize,
    pub speed: f64,
    pub volume: f64,
    pub muted: bool,
    pub hwdec: String,
    pub has_artwork: bool,
    pub shader_count: usize,
    pub tracks: Arc<[Track]>,
    /// A rejected control; playback remains available. Cleared by the next successful command.
    pub command_error: String,
    pub error: String,
}
enum Request {
    Command(Vec<String>),
    Shaders(Vec<player_services::ShaderFile>),
}

type SnapshotNotifier = Arc<dyn Fn() + Send + Sync>;

pub struct Engine {
    pub(crate) api: Arc<Api>,
    pub(crate) ready: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
    sender: SyncSender<Request>,
    snapshot: Arc<Mutex<Snapshot>>,
    notifier: Arc<Mutex<Option<SnapshotNotifier>>>,
    #[cfg(all(test, feature = "gpu-integration"))]
    pub(crate) diagnostics: Arc<Mutex<Vec<String>>>,
}
impl Engine {
    pub fn new() -> Result<Self, String> {
        Self::with_video_site_helper(None)
    }

    pub fn with_video_site_helper(helper: Option<&std::path::Path>) -> Result<Self, String> {
        Self::create(helper, false)
    }

    pub(crate) fn new_thumbnail() -> Result<Self, String> {
        Self::create(None, true)
    }

    #[cfg(test)]
    fn new_audio_capture(path: &std::path::Path) -> Result<Self, String> {
        let path = CString::new(path.to_string_lossy().as_bytes())
            .map_err(|_| "PCM capture path contains a null byte")?;
        let options = [
            (c"vo", c"null"),
            (c"ao", c"pcm"),
            (c"ao-pcm-file", path.as_c_str()),
            (c"ao-pcm-waveheader", c"yes"),
            (c"audio-format", c"s16"),
            (c"audio-samplerate", c"44100"),
            (c"audio-channels", c"mono"),
        ];
        Self::create_with_options(None, false, &options)
    }

    #[cfg(test)]
    fn new_audio_device_capture(device: &str) -> Result<Self, String> {
        let device = CString::new(device).map_err(|_| "Audio device contains a null byte")?;
        let options = [
            (c"vo", c"null"),
            (c"ao", c"pulse"),
            (c"audio-device", device.as_c_str()),
            (c"audio-format", c"s16"),
            (c"audio-samplerate", c"44100"),
            (c"audio-channels", c"mono"),
        ];
        Self::create_with_options(None, false, &options)
    }

    fn create(helper: Option<&std::path::Path>, thumbnail: bool) -> Result<Self, String> {
        Self::create_with_options(helper, thumbnail, &[])
    }

    fn create_with_options(
        helper: Option<&std::path::Path>,
        thumbnail: bool,
        extra_options: &[(&CStr, &CStr)],
    ) -> Result<Self, String> {
        let helper = helper
            .map(player_services::VideoSiteTool::new)
            .transpose()?;
        let api = Api::load_with_options(helper.as_ref(), thumbnail, extra_options)?;
        let ready = Arc::new(AtomicBool::new(false));
        let stop = Arc::new(AtomicBool::new(false));
        let snapshot = Arc::new(Mutex::new(Snapshot::default()));
        let notifier = Arc::new(Mutex::new(None::<SnapshotNotifier>));
        let notify_output = notifier.clone();
        #[cfg(all(test, feature = "gpu-integration"))]
        let diagnostics = Arc::new(Mutex::new(Vec::<String>::new()));
        #[cfg(all(test, feature = "gpu-integration"))]
        let diagnostic_output = diagnostics.clone();
        let (sender, receiver) = mpsc::sync_channel::<Request>(32);
        let (client, active, stopped, output) =
            (api.clone(), ready.clone(), stop.clone(), snapshot.clone());
        thread::Builder::new()
            .name(if thumbnail {
                "foxide-thumbnail-engine".into()
            } else {
                "foxide-engine".into()
            })
            .spawn(move || {
                let mut last_path = String::new();
                let mut requested_path: Arc<str> = Arc::from("");
                let mut file_loaded = false;
                let mut tracks: Arc<[Track]> = Arc::from([]);
                let mut has_artwork = false;
                let mut error = String::new();
                let mut command_error = String::new();
                let mut shader_count = 0;
                while !stopped.load(Ordering::Acquire) {
                    if !active.load(Ordering::Acquire) {
                        thread::sleep(Duration::from_millis(20));
                        continue;
                    }
                    match receiver.recv_timeout(Duration::from_millis(200)) {
                        Ok(Request::Shaders(shaders)) => {
                            command_error.clear();
                            match client.set_shaders(&shaders) {
                                Ok(()) => shader_count = shaders.len(),
                                Err(e) => command_error = e,
                            }
                        }
                        Ok(Request::Command(args)) => {
                            let loading = args.first().is_some_and(|s| s == "loadfile");
                            if loading {
                                file_loaded = false;
                                requested_path = args
                                    .get(1)
                                    .map_or_else(|| Arc::from(""), |path| Arc::from(path.as_str()));
                                error.clear();
                                last_path.clear();
                            }
                            command_error.clear();
                            if let Err(e) = client.send(&args) {
                                if loading {
                                    error = e;
                                } else {
                                    command_error = e;
                                }
                            }
                            tracks = Arc::from([]);
                        }
                        Err(mpsc::RecvTimeoutError::Disconnected) => break,
                        Err(mpsc::RecvTimeoutError::Timeout) => {}
                    }
                    // SAFETY: event data is inspected before the next wait_event call
                    // invalidates it. Only this worker consumes the client's events.
                    unsafe {
                        loop {
                            let event = (client.wait_event)(client.handle, 0.0);
                            if event.is_null() || (*event).kind == 0 {
                                break;
                            }
                            #[cfg(all(test, feature = "gpu-integration"))]
                            if (*event).kind == 2 && !(*event).data.is_null() {
                                let message = &*((*event).data.cast::<LogMessage>());
                                if !message.prefix.is_null() && !message.text.is_null() {
                                    let text = format!(
                                        "[{}] {}",
                                        CStr::from_ptr(message.prefix).to_string_lossy(),
                                        CStr::from_ptr(message.text).to_string_lossy()
                                    );
                                    if let Ok(mut logs) = diagnostic_output.lock()
                                        && logs.len() < 10000
                                    {
                                        logs.push(text);
                                    }
                                }
                            }
                            if (*event).kind == 2 && !(*event).data.is_null() {
                                let message = &*((*event).data.cast::<LogMessage>());
                                if !message.prefix.is_null()
                                    && !message.text.is_null()
                                    && let Some(warning) = renderer_warning(
                                        &CStr::from_ptr(message.prefix).to_string_lossy(),
                                        message.log_level,
                                        &CStr::from_ptr(message.text).to_string_lossy(),
                                    )
                                {
                                    command_error = warning;
                                }
                            }
                            if matches!((*event).kind, 6 | 7) {
                                file_loaded = false;
                            }
                            if (*event).kind == 8 {
                                file_loaded = true;
                            }
                            if (*event).kind == 8 || ((*event).kind == 22 && (*event).userdata == 1)
                            {
                                tracks = Arc::from([]);
                            }
                            if (*event).kind == 7 && !(*event).data.is_null() {
                                let end = &*((*event).data.cast::<End>());
                                if end.reason == 4 {
                                    error = format!(
                                        "Unable to play this media (engine error {})",
                                        end.error
                                    );
                                }
                            }
                        }
                    }
                    let path = client.get(c"path");
                    if path != last_path || tracks.is_empty() {
                        let mut next_tracks = Vec::new();
                        has_artwork = false;
                        let count = client.number(c"track-list/count").clamp(0.0, 256.0) as u32;
                        for i in 0..count {
                            let kind = client.track_property(i, "type");
                            if !matches!(kind.as_str(), "audio" | "sub" | "video") {
                                continue;
                            }
                            if kind == "video" && client.track_property(i, "albumart") == "yes" {
                                has_artwork |= client.track_property(i, "selected") == "yes";
                                continue;
                            }
                            let id = client
                                .track_property(i, "id")
                                .parse::<f64>()
                                .ok()
                                .filter(|v| v.is_finite())
                                .unwrap_or(0.0) as u32;
                            let title = client.track_property(i, "title");
                            let label = if title.is_empty() {
                                client.track_property(i, "lang")
                            } else {
                                title
                            };
                            next_tracks.push(Track {
                                id,
                                label: format!("{id} · {label}"),
                                kind,
                                selected: client.track_property(i, "selected") == "yes",
                            });
                        }
                        tracks = next_tracks.into();
                        last_path.clone_from(&path);
                    }
                    let next = Snapshot {
                        loaded: file_loaded
                            && !path.is_empty()
                            && client.get(c"idle-active") == "no"
                            && error.is_empty(),
                        path,
                        requested_path: requested_path.clone(),
                        title: client.get(c"media-title"),
                        artist: client.get(c"metadata/by-key/artist"),
                        album: client.get(c"metadata/by-key/album"),
                        paused: client.get(c"pause") == "yes",
                        position: client.number(c"time-pos"),
                        duration: client.number(c"duration"),
                        playlist_position: client.number(c"playlist-pos").max(0.0) as usize,
                        speed: client.number(c"speed"),
                        volume: client.number(c"volume"),
                        muted: client.get(c"mute") == "yes",
                        hwdec: client.get(c"hwdec-current"),
                        has_artwork,
                        shader_count,
                        tracks: tracks.clone(),
                        command_error: command_error.clone(),
                        error: error.clone(),
                    };
                    let changed = if let Ok(mut current) = output.lock() {
                        if *current == next {
                            false
                        } else {
                            *current = next;
                            true
                        }
                    } else {
                        false
                    };
                    if changed {
                        let callback = notify_output.lock().ok().and_then(|slot| slot.clone());
                        if let Some(callback) = callback {
                            callback();
                        }
                    }
                }
            })
            .map_err(|e| e.to_string())?;
        Ok(Self {
            api,
            ready,
            stop,
            sender,
            snapshot,
            notifier,
            #[cfg(all(test, feature = "gpu-integration"))]
            diagnostics,
        })
    }
    pub fn command(&self, args: Vec<String>) -> Result<(), String> {
        self.sender
            .try_send(Request::Command(args))
            .map_err(|e| format!("Player is busy: {e}"))
    }
    /// Replace the session shader stack; only explicitly validated files are accepted.
    pub fn set_shaders(&self, shaders: Vec<player_services::ShaderFile>) -> Result<(), String> {
        if shaders.len() > 8 {
            return Err("Choose at most eight shaders".into());
        }
        self.sender
            .try_send(Request::Shaders(shaders))
            .map_err(|e| format!("Player is busy: {e}"))
    }
    pub fn set_display_profile(
        &self,
        profile: Option<player_services::DisplayProfile>,
    ) -> Result<(), String> {
        let path = profile
            .as_ref()
            .map(|profile| profile.path().to_string_lossy().into_owned())
            .unwrap_or_default();
        self.command(vec!["set".into(), "icc-profile".into(), path])
    }
    /// Install one status observer. It runs outside snapshot locks and is called
    /// immediately once so a late subscriber can read the current paused state.
    pub fn set_snapshot_notifier(&self, notifier: Arc<dyn Fn() + Send + Sync>) {
        if let Ok(mut slot) = self.notifier.lock() {
            *slot = Some(notifier.clone());
        }
        notifier();
    }
    pub fn snapshot(&self) -> Option<Snapshot> {
        self.snapshot.try_lock().ok().map(|s| s.clone())
    }
}
impl Drop for Engine {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
    }
}

#[cfg(test)]
mod snapshot_allocation_tests {
    use super::*;
    #[test]
    fn renderer_warnings_include_render_api_prefixes_and_are_bounded() {
        assert_eq!(
            renderer_warning("libmpv_render/opengl", 20, "compile failed\n"),
            Some("Renderer warning: compile failed".into())
        );
        assert!(renderer_warning("vo/gpu", 30, "warning").is_some());
        assert!(renderer_warning("demux", 20, "unrelated").is_none());
        assert!(renderer_warning("libmpv_render", 40, "info").is_none());
        let large = "🦊".repeat(3000);
        let result = renderer_warning("libmpv_render", 20, &large).unwrap();
        assert!(result.ends_with("🦊") && result.len() < 9000);
    }
    #[test]
    fn cloning_snapshot_shares_immutable_track_metadata() {
        let snapshot = Snapshot {
            tracks: vec![Track {
                id: 1,
                label: "English".into(),
                kind: "audio".into(),
                selected: true,
            }]
            .into(),
            ..Default::default()
        };
        let copy = snapshot.clone();
        assert_eq!(copy, snapshot);
        assert_eq!(
            copy.tracks.as_ptr(),
            snapshot.tracks.as_ptr(),
            "A position update must not allocate and clone immutable track strings"
        );
    }

    #[test]
    #[ignore = "Native libmpv metadata acceptance; requires generated tagged fixture and libmpv"]
    #[cfg(feature = "system-media-integration")]
    fn native_snapshot_exposes_embedded_track_metadata() {
        use std::time::Instant;

        let fixture = std::env::var("FOXIDE_SYSTEM_MEDIA_FIXTURE")
            .expect("workflow must provide the tagged metadata fixture");
        let engine = Engine::new().expect("libmpv must be available");
        for (name, value) in [("vo", "null"), ("ao", "null"), ("pause", "no")] {
            engine
                .command(vec!["set".into(), name.into(), value.into()])
                .unwrap();
        }
        engine.ready.store(true, Ordering::Release);
        engine
            .command(vec!["loadfile".into(), fixture, "replace".into()])
            .unwrap();

        let deadline = Instant::now() + Duration::from_secs(15);
        while Instant::now() < deadline {
            let snapshot = engine.snapshot().unwrap_or_default();
            if !snapshot.error.is_empty() {
                panic!("libmpv failed to load metadata fixture: {}", snapshot.error);
            }
            if snapshot.loaded && !snapshot.artist.is_empty() && !snapshot.album.is_empty() {
                assert_eq!(snapshot.title, "Foxide system media fixture");
                assert_eq!(snapshot.artist, "Foxide CI");
                assert_eq!(snapshot.album, "System media controls");
                engine.command(vec!["stop".into()]).unwrap();
                return;
            }
            thread::sleep(Duration::from_millis(50));
        }
        panic!("libmpv snapshot did not expose embedded title, artist, and album metadata");
    }

    #[test]
    #[ignore = "Native playlist acceptance; requires generated short audio fixtures and libmpv"]
    #[cfg(feature = "system-media-integration")]
    fn native_playlist_advances_to_the_queued_audio_track() {
        use std::time::Instant;

        let first = std::env::var("FOXIDE_GAPLESS_FIRST")
            .expect("workflow must provide the first short audio fixture");
        let second = std::env::var("FOXIDE_GAPLESS_SECOND")
            .expect("workflow must provide the second short audio fixture");
        let engine = Engine::new().expect("libmpv must be available");
        for (name, value) in [("vo", "null"), ("ao", "null"), ("pause", "no")] {
            engine
                .command(vec!["set".into(), name.into(), value.into()])
                .unwrap();
        }
        engine.ready.store(true, Ordering::Release);
        engine
            .command(vec!["loadfile".into(), first, "replace".into()])
            .unwrap();
        engine
            .command(vec!["loadfile".into(), second, "append".into()])
            .unwrap();

        let deadline = Instant::now() + Duration::from_secs(15);
        while Instant::now() < deadline {
            let snapshot = engine.snapshot().unwrap_or_default();
            if !snapshot.error.is_empty() {
                panic!("libmpv failed to play queued audio: {}", snapshot.error);
            }
            if snapshot.loaded && snapshot.playlist_position == 1 {
                engine.command(vec!["stop".into()]).unwrap();
                return;
            }
            thread::sleep(Duration::from_millis(20));
        }
        panic!("libmpv did not automatically advance to the queued audio track");
    }

    #[test]
    #[ignore = "Native gapless audio acceptance; requires generated WAV fixtures and libmpv"]
    #[cfg(feature = "system-media-integration")]
    fn native_playlist_pcm_capture_has_no_silence_at_track_boundary() {
        use std::time::Instant;

        let first = std::env::var("FOXIDE_GAPLESS_FIRST")
            .expect("workflow must provide the first short audio fixture");
        let second = std::env::var("FOXIDE_GAPLESS_SECOND")
            .expect("workflow must provide the second short audio fixture");
        let capture = std::env::var("FOXIDE_GAPLESS_CAPTURE")
            .expect("workflow must provide a PCM capture path");
        let engine = Engine::new_audio_capture(std::path::Path::new(&capture))
            .expect("libmpv must initialize the PCM capture output");
        engine.ready.store(true, Ordering::Release);
        engine
            .command(vec!["loadfile".into(), first, "replace".into()])
            .unwrap();
        engine
            .command(vec!["loadfile".into(), second, "append".into()])
            .unwrap();

        let deadline = Instant::now() + Duration::from_secs(15);
        while Instant::now() < deadline {
            let snapshot = engine.snapshot().unwrap_or_default();
            assert!(
                snapshot.error.is_empty(),
                "libmpv failed to capture queued audio: {}",
                snapshot.error
            );
            if snapshot.loaded
                && snapshot.playlist_position == 1
                && snapshot.duration > 0.0
                && snapshot.position >= snapshot.duration - 0.02
            {
                engine.command(vec!["stop".into()]).unwrap();
                break;
            }
            thread::sleep(Duration::from_millis(20));
        }
        assert!(
            Instant::now() < deadline,
            "libmpv did not finish the second queued audio track"
        );
        drop(engine);
        thread::sleep(Duration::from_millis(250));

        let bytes = std::fs::read(capture).expect("libmpv should write its PCM capture");
        assert_pcm_wav_has_gapless_boundary(&bytes, "libmpv PCM writer");
    }

    #[test]
    #[ignore = "Native PulseAudio loopback acceptance; requires a configured virtual sink and libmpv"]
    #[cfg(feature = "system-media-integration")]
    fn native_playlist_audio_device_loopback_has_no_silence_at_track_boundary() {
        use std::time::Instant;
        use std::{
            io::Read,
            process::{Command, Stdio},
        };

        let first = std::env::var("FOXIDE_GAPLESS_FIRST")
            .expect("workflow must provide the first short audio fixture");
        let second = std::env::var("FOXIDE_GAPLESS_SECOND")
            .expect("workflow must provide the second short audio fixture");
        let sink = std::env::var("FOXIDE_GAPLESS_PULSE_SINK")
            .expect("workflow must provide the virtual PulseAudio sink name");
        let monitor = format!("{sink}.monitor");
        let mut recorder = Command::new("parecord")
            .args(["--raw", "--format=s16le", "--rate=44100", "--channels=1"])
            .arg(format!("--device={monitor}"))
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("PulseAudio recorder must be installed in the native acceptance job");
        let mut output = recorder
            .stdout
            .take()
            .expect("PulseAudio recorder must expose its PCM stream");
        let capture_reader = thread::spawn(move || {
            let mut bytes = Vec::new();
            output
                .read_to_end(&mut bytes)
                .expect("read PulseAudio monitor PCM");
            bytes
        });
        thread::sleep(Duration::from_millis(250));
        assert!(
            recorder.try_wait().unwrap().is_none(),
            "PulseAudio loopback recorder exited before playback"
        );

        let engine = Engine::new_audio_device_capture(&format!("pulse/{sink}"))
            .expect("libmpv must initialize the PulseAudio sink");
        engine.ready.store(true, Ordering::Release);
        engine
            .command(vec!["loadfile".into(), first, "replace".into()])
            .unwrap();
        engine
            .command(vec!["loadfile".into(), second, "append".into()])
            .unwrap();

        let deadline = Instant::now() + Duration::from_secs(15);
        let mut second_track_started = None;
        let mut last_snapshot = Snapshot::default();
        while Instant::now() < deadline {
            let snapshot = engine.snapshot().unwrap_or_default();
            last_snapshot = snapshot.clone();
            assert!(
                snapshot.error.is_empty(),
                "libmpv failed to play through PulseAudio: {}",
                snapshot.error
            );
            if snapshot.playlist_position == 1 {
                let started = second_track_started.get_or_insert_with(Instant::now);
                if started.elapsed() >= Duration::from_millis(1_200) {
                    break;
                }
            }
            thread::sleep(Duration::from_millis(20));
        }
        engine.command(vec!["stop".into()]).unwrap();
        assert!(
            second_track_started.is_some(),
            "libmpv did not advance to the second queued track: loaded={}, path={:?}, playlist_position={}, position={:.3}, duration={:.3}, paused={}, error={:?}, command_error={:?}",
            last_snapshot.loaded,
            last_snapshot.path,
            last_snapshot.playlist_position,
            last_snapshot.position,
            last_snapshot.duration,
            last_snapshot.paused,
            last_snapshot.error,
            last_snapshot.command_error
        );
        drop(engine);
        recorder.kill().expect("stop PulseAudio loopback recorder");
        let _ = recorder.wait().expect("wait for PulseAudio recorder");

        let pcm = capture_reader
            .join()
            .expect("PulseAudio monitor reader thread must finish");
        assert_eq!(pcm.len() % 2, 0, "PCM16 capture must contain whole samples");
        let samples = pcm
            .chunks_exact(2)
            .map(|sample| i16::from_le_bytes([sample[0], sample[1]]))
            .collect::<Vec<_>>();
        let active_span_ms = assert_pcm_wav_has_gapless_boundary(
            &test_pcm_wav(&samples),
            "PulseAudio device loopback",
        );
        assert!(
            active_span_ms >= 3_000,
            "PulseAudio loopback captured only {active_span_ms} ms of active audio; expected both 2-second tracks and a sustained second track"
        );
    }

    #[test]
    #[cfg(feature = "system-media-integration")]
    fn gapless_loopback_analyzer_ignores_capture_edges_and_rejects_internal_silence() {
        let first_track = test_tone(440.0, 4_410);
        let second_track = test_tone(880.0, 4_410);
        let mut contiguous = vec![0_i16; 441];
        contiguous.extend_from_slice(&first_track);
        contiguous.extend(std::iter::repeat_n(0_i16, 220));
        contiguous.extend_from_slice(&second_track);
        contiguous.extend(std::iter::repeat_n(0_i16, 441));
        let active_span_ms =
            assert_pcm_wav_has_gapless_boundary(&test_pcm_wav(&contiguous), "synthetic capture");
        assert!(
            active_span_ms >= 190,
            "synthetic capture should retain the active audio span, got {active_span_ms} ms"
        );

        let mut with_gap = vec![0_i16; 441];
        with_gap.extend_from_slice(&first_track);
        with_gap.extend(std::iter::repeat_n(0_i16, 1_323));
        with_gap.extend_from_slice(&second_track);
        with_gap.extend(std::iter::repeat_n(0_i16, 441));
        assert!(
            std::panic::catch_unwind(|| {
                assert_pcm_wav_has_gapless_boundary(&test_pcm_wav(&with_gap), "synthetic capture");
            })
            .is_err()
        );
    }

    #[cfg(feature = "system-media-integration")]
    fn test_tone(frequency: f64, sample_count: usize) -> Vec<i16> {
        (0..sample_count)
            .map(|sample| {
                (8_000.0 * (std::f64::consts::TAU * frequency * sample as f64 / 44_100.0).sin())
                    as i16
            })
            .collect()
    }

    #[cfg(feature = "system-media-integration")]
    fn test_pcm_wav(samples: &[i16]) -> Vec<u8> {
        let data_length = (samples.len() * 2) as u32;
        let mut wav = Vec::with_capacity(44 + data_length as usize);
        wav.extend_from_slice(b"RIFF");
        wav.extend_from_slice(&(36 + data_length).to_le_bytes());
        wav.extend_from_slice(b"WAVEfmt ");
        wav.extend_from_slice(&16_u32.to_le_bytes());
        wav.extend_from_slice(&1_u16.to_le_bytes());
        wav.extend_from_slice(&1_u16.to_le_bytes());
        wav.extend_from_slice(&44_100_u32.to_le_bytes());
        wav.extend_from_slice(&88_200_u32.to_le_bytes());
        wav.extend_from_slice(&2_u16.to_le_bytes());
        wav.extend_from_slice(&16_u16.to_le_bytes());
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&data_length.to_le_bytes());
        for sample in samples {
            wav.extend_from_slice(&sample.to_le_bytes());
        }
        wav
    }

    fn assert_pcm_wav_has_gapless_boundary(bytes: &[u8], capture_kind: &str) -> u64 {
        assert!(
            bytes.starts_with(b"RIFF"),
            "{capture_kind} must provide a WAV file"
        );
        let mut channels = 0_u16;
        let mut sample_rate = 0_u32;
        let mut bits_per_sample = 0_u16;
        let mut data = None;
        let mut offset = 12;
        while offset + 8 <= bytes.len() {
            let tag = &bytes[offset..offset + 4];
            let length =
                u32::from_le_bytes(bytes[offset + 4..offset + 8].try_into().unwrap()) as usize;
            let start = offset + 8;
            let end = start.saturating_add(length);
            assert!(end <= bytes.len(), "WAV chunk exceeds {capture_kind}");
            if tag == b"fmt " {
                assert!(length >= 16, "WAV format chunk is incomplete");
                let format = u16::from_le_bytes(bytes[start..start + 2].try_into().unwrap());
                if format == 0xfffe {
                    assert!(length >= 40, "WAV extensible format chunk is incomplete");
                    let subtype =
                        u32::from_le_bytes(bytes[start + 24..start + 28].try_into().unwrap());
                    assert_eq!(subtype, 1, "WAV extensible subtype should be PCM");
                } else {
                    assert_eq!(format, 1, "{capture_kind} should use PCM encoding");
                }
                channels = u16::from_le_bytes(bytes[start + 2..start + 4].try_into().unwrap());
                sample_rate = u32::from_le_bytes(bytes[start + 4..start + 8].try_into().unwrap());
                bits_per_sample =
                    u16::from_le_bytes(bytes[start + 14..start + 16].try_into().unwrap());
            } else if tag == b"data" {
                data = Some(&bytes[start..end]);
                break;
            }
            offset = end + (length & 1);
        }
        assert_eq!(channels, 1, "{capture_kind} should be mono");
        assert_eq!(
            sample_rate, 44_100,
            "{capture_kind} sample rate should be stable"
        );
        assert_eq!(bits_per_sample, 16, "{capture_kind} should use PCM16");
        let data = data.expect("WAV capture must contain audio samples");
        assert_eq!(data.len() % 2, 0, "PCM16 must contain whole samples");
        let samples = data
            .chunks_exact(2)
            .map(|sample| i16::from_le_bytes([sample[0], sample[1]]) as i32)
            .collect::<Vec<_>>();
        let window = sample_rate as usize / 200;
        let quiet_windows = samples
            .chunks(window)
            .map(|chunk| {
                let mean_square = chunk
                    .iter()
                    .map(|sample| i64::from(*sample) * i64::from(*sample))
                    .sum::<i64>() as f64
                    / chunk.len() as f64;
                mean_square.sqrt() < 1_500.0
            })
            .collect::<Vec<_>>();
        let first_active = quiet_windows
            .iter()
            .position(|quiet| !quiet)
            .expect("capture must include the first queued track");
        let last_active = quiet_windows
            .iter()
            .rposition(|quiet| !quiet)
            .expect("capture must include the last queued track");
        let frequency_window = sample_rate as usize / 10;
        let first_start = first_active * window;
        let last_end = ((last_active + 1) * window).min(samples.len());
        assert!(
            first_start + frequency_window <= samples.len()
                && last_end >= frequency_window
                && last_end - frequency_window >= first_start,
            "{capture_kind} must include enough samples from both tracks"
        );
        let first_frequency = positive_zero_crossing_frequency(
            &samples[first_start..first_start + frequency_window],
            sample_rate,
        );
        let last_frequency = positive_zero_crossing_frequency(
            &samples[last_end - frequency_window..last_end],
            sample_rate,
        );
        assert!(
            (350.0..=550.0).contains(&first_frequency),
            "{capture_kind}'s first track should be the 440 Hz fixture; measured {first_frequency:.1} Hz"
        );
        assert!(
            (750.0..=1_000.0).contains(&last_frequency),
            "{capture_kind}'s second track should be the 880 Hz fixture; measured {last_frequency:.1} Hz"
        );
        let mut longest_silence_windows = 0;
        let mut current_silence_windows = 0;
        for quiet in &quiet_windows[first_active..=last_active] {
            if *quiet {
                current_silence_windows += 1;
                longest_silence_windows = longest_silence_windows.max(current_silence_windows);
            } else {
                current_silence_windows = 0;
            }
        }
        let active_span_ms = (last_active - first_active) as u64 * 5;
        println!(
            "gapless {capture_kind} sample_rate={sample_rate} sample_count={} first_track_hz={first_frequency:.1} second_track_hz={last_frequency:.1} active_span_ms={active_span_ms} longest_internal_silence_ms={}",
            samples.len(),
            longest_silence_windows * 5
        );
        assert!(
            longest_silence_windows <= 4,
            "{capture_kind} contains a track-boundary silence gap of {} ms",
            longest_silence_windows * 5
        );
        active_span_ms
    }

    fn positive_zero_crossing_frequency(samples: &[i32], sample_rate: u32) -> f64 {
        let crossings = samples
            .windows(2)
            .filter(|pair| pair[0] <= 0 && pair[1] > 0)
            .count();
        crossings as f64 * f64::from(sample_rate) / samples.len() as f64
    }
}

#[cfg(all(test, windows))]
mod missing_runtime_tests {
    use super::{Api, windows_runtime_error};

    #[test]
    fn absent_installed_runtime_explains_how_to_recover() {
        let Err(error) = Api::load(None, false) else {
            // Developer machines may already have the optional runtime installed.
            return;
        };
        assert!(
            error.contains("Install playback runtime"),
            "Missing runtime should give the installed-app recovery step; got: {error}"
        );
    }

    #[test]
    fn distinguishes_missing_runtime_from_a_failed_runtime_load() {
        assert!(windows_runtime_error(false, "not found").contains("is not installed"));
        assert!(windows_runtime_error(true, "dependency missing").contains("could not initialize"));
    }
}

#[cfg(all(test, feature = "video-site-integration"))]
#[path = "native_sites.rs"]
mod video_site_tests;

#[cfg(all(test, feature = "live-service-integration"))]
#[path = "native_live.rs"]
mod live_services;
