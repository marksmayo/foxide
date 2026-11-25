use libloading::Library;
use std::{
    ffi::{CStr, CString, c_char, c_int, c_ulong, c_void},
    fs,
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
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
            ] {
                if option(api.handle, name.as_ptr(), value.as_ptr()) < 0 {
                    return Err("libmpv rejected a required playback option".into());
                }
            }
            for (name, value) in extra_options {
                if option(api.handle, name.as_ptr(), value.as_ptr()) < 0 {
                    return Err(format!(
                        "libmpv rejected the requested encoding option {}",
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AudioExportFormat {
    WavPcm16,
    Flac,
}

impl AudioExportFormat {
    fn extension(self) -> &'static str {
        match self {
            Self::WavPcm16 => "wav",
            Self::Flac => "flac",
        }
    }

    fn container(self) -> &'static CStr {
        match self {
            Self::WavPcm16 => c"wav",
            Self::Flac => c"flac",
        }
    }

    fn codec(self) -> &'static CStr {
        match self {
            Self::WavPcm16 => c"pcm_s16le",
            Self::Flac => c"flac",
        }
    }
}

/// Export one local media file's audio stream. The destination is created only
/// after encoding succeeds, and an existing file is never replaced.
pub fn export_audio(
    input: &Path,
    destination: &Path,
    format: AudioExportFormat,
) -> Result<(), String> {
    let (input, destination, parent) = validate_export_paths(input, destination, format)?;
    let temp = tempfile::Builder::new()
        .prefix(".foxide-audio-export-")
        .suffix(&format!(".{}", format.extension()))
        .tempfile_in(&parent)
        .map_err(|error| format!("Could not reserve a temporary export file: {error}"))?;
    let temp_path = temp.into_temp_path();
    let output_value = CString::new(temp_path.to_string_lossy().as_bytes())
        .map_err(|_| "Export path contains a null byte")?;
    let options = [
        (c"o", output_value.as_c_str()),
        (c"of", format.container()),
        (c"oac", format.codec()),
        (c"ovc", c"no"),
        (c"vid", c"no"),
        (c"keep-open", c"no"),
    ];
    let api = Api::load(None, false, &options)?;
    let input_value = input.to_string_lossy().into_owned();
    api.send(&["loadfile".into(), input_value, "replace".into()])?;

    let mut file_loaded = false;
    let mut source_duration = None;
    loop {
        // SAFETY: event contents are read before the next wait_event call, which
        // invalidates the previous event as documented by libmpv.
        let event = unsafe { (api.wait_event)(api.handle, 0.25) };
        if event.is_null() {
            return Err("libmpv stopped while exporting audio".into());
        }
        // SAFETY: event is non-null and its payload is inspected before waiting again.
        let event = unsafe { &*event };
        match event.kind {
            0 => continue,
            8 => {
                file_loaded = true;
                source_duration = api
                    .get(c"duration")
                    .parse::<f64>()
                    .ok()
                    .filter(|duration| duration.is_finite() && *duration > 0.0);
            }
            7 => {
                if event.data.is_null() {
                    return Err("libmpv ended audio export without a result".into());
                }
                // SAFETY: MPV_EVENT_END_FILE carries mpv_event_end_file.
                let end = unsafe { &*event.data.cast::<End>() };
                if end.reason != 0 || !file_loaded {
                    return Err(format!(
                        "Audio export did not finish successfully (reason {}, error {})",
                        end.reason, end.error
                    ));
                }
                break;
            }
            1 => return Err("libmpv shut down during audio export".into()),
            _ => {}
        }
    }
    drop(api);

    let source_duration =
        source_duration.ok_or("Could not determine the source duration for audio export")?;
    trim_export_to_duration(&temp_path, format, source_duration)?;

    let metadata = fs::metadata(&temp_path)
        .map_err(|error| format!("libmpv did not produce an audio file: {error}"))?;
    if metadata.len() < 44 {
        return Err("The exported audio file is empty or incomplete".into());
    }
    temp_path
        .persist_noclobber(&destination)
        .map_err(|error| format!("Could not create the export destination: {}", error.error))?;
    Ok(())
}

fn trim_export_to_duration(
    path: &Path,
    format: AudioExportFormat,
    duration: f64,
) -> Result<(), String> {
    let mut file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .map_err(|error| format!("Could not inspect encoded audio: {error}"))?;
    match format {
        AudioExportFormat::WavPcm16 => trim_wav_to_duration(&mut file, duration),
        AudioExportFormat::Flac => trim_flac_to_duration(&mut file, duration),
    }
}

fn expected_frames(duration: f64, sample_rate: u32) -> Result<u64, String> {
    let frames = duration * f64::from(sample_rate);
    if !frames.is_finite() || frames <= 0.0 || frames > u64::MAX as f64 {
        return Err("Source duration is outside the supported export range".into());
    }
    Ok(frames.round() as u64)
}

fn trim_wav_to_duration(file: &mut fs::File, duration: f64) -> Result<(), String> {
    let file_len = file.metadata().map_err(|error| error.to_string())?.len();
    let mut header = [0; 12];
    file.seek(SeekFrom::Start(0))
        .map_err(|error| error.to_string())?;
    file.read_exact(&mut header)
        .map_err(|error| error.to_string())?;
    let rf64 = &header[..4] == b"RF64";
    if (!rf64 && &header[..4] != b"RIFF") || &header[8..12] != b"WAVE" {
        return Err("libmpv produced an unsupported WAV container".into());
    }

    let mut ds64 = None;
    let mut format = None;
    let mut data = None;
    let mut offset = 12_u64;
    while offset + 8 <= file_len {
        file.seek(SeekFrom::Start(offset))
            .map_err(|error| error.to_string())?;
        let mut chunk_header = [0; 8];
        file.read_exact(&mut chunk_header)
            .map_err(|error| error.to_string())?;
        let size32 = u32::from_le_bytes(chunk_header[4..8].try_into().unwrap());
        let payload = offset + 8;
        let mut size = u64::from(size32);
        if &chunk_header[..4] == b"ds64" {
            if size32 < 28 {
                return Err("WAV RF64 ds64 chunk is incomplete".into());
            }
            let mut values = [0; 28];
            file.read_exact(&mut values)
                .map_err(|error| error.to_string())?;
            ds64 = Some((
                payload,
                u64::from_le_bytes(values[8..16].try_into().unwrap()),
            ));
        } else if &chunk_header[..4] == b"fmt " {
            if size32 < 16 {
                return Err("WAV format chunk is incomplete".into());
            }
            let mut values = [0; 16];
            file.read_exact(&mut values)
                .map_err(|error| error.to_string())?;
            format = Some((
                u16::from_le_bytes(values[0..2].try_into().unwrap()),
                u16::from_le_bytes(values[2..4].try_into().unwrap()),
                u32::from_le_bytes(values[4..8].try_into().unwrap()),
                u16::from_le_bytes(values[14..16].try_into().unwrap()),
            ));
        } else if &chunk_header[..4] == b"data" {
            if rf64 && size32 == u32::MAX {
                size = ds64
                    .map(|(_, data_size)| data_size)
                    .ok_or("WAV RF64 data has no ds64 size")?;
            }
            data = Some((offset + 4, payload, size));
        }
        offset = payload
            .checked_add(size)
            .and_then(|end| end.checked_add(size % 2))
            .ok_or("WAV chunk size overflow")?;
        if offset > file_len {
            return Err("WAV chunk extends beyond the output file".into());
        }
    }

    let (format_code, channels, sample_rate, bits) =
        format.ok_or("WAV export has no format chunk")?;
    if format_code != 1 || channels == 0 || bits != 16 || sample_rate == 0 {
        return Err("WAV export is not valid PCM16 audio".into());
    }
    let (data_size_field, data_start, data_size) = data.ok_or("WAV export has no data chunk")?;
    let bytes_per_frame = u64::from(channels) * 2;
    let target_size = expected_frames(duration, sample_rate)?
        .checked_mul(bytes_per_frame)
        .ok_or("WAV target size overflow")?;
    if target_size > data_size || target_size % bytes_per_frame != 0 {
        return Err("WAV export ended before the source duration".into());
    }
    let removed = data_size - target_size;
    if removed == 0 {
        return Ok(());
    }

    let old_data_end = data_start
        .checked_add(data_size)
        .filter(|end| *end <= file_len)
        .ok_or("WAV data extends beyond the output file")?;
    let new_data_end = data_start
        .checked_add(target_size)
        .ok_or("WAV target size overflow")?;
    let mut read_at = old_data_end;
    let mut write_at = new_data_end;
    let mut buffer = [0; 64 * 1024];
    while read_at < file_len {
        let count = (file_len - read_at).min(buffer.len() as u64) as usize;
        file.seek(SeekFrom::Start(read_at))
            .map_err(|error| error.to_string())?;
        file.read_exact(&mut buffer[..count])
            .map_err(|error| error.to_string())?;
        file.seek(SeekFrom::Start(write_at))
            .map_err(|error| error.to_string())?;
        file.write_all(&buffer[..count])
            .map_err(|error| error.to_string())?;
        read_at += count as u64;
        write_at += count as u64;
    }
    let new_len = file_len - removed;
    file.set_len(new_len).map_err(|error| error.to_string())?;
    if rf64 {
        let (ds64_offset, _) = ds64.ok_or("WAV RF64 container has no ds64 chunk")?;
        file.seek(SeekFrom::Start(ds64_offset))
            .map_err(|error| error.to_string())?;
        file.write_all(&(new_len - 8).to_le_bytes())
            .and_then(|_| file.write_all(&target_size.to_le_bytes()))
            .and_then(|_| file.write_all(&(target_size / bytes_per_frame).to_le_bytes()))
            .map_err(|error| error.to_string())?;
    } else {
        let riff_size =
            u32::try_from(new_len - 8).map_err(|_| "WAV output exceeds RIFF's size limit")?;
        let data_size =
            u32::try_from(target_size).map_err(|_| "WAV data exceeds RIFF's size limit")?;
        file.seek(SeekFrom::Start(4))
            .map_err(|error| error.to_string())?;
        file.write_all(&riff_size.to_le_bytes())
            .map_err(|error| error.to_string())?;
        file.seek(SeekFrom::Start(data_size_field))
            .map_err(|error| error.to_string())?;
        file.write_all(&data_size.to_le_bytes())
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn trim_flac_to_duration(file: &mut fs::File, duration: f64) -> Result<(), String> {
    let mut header = [0; 42];
    file.seek(SeekFrom::Start(0))
        .map_err(|error| error.to_string())?;
    file.read_exact(&mut header)
        .map_err(|error| error.to_string())?;
    if &header[..4] != b"fLaC" || header[4] & 0x7f != 0 {
        return Err("libmpv produced an unsupported FLAC stream".into());
    }
    let metadata_size = u32::from_be_bytes([0, header[5], header[6], header[7]]);
    if metadata_size < 34 {
        return Err("FLAC STREAMINFO block is incomplete".into());
    }
    let packed = u64::from_be_bytes(header[18..26].try_into().unwrap());
    let sample_rate = ((packed >> 44) & 0x0f_ffff) as u32;
    if sample_rate == 0 {
        return Err("FLAC stream has an invalid sample rate".into());
    }
    let sample_mask = (1_u64 << 36) - 1;
    let current_samples = packed & sample_mask;
    let target_samples = expected_frames(duration, sample_rate)?;
    if target_samples > current_samples {
        return Err("FLAC export ended before the source duration".into());
    }
    if target_samples == current_samples {
        return Ok(());
    }
    let adjusted = (packed & !sample_mask) | target_samples;
    file.seek(SeekFrom::Start(18))
        .map_err(|error| error.to_string())?;
    file.write_all(&adjusted.to_be_bytes())
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod audio_export_trim_tests {
    use super::{
        AudioExportFormat, expected_frames, trim_flac_to_duration, trim_wav_to_duration,
        validate_export_paths,
    };
    use std::{fs, io::Write, path::Path};

    fn wav_fixture(data_size: u32, format_code: u16) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&0_u32.to_le_bytes());
        bytes.extend_from_slice(b"WAVEfmt ");
        bytes.extend_from_slice(&16_u32.to_le_bytes());
        bytes.extend_from_slice(&format_code.to_le_bytes());
        bytes.extend_from_slice(&1_u16.to_le_bytes());
        bytes.extend_from_slice(&4_u32.to_le_bytes());
        bytes.extend_from_slice(&8_u32.to_le_bytes());
        bytes.extend_from_slice(&2_u16.to_le_bytes());
        bytes.extend_from_slice(&16_u16.to_le_bytes());
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&data_size.to_le_bytes());
        bytes.extend_from_slice(&[0x2a; 16]);
        bytes.extend_from_slice(b"JUNK");
        bytes.extend_from_slice(&4_u32.to_le_bytes());
        bytes.extend_from_slice(b"tail");
        let riff_size = (bytes.len() - 8) as u32;
        bytes[4..8].copy_from_slice(&riff_size.to_le_bytes());
        bytes
    }

    fn plain_wav(data_size: u32) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&0_u32.to_le_bytes());
        bytes.extend_from_slice(b"WAVEfmt ");
        bytes.extend_from_slice(&16_u32.to_le_bytes());
        bytes.extend_from_slice(&1_u16.to_le_bytes());
        bytes.extend_from_slice(&1_u16.to_le_bytes());
        bytes.extend_from_slice(&4_u32.to_le_bytes());
        bytes.extend_from_slice(&8_u32.to_le_bytes());
        bytes.extend_from_slice(&2_u16.to_le_bytes());
        bytes.extend_from_slice(&16_u16.to_le_bytes());
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&data_size.to_le_bytes());
        bytes.extend(std::iter::repeat_n(0x2a, data_size as usize));
        let riff_size = (bytes.len() - 8) as u32;
        bytes[4..8].copy_from_slice(&riff_size.to_le_bytes());
        bytes
    }

    fn flac_fixture(sample_rate: u32, sample_count: u64) -> Vec<u8> {
        let mut bytes = [0_u8; 42];
        bytes[..4].copy_from_slice(b"fLaC");
        bytes[7] = 34;
        let packed = (u64::from(sample_rate) << 44) | (16_u64 << 36) | sample_count;
        bytes[18..26].copy_from_slice(&packed.to_be_bytes());
        bytes.to_vec()
    }

    #[test]
    fn expected_frames_rounds_duration_to_the_nearest_sample() {
        assert_eq!(expected_frames(1.0, 44_100).unwrap(), 44_100);
        assert_eq!(expected_frames(0.375, 8).unwrap(), 3);
        assert!(expected_frames(f64::NAN, 44_100).is_err());
        assert!(expected_frames(0.0, 44_100).is_err());
        assert!(expected_frames(f64::INFINITY, 44_100).is_err());
    }

    #[test]
    fn export_path_validation_rejects_non_files_and_invalid_destinations() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("source.wav");
        fs::write(&source, b"source").unwrap();
        let missing = directory.path().join("missing.wav");
        let source_directory = directory.path().join("source-folder");
        fs::create_dir(&source_directory).unwrap();

        assert!(
            validate_export_paths(
                &missing,
                &directory.path().join("out.wav"),
                AudioExportFormat::WavPcm16
            )
            .unwrap_err()
            .contains("existing local media file")
        );
        assert!(
            validate_export_paths(
                &source_directory,
                &directory.path().join("out.wav"),
                AudioExportFormat::WavPcm16
            )
            .unwrap_err()
            .contains("existing local media file")
        );
        assert!(
            validate_export_paths(
                &source,
                &directory.path().join("absent/out.wav"),
                AudioExportFormat::WavPcm16
            )
            .unwrap_err()
            .contains("existing destination folder")
        );
        assert!(
            validate_export_paths(&source, Path::new(""), AudioExportFormat::WavPcm16)
                .unwrap_err()
                .contains("file name")
        );
        assert!(
            validate_export_paths(
                &source,
                &directory.path().join("extensionless"),
                AudioExportFormat::WavPcm16
            )
            .unwrap_err()
            .contains(".wav or .flac")
        );
        assert!(
            validate_export_paths(
                &source,
                &directory.path().join("out.flac"),
                AudioExportFormat::WavPcm16
            )
            .unwrap_err()
            .contains(".wav destination")
        );
    }

    #[test]
    fn export_rejects_the_source_as_its_destination() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("source.wav");
        fs::write(&source, b"source").unwrap();

        let error =
            validate_export_paths(&source, &source, AudioExportFormat::WavPcm16).unwrap_err();

        assert_eq!(
            error,
            "The export destination must differ from the source file"
        );
    }

    #[test]
    fn wav_trimming_preserves_trailing_chunks_and_repairs_sizes() {
        let mut fixture = tempfile::NamedTempFile::new().unwrap();
        fixture.write_all(&wav_fixture(16, 1)).unwrap();

        trim_wav_to_duration(fixture.as_file_mut(), 1.0).unwrap();

        let output = fs::read(fixture.path()).unwrap();
        assert_eq!(&output[0..4], b"RIFF");
        assert_eq!(
            u32::from_le_bytes(output[4..8].try_into().unwrap()) as usize,
            output.len() - 8
        );
        assert_eq!(u32::from_le_bytes(output[40..44].try_into().unwrap()), 8);
        assert_eq!(&output[52..64], b"JUNK\x04\0\0\0tail");
    }

    #[test]
    fn rf64_trimming_repairs_large_container_and_sample_sizes() {
        let mut bytes = b"RF64\xff\xff\xff\xffWAVEds64\x1c\0\0\0".to_vec();
        bytes.extend_from_slice(&0_u64.to_le_bytes());
        bytes.extend_from_slice(&24_u64.to_le_bytes());
        bytes.extend_from_slice(&12_u64.to_le_bytes());
        bytes.extend_from_slice(&0_u32.to_le_bytes());
        bytes.extend_from_slice(b"fmt \x10\0\0\0");
        bytes.extend_from_slice(&1_u16.to_le_bytes());
        bytes.extend_from_slice(&1_u16.to_le_bytes());
        bytes.extend_from_slice(&4_u32.to_le_bytes());
        bytes.extend_from_slice(&8_u32.to_le_bytes());
        bytes.extend_from_slice(&2_u16.to_le_bytes());
        bytes.extend_from_slice(&16_u16.to_le_bytes());
        bytes.extend_from_slice(b"data\xff\xff\xff\xff");
        bytes.extend_from_slice(&[0x2a; 24]);
        let mut fixture = tempfile::NamedTempFile::new().unwrap();
        fixture.write_all(&bytes).unwrap();

        trim_wav_to_duration(fixture.as_file_mut(), 2.0).unwrap();

        let output = fs::read(fixture.path()).unwrap();
        assert_eq!(output.len(), 96);
        assert_eq!(u64::from_le_bytes(output[20..28].try_into().unwrap()), 88);
        assert_eq!(u64::from_le_bytes(output[28..36].try_into().unwrap()), 16);
        assert_eq!(u64::from_le_bytes(output[36..44].try_into().unwrap()), 8);
        assert_eq!(&output[80..96], &[0x2a; 16]);
    }

    #[test]
    fn wav_trimming_rejects_missing_rf64_size_and_short_audio() {
        let mut missing_ds64 = tempfile::NamedTempFile::new().unwrap();
        missing_ds64
            .write_all(b"RF64\xff\xff\xff\xffWAVEdata\xff\xff\xff\xff")
            .unwrap();
        assert!(
            trim_wav_to_duration(missing_ds64.as_file_mut(), 1.0)
                .unwrap_err()
                .contains("no ds64 size")
        );

        let mut short = tempfile::NamedTempFile::new().unwrap();
        short.write_all(&plain_wav(8)).unwrap();
        assert!(
            trim_wav_to_duration(short.as_file_mut(), 2.0)
                .unwrap_err()
                .contains("ended before the source duration")
        );
    }

    #[test]
    fn wav_trimming_preserves_a_file_already_at_source_duration() {
        let original = plain_wav(8);
        let mut fixture = tempfile::NamedTempFile::new().unwrap();
        fixture.write_all(&original).unwrap();

        trim_wav_to_duration(fixture.as_file_mut(), 1.0).unwrap();

        assert_eq!(fs::read(fixture.path()).unwrap(), original);
    }

    #[test]
    fn wav_trimming_rejects_non_pcm16_and_out_of_bounds_data() {
        let mut float_wav = tempfile::NamedTempFile::new().unwrap();
        float_wav.write_all(&wav_fixture(16, 3)).unwrap();
        assert!(
            trim_wav_to_duration(float_wav.as_file_mut(), 1.0)
                .unwrap_err()
                .contains("PCM16")
        );

        let mut truncated = tempfile::NamedTempFile::new().unwrap();
        truncated.write_all(&wav_fixture(32, 1)).unwrap();
        assert!(
            trim_wav_to_duration(truncated.as_file_mut(), 1.0)
                .unwrap_err()
                .contains("extends beyond")
        );
    }

    #[test]
    fn wav_trimming_rejects_malformed_containers_and_missing_chunks() {
        for (bytes, expected) in [
            (b"not a wav!!!".to_vec(), "unsupported WAV container"),
            (b"RIFF\x04\0\0\0WAVE".to_vec(), "no format chunk"),
            (
                [b"RIFF\x1c\0\0\0WAVEfmt \x08\0\0\0".as_slice(), &[0; 8]].concat(),
                "format chunk is incomplete",
            ),
            (
                [
                    b"RIFF\x1c\0\0\0WAVEfmt \x10\0\0\0".as_slice(),
                    &[1, 0, 1, 0, 4, 0, 0, 0, 8, 0, 0, 0, 2, 0, 16, 0],
                ]
                .concat(),
                "no data chunk",
            ),
            (
                b"RF64\xff\xff\xff\xffWAVEds64\x08\0\0\0\0\0\0\0".to_vec(),
                "ds64 chunk is incomplete",
            ),
        ] {
            let mut fixture = tempfile::NamedTempFile::new().unwrap();
            fixture.write_all(&bytes).unwrap();

            let error = trim_wav_to_duration(fixture.as_file_mut(), 1.0).unwrap_err();

            assert!(
                error.contains(expected),
                "{error:?} did not contain {expected:?}"
            );
        }
    }

    #[test]
    fn flac_trimming_updates_streaminfo_sample_count() {
        let mut fixture = tempfile::NamedTempFile::new().unwrap();
        let mut bytes = [0_u8; 42];
        bytes[..4].copy_from_slice(b"fLaC");
        bytes[4] = 0x00;
        bytes[7] = 34;
        let packed = (44_100_u64 << 44) | (16_u64 << 36) | 88_200;
        bytes[18..26].copy_from_slice(&packed.to_be_bytes());
        fixture.write_all(&bytes).unwrap();

        trim_flac_to_duration(fixture.as_file_mut(), 1.0).unwrap();

        let output = fs::read(fixture.path()).unwrap();
        let adjusted = u64::from_be_bytes(output[18..26].try_into().unwrap());
        assert_eq!(adjusted & ((1_u64 << 36) - 1), 44_100);
    }

    #[test]
    fn flac_trimming_rejects_invalid_metadata_and_short_sources() {
        let mut short_header = tempfile::NamedTempFile::new().unwrap();
        short_header.write_all(b"fLaC").unwrap();
        assert!(trim_flac_to_duration(short_header.as_file_mut(), 1.0).is_err());

        let mut unsupported = flac_fixture(44_100, 44_100);
        unsupported[..4].copy_from_slice(b"nope");
        let mut incomplete = flac_fixture(44_100, 44_100);
        incomplete[7] = 33;
        for (bytes, duration, expected) in [
            (unsupported, 1.0, "unsupported FLAC stream"),
            (incomplete, 1.0, "STREAMINFO block is incomplete"),
            (flac_fixture(0, 44_100), 1.0, "invalid sample rate"),
            (
                flac_fixture(44_100, 44_100),
                2.0,
                "ended before the source duration",
            ),
        ] {
            let mut fixture = tempfile::NamedTempFile::new().unwrap();
            fixture.write_all(&bytes).unwrap();
            let error = trim_flac_to_duration(fixture.as_file_mut(), duration).unwrap_err();
            assert!(
                error.contains(expected),
                "{error:?} did not contain {expected:?}"
            );
        }
    }

    #[test]
    fn flac_trimming_leaves_an_exact_duration_unchanged() {
        let bytes = flac_fixture(44_100, 44_100);
        let mut fixture = tempfile::NamedTempFile::new().unwrap();
        fixture.write_all(&bytes).unwrap();

        trim_flac_to_duration(fixture.as_file_mut(), 1.0).unwrap();

        assert_eq!(fs::read(fixture.path()).unwrap(), bytes);
    }
}

fn validate_export_paths(
    input: &Path,
    destination: &Path,
    format: AudioExportFormat,
) -> Result<(PathBuf, PathBuf, PathBuf), String> {
    let input = input
        .canonicalize()
        .map_err(|_| "Choose an existing local media file to export")?;
    if !input.is_file() {
        return Err("Choose an existing local media file to export".into());
    }
    let parent = destination
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
        .canonicalize()
        .map_err(|_| "Choose an existing destination folder")?;
    let name = destination
        .file_name()
        .filter(|name| !name.is_empty())
        .ok_or("Choose a file name for the audio export")?;
    let extension = destination
        .extension()
        .and_then(|extension| extension.to_str())
        .ok_or("Choose a .wav or .flac destination")?;
    if !extension.eq_ignore_ascii_case(format.extension()) {
        return Err(format!(
            "Choose a .{} destination for this export format",
            format.extension()
        ));
    }
    let destination = parent.join(name);
    if same_path(&input, &destination) {
        return Err("The export destination must differ from the source file".into());
    }
    if destination.exists() {
        return Err("The export destination already exists; choose another file".into());
    }
    Ok((input, destination, parent))
}

#[cfg(windows)]
fn same_path(left: &Path, right: &Path) -> bool {
    left.to_string_lossy()
        .eq_ignore_ascii_case(&right.to_string_lossy())
}

#[cfg(not(windows))]
fn same_path(left: &Path, right: &Path) -> bool {
    left == right
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

    fn create(helper: Option<&std::path::Path>, thumbnail: bool) -> Result<Self, String> {
        let helper = helper
            .map(player_services::VideoSiteTool::new)
            .transpose()?;
        let api = Api::load(helper.as_ref(), thumbnail, &[])?;
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
}

#[cfg(all(test, windows))]
mod missing_runtime_tests {
    use super::{Api, windows_runtime_error};

    #[test]
    fn absent_installed_runtime_explains_how_to_recover() {
        let Err(error) = Api::load(None, false, &[]) else {
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
