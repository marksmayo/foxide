//! CPU benchmarks for backend-independent playback policy.
use std::{
    fs,
    path::{Path, PathBuf},
};

use divan::{Bencher, black_box};
use player_core::{DecodePath, PlaybackOptions, classify_decode_path, folder_queue};

fn main() {
    divan::main();
}

fn full_options() -> PlaybackOptions {
    PlaybackOptions {
        speed: 1.5,
        audio_track: Some(2),
        subtitle_track: Some(3),
        subtitle_files: (0..8)
            .map(|i| PathBuf::from(format!("/media/subs/episode {i:02}.en.srt")))
            .collect(),
        audio_delay: -0.25,
        subtitle_delay: 0.5,
        brightness: 10,
        contrast: -5,
        gamma: 3,
        deinterlace: true,
        tone_map: true,
        start: Some(42.5),
        resume_dir: Some(PathBuf::from("/home/user/.local/state/foxide/watch_later")),
        minimal: true,
        on_top: true,
        loop_playlist: true,
        ab_loop: Some((10.0, 20.0)),
        paused: true,
        ..PlaybackOptions::default()
    }
}

#[divan::bench]
fn mpv_args_default(bencher: Bencher) {
    let options = PlaybackOptions::default();
    bencher.bench_local(|| black_box(&options).mpv_args());
}

#[divan::bench]
fn mpv_args_all_features(bencher: Bencher) {
    let options = full_options();
    bencher.bench_local(|| black_box(&options).mpv_args());
}

#[divan::bench]
fn mpv_args_rejected(bencher: Bencher) {
    let options = PlaybackOptions {
        ab_loop: Some((20.0, 10.0)),
        ..full_options()
    };
    bencher.bench_local(|| black_box(&options).mpv_args());
}

#[divan::bench]
fn classify_decode_paths(bencher: Bencher) {
    let names: Vec<Option<String>> = [
        "",
        "no",
        "vaapi-copy",
        "d3d11va",
        "videotoolbox",
        "nvdec-copy",
        "vulkan",
        "auto-copy",
    ]
    .iter()
    .cycle()
    .take(256)
    .map(|name| Some(name.to_string()))
    .chain(std::iter::once(None))
    .collect();
    bencher.bench_local(|| {
        black_box(&names)
            .iter()
            .filter(|name| {
                classify_decode_path(black_box(name.as_deref())) == DecodePath::HardwareUnverified
            })
            .count()
    });
}

/// Temporary media folder removed when the benchmark finishes.
struct MediaFolder(PathBuf);

impl MediaFolder {
    fn new(files: usize) -> Self {
        let dir = fs::canonicalize(std::env::temp_dir())
            .unwrap()
            .join(format!("foxide-bench-queue-{}-{files}", std::process::id()));
        fs::create_dir(&dir).expect("benchmark fixture directory must be unused");
        fs::create_dir(dir.join("Extras.mkv")).unwrap();
        let extensions = ["mkv", "MP4", "srt", "webm", "nfo", "flac"];
        for i in 0..files {
            let extension = extensions[i % extensions.len()];
            let name = format!(
                "Show S01E{:0width$} Part {}.{extension}",
                i * 7 % files,
                i % 3,
                width = 1 + i % 3
            );
            fs::write(dir.join(name), b"").unwrap();
        }
        Self(dir)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for MediaFolder {
    fn drop(&mut self) {
        if let (Ok(target), Ok(base)) = (self.0.canonicalize(), std::env::temp_dir().canonicalize())
            && target.parent() == Some(base.as_path())
            && target.file_name() == self.0.file_name()
        {
            let _ = fs::remove_dir_all(target);
        }
    }
}

#[divan::bench(args = [16, 256])]
fn folder_queue_natural_order(bencher: Bencher, files: usize) {
    let folder = MediaFolder::new(files);
    bencher.bench_local(|| folder_queue(black_box(folder.path())).unwrap());
}
