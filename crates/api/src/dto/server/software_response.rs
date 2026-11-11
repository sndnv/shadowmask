use serde::Serialize;

use super::Capability;

#[derive(Debug, Serialize)]
pub struct SoftwareResponse {
    pub version: String,
    pub yt_dlp: Option<String>,
    pub features: Vec<Capability>,
}

impl SoftwareResponse {
    pub fn current(yt_dlp: Option<String>, features: Vec<Capability>) -> Self {
        SoftwareResponse { version: env!("CARGO_PKG_VERSION").to_owned(), yt_dlp, features }
    }
}
