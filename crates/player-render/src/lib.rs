//! Auditable native rendering seam; the policy/UI crates remain unsafe-free.
#[cfg(feature = "desktop")]
mod gpu;
#[cfg(feature = "desktop")]
mod native;
#[cfg(feature = "desktop")]
pub use gpu::attach;
#[cfg(feature = "desktop")]
pub use native::{Engine, Snapshot, Track};

#[cfg(any(feature = "desktop", test))]
mod render_contract;
