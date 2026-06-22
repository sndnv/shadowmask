use serde::Deserialize;

use domain::session::ClientCapabilities;

#[derive(Debug, Deserialize)]
pub struct ClientCapabilitiesDto {
    pub platform: String,
    pub profile_version: u32,
    pub max_bitrate: Option<u64>,
}

impl From<ClientCapabilitiesDto> for ClientCapabilities {
    fn from(c: ClientCapabilitiesDto) -> Self {
        ClientCapabilities {
            platform: c.platform,
            profile_version: c.profile_version,
            max_bitrate: c.max_bitrate,
        }
    }
}
