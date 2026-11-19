//! Windows libraries are anchored to the executable, never to the media folder.
use std::path::{Path, PathBuf};

pub(crate) fn windows_libraries(executable: &Path) -> Vec<PathBuf> {
    let application_dir = executable.parent().expect("Executable has a parent");
    let mut paths = vec![
        executable.with_file_name("libmpv-2.dll"),
        application_dir.join("tools/libmpv/libmpv-2.dll"),
    ];
    if application_dir
        .file_name()
        .is_some_and(|name| name == "debug")
        && application_dir
            .parent()
            .and_then(Path::file_name)
            .is_some_and(|name| name == "target")
    {
        if let Some(repository_root) = application_dir.parent().and_then(Path::parent) {
            paths.push(repository_root.join("tools/libmpv/libmpv-2.dll"));
        }
    }
    paths
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn library_discovery_never_leaves_the_application_directory() {
        let root = std::env::current_dir().unwrap();
        let executable = root.join("application/bin/foxide-desktop.exe");
        let paths = windows_libraries(&executable);
        assert_eq!(paths[0], executable.with_file_name("libmpv-2.dll"));
        assert_eq!(
            paths,
            vec![
                executable.with_file_name("libmpv-2.dll"),
                executable
                    .parent()
                    .unwrap()
                    .join("tools/libmpv/libmpv-2.dll")
            ]
        );
    }
}
