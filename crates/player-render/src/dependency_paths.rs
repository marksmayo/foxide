//! Windows libraries are anchored to the executable, never to the media folder.
use std::path::{Path, PathBuf};

pub(crate) fn windows_libraries(executable: &Path) -> Vec<PathBuf> {
    let mut paths = vec![executable.with_file_name("libmpv-2.dll")];
    paths.extend(
        executable
            .ancestors()
            .skip(1)
            .take(3)
            .map(|root| root.join("tools/libmpv/libmpv-2.dll")),
    );
    paths
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn installed_library_precedes_development_copies_and_all_paths_are_anchored() {
        let root = std::env::current_dir().unwrap();
        let executable = root.join("application/target/debug/foxide-desktop.exe");
        let paths = windows_libraries(&executable);
        assert_eq!(paths[0], executable.with_file_name("libmpv-2.dll"));
        assert_eq!(paths.len(), 4);
        assert_eq!(paths[3], root.join("application/tools/libmpv/libmpv-2.dll"));
        assert!(
            paths
                .iter()
                .all(|p| p.is_absolute() && p.starts_with(root.join("application")))
        );
        assert!(!paths.contains(&root.join("tools/libmpv/libmpv-2.dll")));
    }
}
