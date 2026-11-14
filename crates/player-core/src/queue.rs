use std::{
    fs, io,
    path::{Path, PathBuf},
};

fn natural_key(name: &str) -> Vec<(bool, usize, String)> {
    let mut chunks = Vec::new();
    let lower = name.to_lowercase();
    let mut characters = lower.chars().peekable();
    while let Some(first) = characters.next() {
        let numeric = first.is_ascii_digit();
        let mut text = first.to_string();
        while characters
            .peek()
            .is_some_and(|c| c.is_ascii_digit() == numeric)
        {
            text.push(characters.next().unwrap());
        }
        let significant = if numeric {
            text.trim_start_matches('0')
        } else {
            &text
        };
        chunks.push((
            numeric,
            if numeric { significant.len() } else { 0 },
            significant.to_owned(),
        ));
    }
    chunks
}

/// Return only direct media files, in case-insensitive natural episode order.
/// Explicitly opened files are not restricted by this folder-queue allowlist.
pub fn folder_queue(directory: &Path) -> io::Result<Vec<PathBuf>> {
    const EXTENSIONS: &[&str] = &[
        "mp4", "mkv", "webm", "avi", "mov", "m4v", "ts", "m2ts", "wmv", "flv", "mpg", "mpeg",
        "flac", "mp3", "wav", "ogg", "opus", "m4a", "aac", "wma", "aiff", "ape",
    ];
    let mut files = Vec::new();
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let name = entry.file_name();
        if Path::new(&name)
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| EXTENSIONS.iter().any(|known| e.eq_ignore_ascii_case(known)))
            && entry.file_type()?.is_file()
        {
            files.push(entry.path());
        }
    }
    files.sort_by_cached_key(|p| {
        (
            natural_key(&p.file_name().unwrap().to_string_lossy()),
            p.clone(),
        )
    });
    Ok(files)
}
