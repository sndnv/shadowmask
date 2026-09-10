use crate::profile::ClientDecoding;

#[derive(Debug, Clone)]
pub struct ClientCapabilities {
    pub platform: String,
    pub profile_version: u32,
    pub max_bitrate: Option<u64>,
    pub decoding: Option<ClientDecoding>,
}
