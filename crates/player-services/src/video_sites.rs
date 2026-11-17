use std::path::{Path, PathBuf};

/// A deliberately selected native helper, passed as one mpv script option.
#[derive(Clone, Debug)]
pub struct VideoSiteTool {
    path: PathBuf,
}
impl VideoSiteTool {
    pub fn new(path: &Path) -> Result<Self, String> {
        fn validate(path: &Path) -> Result<(), String> {
            let text = path.to_str().ok_or("Helper path must be Unicode")?;
            let separator = if cfg!(windows) { ';' } else { ':' };
            if text.contains(separator) || text.chars().any(char::is_control) {
                return Err(
                    "Helper path cannot contain control characters or path-list separators".into(),
                );
            }
            Ok(())
        }
        validate(path)?;
        let path = player_core::external_path(crate::process::trusted_executable(path)?);
        validate(&path)?;
        Ok(Self { path })
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn script_option(&self) -> String {
        format!(
            "ytdl_hook-ytdl_path={}",
            self.path.to_str().expect("Validated Unicode path")
        )
    }
}
