use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};
/// An explicitly selected, bounded mpv user shader. Selection authorizes GPU code.
#[derive(Clone, Debug)]
pub struct ShaderFile {
    path: PathBuf,
}
impl ShaderFile {
    pub fn new(path: &Path) -> Result<Self, String> {
        if !path.is_absolute() {
            return Err("Choose an absolute shader path".into());
        }
        let path = path.canonicalize().map_err(|e| e.to_string())?;
        let text = path.to_str().ok_or("Shader path must be Unicode")?;
        if text.chars().any(char::is_control) {
            return Err("Shader path contains control characters".into());
        }
        let file = File::open(&path).map_err(|e| e.to_string())?;
        if !file.metadata().map_err(|e| e.to_string())?.is_file() {
            return Err("Choose a regular shader file".into());
        }
        let mut bytes = Vec::new();
        file.take(1024 * 1024 + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        if bytes.len() > 1024 * 1024 {
            return Err("Shader must not exceed 1 MiB".into());
        }
        let source = std::str::from_utf8(&bytes).map_err(|_| "Shader must be UTF-8 text")?;
        if source.contains('\0') || !source.lines().any(|line| line.starts_with("//!HOOK ")) {
            return Err("Shader must contain an mpv //!HOOK directive and no NUL bytes".into());
        }
        Ok(Self { path })
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
}
