use crate::media::HdrFormat;
use crate::profile::{AudioCodecCap, Container, VideoCodecCap};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityProfile {
    pub containers: Vec<Container>,
    pub video: Vec<VideoCodecCap>,
    pub audio: Vec<AudioCodecCap>,
    pub hdr: Vec<HdrFormat>,
    pub max_width: u32,
    pub max_height: u32,
    pub max_bitrate: u64,
}
