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
#[cfg(test)]
#[repr(C)]
struct LogMessage {
    prefix: *const c_char,
    level: *const c_char,
    text: *const c_char,
    log_level: c_int,
}

pub(crate) struct Api {
    pub handle: Handle,
    _library: Library,
    command: unsafe extern "C" fn(Handle, *const *const c_char) -> c_int,
    property: unsafe extern "C" fn(Handle, *const c_char) -> *mut c_char,
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
    fn load() -> Result<Arc<Self>, String> {
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
            let loaded = unsafe { Self::from_library(library) };
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

    unsafe fn from_library(library: Library) -> Result<Self, String> {
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
            if initialize(api.handle) < 0 {
                return Err("libmpv initialization failed".into());
            }
            api.initialized = true;
            #[cfg(test)]
            {
                let logs = api
                    ._library
                    .get::<unsafe extern "C" fn(Handle, *const c_char) -> c_int>(
                        b"mpv_request_log_messages\0",
                    )
                    .map_err(|error| error.to_string())?;
                logs(api.handle, c"debug".as_ptr());
            }
            Ok(api)
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

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Track {
    pub id: u32,
    pub label: String,
    pub kind: String,
    pub selected: bool,
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Snapshot {
    pub path: String,
    pub title: String,
    pub loaded: bool,
    pub paused: bool,
    pub position: f64,
    pub duration: f64,
    pub speed: f64,
    pub volume: f64,
    pub muted: bool,
    pub hwdec: String,
    pub tracks: Vec<Track>,
    pub error: String,
}
pub struct Engine {
    pub(crate) api: Arc<Api>,
    pub(crate) ready: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
    sender: SyncSender<Vec<String>>,
    snapshot: Arc<Mutex<Snapshot>>,
    #[cfg(test)]
    pub(crate) diagnostics: Arc<Mutex<Vec<String>>>,
}
impl Engine {
    pub fn new() -> Result<Self, String> {
        let api = Api::load()?;
        let ready = Arc::new(AtomicBool::new(false));
        let stop = Arc::new(AtomicBool::new(false));
        let snapshot = Arc::new(Mutex::new(Snapshot::default()));
        #[cfg(test)]
        let diagnostics = Arc::new(Mutex::new(Vec::<String>::new()));
        #[cfg(test)]
        let diagnostic_output = diagnostics.clone();
        let (sender, receiver) = mpsc::sync_channel::<Vec<String>>(32);
        let (client, active, stopped, output) =
            (api.clone(), ready.clone(), stop.clone(), snapshot.clone());
        thread::Builder::new()
            .name("foxide-engine".into())
            .spawn(move || {
                let mut last_path = String::new();
                let mut tracks = Vec::new();
                let mut error = String::new();
                while !stopped.load(Ordering::Acquire) {
                    if !active.load(Ordering::Acquire) {
                        thread::sleep(Duration::from_millis(20));
                        continue;
                    }
                    match receiver.recv_timeout(Duration::from_millis(200)) {
                        Ok(args) => {
                            if args.first().is_some_and(|s| s == "loadfile") {
                                error.clear();
                                last_path.clear();
                            }
                            if let Err(e) = client.send(&args) {
                                error = e;
                            }
                            tracks.clear();
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
                            #[cfg(test)]
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
                        tracks.clear();
                        let count = client.number(c"track-list/count").clamp(0.0, 256.0) as u32;
                        for i in 0..count {
                            let kind = client.track_property(i, "type");
                            if kind != "audio" && kind != "sub" {
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
                            tracks.push(Track {
                                id,
                                label: format!("{id} · {label}"),
                                kind,
                                selected: client.track_property(i, "selected") == "yes",
                            });
                        }
                        last_path.clone_from(&path);
                    }
                    let next = Snapshot {
                        loaded: !path.is_empty()
                            && client.get(c"idle-active") == "no"
                            && error.is_empty(),
                        path,
                        title: client.get(c"media-title"),
                        paused: client.get(c"pause") == "yes",
                        position: client.number(c"time-pos"),
                        duration: client.number(c"duration"),
                        speed: client.number(c"speed"),
                        volume: client.number(c"volume"),
                        muted: client.get(c"mute") == "yes",
                        hwdec: client.get(c"hwdec-current"),
                        tracks: tracks.clone(),
                        error: error.clone(),
                    };
                    if let Ok(mut current) = output.lock() {
                        *current = next;
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
            #[cfg(test)]
            diagnostics,
        })
    }
    pub fn command(&self, args: Vec<String>) -> Result<(), String> {
        self.sender
            .try_send(args)
            .map_err(|e| format!("Player is busy: {e}"))
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
    fn cloning_snapshot_shares_immutable_track_metadata() {
        let snapshot = Snapshot { tracks: vec![Track { id: 1, label: "English".into(), kind: "audio".into(), selected: true }], ..Default::default() };
        let copy = snapshot.clone();
        assert_eq!(copy, snapshot);
        assert_eq!(copy.tracks.as_ptr(), snapshot.tracks.as_ptr(),
            "A position update must not allocate and clone immutable track strings");
    }
}
