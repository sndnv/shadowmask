#[derive(Debug, Clone)]
pub struct ClientCapabilities {
    pub platform: String,
    pub profile_version: u32,
    pub max_bitrate: Option<u64>,
}
