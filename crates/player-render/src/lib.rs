//! Auditable native rendering seam; the policy/UI crates remain unsafe-free.
#[cfg(feature = "desktop")]
mod gpu;
#[cfg(feature = "desktop")]
mod native;
#[cfg(feature = "desktop")]
mod thumbnail_cache;
#[cfg(feature = "desktop")]
mod thumbnails;
#[cfg(feature = "desktop")]
pub use gpu::attach;
#[cfg(feature = "gpu-integration")]
pub use gpu::attach_observed;
#[cfg(feature = "desktop")]
pub use gpu::attach_with_thumbnails;
#[cfg(feature = "desktop")]
pub use native::{Engine, Snapshot, Track};
#[cfg(feature = "desktop")]
pub use thumbnails::ThumbnailController;

/// Build the libmpv filter graph that preserves audio output and renders a waveform.
pub fn audio_visualization_filter(audio_track_id: u32) -> String {
    format!(
        "[aid{audio_track_id}] asplit [foxide-viz] [ao] ; [foxide-viz] showwaves=s=960x270:mode=cline:rate=24:colors=0x87e64b [vo]"
    )
}

#[cfg(any(feature = "desktop", test))]
mod render_contract;

#[cfg(any(feature = "desktop", test))]
mod dependency_paths;
