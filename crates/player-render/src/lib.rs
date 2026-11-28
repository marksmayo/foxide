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
#[cfg(all(feature = "desktop", windows))]
pub use native::windows_runtime_missing;
#[cfg(feature = "desktop")]
pub use native::{Engine, Snapshot, Track};
#[cfg(feature = "desktop")]
pub use thumbnails::ThumbnailController;

#[cfg(any(feature = "desktop", test))]
mod render_contract;

#[cfg(any(feature = "desktop", test))]
mod dependency_paths;
