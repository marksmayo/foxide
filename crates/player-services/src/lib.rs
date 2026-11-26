//! Optional network and media tools, separate from playback policy.
mod radio;
pub use radio::{HttpClient, RadioDirectory, Station, public_http_url};
mod subtitles;
pub use subtitles::{SubtitleProvider, SubtitleResult};

mod http;
pub use http::CurlClient;
pub mod process;

pub use url::Url;

mod video_sites;
pub use video_sites::VideoSiteTool;

mod shaders;
pub use shaders::ShaderFile;

mod color_profiles;
pub use color_profiles::DisplayProfile;

mod updates;
pub use updates::{
    WindowsUpdate, download_and_stage_windows_update, latest_windows_update, parse_sha256sum,
    verify_windows_package,
};

/// Transport-independent Cast protocol framing.
pub mod cast_wire;
