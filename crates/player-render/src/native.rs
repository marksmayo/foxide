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

// Windows dependencies may resolve only beside the requested DLL or in trusted
// system/application directories; LOAD_LIBRARY_SEARCH_DEFAULT_DIRS excludes cwd.
unsafe fn load_library(path: &std::path::Path) -> Result<Library, String> {
    #[cfg(windows)]
    {
        use libloading::os::windows::{
            LOAD_LIBRARY_SEARCH_DEFAULT_DIRS, LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR,
            Library as WindowsLibrary,
        };
        let path = path.canonicalize().map_err(|error| error.to_string())?;
        // SAFETY: caller retains the library; the absolute path is selected by
        // executable-relative discovery or an explicit FOXIDE_LIBMPV override.
        unsafe {
            WindowsLibrary::load_with_flags(
                path,
                LOAD_LIBRARY_SEARCH_DEFAULT_DIRS | LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR,
            )
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
    fn load(helper: Option<&player_services::VideoSiteTool>) -> Result<Arc<Self>, String> {
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
        for path in candidates {
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
            let loaded = unsafe { Self::from_library(library, helper) };
            match loaded {
                Ok(api) => return Ok(Arc::new(api)),
                Err(error) => errors.push(error),
            }
        }
        Err(format!(
            "Playback engine unavailable: {}",
            errors.join("; ")
        ))
    }

    unsafe fn from_library(
        library: Library,
        helper: Option<&player_services::VideoSiteTool>,
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
            ] {
                if option(api.handle, name.as_ptr(), value.as_ptr()) < 0 {
                    return Err("libmpv rejected a required playback option".into());
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
        let helper = helper
            .map(player_services::VideoSiteTool::new)
            .transpose()?;
        let api = Api::load(helper.as_ref())?;
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
            .name("foxide-engine".into())
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
}

#[cfg(all(test, feature = "video-site-integration"))]
#[path = "native_sites.rs"]
mod video_site_tests;

#[cfg(all(test, feature = "live-service-integration"))]
#[path = "native_live.rs"]
mod live_services;
