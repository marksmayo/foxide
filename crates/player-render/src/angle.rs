//! Explicit, executable-independent ANGLE runtime loading.
use libloading::Library;
use std::path::Path;

/// Keep these libraries alive until the window and render context are gone.
pub struct AngleRuntime {
    _libraries: Vec<Library>,
}
impl AngleRuntime {
    pub fn from_environment() -> Result<Option<Self>, String> {
        let Some(directory) = std::env::var_os("FOXIDE_ANGLE_RUNTIME") else {
            return Ok(None);
        };
        if !cfg!(windows) {
            return Err("FOXIDE_ANGLE_RUNTIME is supported only on Windows".into());
        }
        let directory = Path::new(&directory);
        if !directory.is_absolute() {
            return Err("ANGLE runtime directory must be absolute".into());
        }
        let directory = directory.canonicalize().map_err(|e| e.to_string())?;
        let mut libraries = Vec::new();
        for name in ["libGLESv2.dll", "libEGL.dll"] {
            let path = directory.join(name);
            // SAFETY: explicit absolute runtime path; Windows dependencies are
            // restricted to this DLL's directory and trusted default directories.
            // Caller retains this guard until all rendering has been torn down.
            #[cfg(windows)]
            let library = unsafe {
                libloading::os::windows::Library::load_with_flags(&path, 0x00000100 | 0x00001000)
                    .map(Library::from)
            };
            #[cfg(not(windows))]
            let library: Result<Library, libloading::Error> = unsafe { Library::new(&path) };
            libraries.push(library.map_err(|e| format!("Cannot load {}: {e}", path.display()))?);
        }
        Ok(Some(Self {
            _libraries: libraries,
        }))
    }
}
