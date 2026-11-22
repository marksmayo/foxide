//! Explicit, executable-independent ANGLE runtime loading.
use libloading::Library;
use std::path::{Path, PathBuf};

/// Keep these libraries alive until the window and render context are gone.
pub struct AngleRuntime {
    _libraries: Vec<Library>,
}
impl AngleRuntime {
    pub fn from_environment() -> Result<Option<Self>, String> {
        let directory = std::env::var_os("FOXIDE_ANGLE_RUNTIME").map(PathBuf::from);
        let Some(directory) = runtime_directory(directory.as_deref(), cfg!(windows))? else {
            return Ok(None);
        };
        let libraries = load_runtime_libraries(&directory, |path| {
            // SAFETY: explicit absolute runtime path; Windows dependencies are
            // restricted to this DLL's directory and trusted default directories.
            // Caller retains this guard until all rendering has been torn down.
            #[cfg(windows)]
            let library = unsafe {
                libloading::os::windows::Library::load_with_flags(path, 0x00000100 | 0x00001000)
                    .map(Library::from)
            };
            #[cfg(not(windows))]
            let library: Result<Library, libloading::Error> = unsafe { Library::new(path) };
            library.map_err(|error| error.to_string())
        })?;
        Ok(Some(Self {
            _libraries: libraries,
        }))
    }
}

fn runtime_directory(
    directory: Option<&Path>,
    is_windows: bool,
) -> Result<Option<PathBuf>, String> {
    let Some(directory) = directory else {
        return Ok(None);
    };
    if !is_windows {
        return Err("FOXIDE_ANGLE_RUNTIME is supported only on Windows".into());
    }
    if !directory.is_absolute() {
        return Err("ANGLE runtime directory must be absolute".into());
    }
    directory
        .canonicalize()
        .map(Some)
        .map_err(|error| error.to_string())
}

fn load_runtime_libraries<T>(
    directory: &Path,
    mut load: impl FnMut(&Path) -> Result<T, String>,
) -> Result<Vec<T>, String> {
    let mut libraries = Vec::new();
    for name in ["libGLESv2.dll", "libEGL.dll"] {
        let path = directory.join(name);
        libraries
            .push(load(&path).map_err(|error| format!("Cannot load {}: {error}", path.display()))?);
    }
    Ok(libraries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    #[test]
    fn runtime_directory_validation_handles_missing_platform_and_path_cases() {
        assert_eq!(runtime_directory(None, true).unwrap(), None);
        assert_eq!(
            runtime_directory(Some(Path::new("C:/angle")), false).unwrap_err(),
            "FOXIDE_ANGLE_RUNTIME is supported only on Windows"
        );
        assert_eq!(
            runtime_directory(Some(Path::new("relative")), true).unwrap_err(),
            "ANGLE runtime directory must be absolute"
        );

        let root =
            std::env::temp_dir().join(format!("foxide-angle-runtime-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        assert_eq!(
            runtime_directory(Some(&root), true).unwrap(),
            Some(root.canonicalize().unwrap())
        );
        assert!(runtime_directory(Some(&root.join("missing")), true).is_err());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_loader_opens_both_dlls_in_order_and_reports_a_missing_dependency() {
        let seen = RefCell::new(Vec::new());
        let loaded = load_runtime_libraries(Path::new("C:/angle"), |path| {
            seen.borrow_mut().push(path.file_name().unwrap().to_owned());
            Ok(path.file_name().unwrap().to_string_lossy().into_owned())
        })
        .unwrap();
        assert_eq!(loaded, ["libGLESv2.dll", "libEGL.dll"]);
        assert_eq!(
            seen.into_inner(),
            vec![
                std::ffi::OsString::from("libGLESv2.dll"),
                std::ffi::OsString::from("libEGL.dll"),
            ]
        );

        let error = load_runtime_libraries(Path::new("C:/angle"), |path| {
            if path.ends_with("libEGL.dll") {
                Err("not found".into())
            } else {
                Ok(())
            }
        })
        .unwrap_err();
        let missing = Path::new("C:/angle").join("libEGL.dll");
        assert!(error.contains(&format!("Cannot load {}: not found", missing.display())));
    }
}
