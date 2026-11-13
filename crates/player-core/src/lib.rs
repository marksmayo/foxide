//! Backend-independent playback policy. No decoder or rendering implementation yet.

/// Conservative hardware-decoding policy shared by the launcher and benchmark.
pub const HWDEC: &str = "auto-safe";

/// Some external Windows players reject Rust's verbatim canonical path spelling.
/// Keep non-Unicode paths unchanged instead of performing a lossy conversion.
pub fn external_path(path: std::path::PathBuf) -> std::path::PathBuf {
    #[cfg(windows)]
    if let Some(text) = path.to_str() {
        if let Some(rest) = text.strip_prefix(r"\\?\UNC\") {
            return format!(r"\\{rest}").into();
        }
        if let Some(rest) = text.strip_prefix(r"\\?\")
            && rest.as_bytes().get(1) == Some(&b':')
        {
            return rest.into();
        }
    }
    path
}

/// A measured hardware decoder name alone does not prove GPU surface sharing.
#[derive(Debug, PartialEq, Eq)]
pub enum DecodePath {
    Software,
    CopyBack,
    HardwareUnverified,
    Unknown,
}

pub fn classify_decode_path(name: Option<&str>) -> DecodePath {
    match name {
        None | Some("") => DecodePath::Unknown,
        Some("no") => DecodePath::Software,
        Some(name) if name.ends_with("-copy") => DecodePath::CopyBack,
        Some(_) => DecodePath::HardwareUnverified,
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    TogglePause,
    SeekRelative(f64),
    SeekExact(f64),
    SetSpeed(f64),
    Next,
    Previous,
    ToggleFullscreen,
    Quit,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(windows)]
    fn external_players_receive_normal_drive_and_unc_paths() {
        assert_eq!(
            external_path(r"\\?\C:\media\clip with spaces.mp4".into()),
            std::path::PathBuf::from(r"C:\media\clip with spaces.mp4")
        );
        assert_eq!(
            external_path(r"\\?\UNC\server\share\clip.mp4".into()),
            std::path::PathBuf::from(r"\\server\share\clip.mp4")
        );
    }

    #[test]
    fn hardware_is_never_automatically_declared_zero_copy() {
        assert_eq!(
            classify_decode_path(Some("d3d11va")),
            DecodePath::HardwareUnverified
        );
        assert_eq!(
            classify_decode_path(Some("vaapi-copy")),
            DecodePath::CopyBack
        );
        assert_eq!(classify_decode_path(Some("no")), DecodePath::Software);
        assert_eq!(classify_decode_path(None), DecodePath::Unknown);
    }
}
