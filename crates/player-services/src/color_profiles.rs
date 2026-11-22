use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};
/// An explicitly selected RGB display profile. Native color management validates
/// color transforms; this boundary limits file size and rejects malformed tables.
#[derive(Clone, Debug)]
pub struct DisplayProfile {
    path: PathBuf,
}
impl DisplayProfile {
    pub fn new(path: &Path) -> Result<Self, String> {
        if !path.is_absolute() {
            return Err("Choose an absolute ICC profile path".into());
        }
        let path = path.canonicalize().map_err(|e| e.to_string())?;
        if path
            .to_str()
            .is_none_or(|text| text.chars().any(char::is_control))
        {
            return Err("ICC profile path must be Unicode without control characters".into());
        }
        if !path.metadata().map_err(|e| e.to_string())?.is_file() {
            return Err("Choose a regular ICC profile file".into());
        }
        let file = File::open(&path).map_err(|e| e.to_string())?;
        if !file.metadata().map_err(|e| e.to_string())?.is_file() {
            return Err("Choose a regular ICC profile file".into());
        }
        let mut bytes = Vec::new();
        file.take(4 * 1024 * 1024 + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        if bytes.len() > 4 * 1024 * 1024 {
            return Err("ICC profile must not exceed 4 MiB".into());
        }
        if bytes.len() < 132 || &bytes[36..40] != b"acsp" {
            return Err("Invalid ICC header".into());
        }
        let number = |offset: usize| {
            u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap()) as usize
        };
        if number(0) != bytes.len() {
            return Err("ICC declared size does not match its file".into());
        }
        if &bytes[12..16] != b"mntr" || &bytes[16..20] != b"RGB " {
            return Err("Choose an RGB display ICC profile".into());
        }
        let count = number(128);
        if count == 0 || count > 4096 || 132 + count * 12 > bytes.len() {
            return Err("Invalid ICC tag table".into());
        }
        for index in 0..count {
            let start = number(132 + index * 12 + 4);
            let size = number(132 + index * 12 + 8);
            if start < 132 + count * 12
                || size < 8
                || start.checked_add(size).is_none_or(|end| end > bytes.len())
            {
                return Err("ICC tag data is outside its profile".into());
            }
        }
        Ok(Self { path })
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
}
