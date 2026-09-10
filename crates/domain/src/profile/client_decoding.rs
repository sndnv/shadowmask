use crate::media::HdrFormat;
use crate::profile::{AudioCodecCap, Container, VideoCodecCap};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ClientDecoding {
    pub containers: Option<Vec<Container>>,
    pub video: Option<Vec<VideoCodecCap>>,
    pub audio: Option<Vec<AudioCodecCap>>,
    pub hdr: Option<Vec<HdrFormat>>,
    pub max_width: Option<u32>,
    pub max_height: Option<u32>,
    pub max_bitrate: Option<u64>,
    pub max_frame_rate: Option<u32>,
}
