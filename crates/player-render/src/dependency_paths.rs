//! Windows libraries are anchored to the executable, never to the media folder.
use std::path::{Path, PathBuf};

pub(crate) fn windows_libraries(executable: &Path) -> Vec<PathBuf> {
    vec![
        executable.with_file_name("libmpv-2.dll"),
        executable
            .parent()
            .expect("Executable has a parent")
            .join("tools/libmpv/libmpv-2.dll"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn library_discovery_never_leaves_the_application_directory() {
        let root = std::env::current_dir().unwrap();
        let executable = root.join("application/target/debug/foxide-desktop.exe");
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
