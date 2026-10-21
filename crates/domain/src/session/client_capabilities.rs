use crate::profile::ClientDecoding;

#[derive(Debug, Clone)]
pub struct ClientCapabilities {
    pub platform: String,
    pub profile_version: u32,
    pub max_bitrate: Option<u64>,
    pub decoding: Option<ClientDecoding>,
    pub tracks: TrackHandling,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TrackHandling {
    pub embedded: bool,
    pub image: bool,
    pub external: bool,
    pub rendition: bool,
    pub audio: bool,
}
