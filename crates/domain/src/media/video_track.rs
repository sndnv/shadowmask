#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HdrFormat {
    Hdr10,
    Hdr10Plus,
    DolbyVision,
    Hlg,
}

#[derive(Debug, Clone)]
pub struct VideoTrack {
    pub index: u32,
    pub codec: String,
    pub width: u32,
    pub height: u32,
    pub bit_depth: u8,
    pub hdr: Option<HdrFormat>,
    pub frame_rate: f32,
    pub bitrate: Option<u64>,
}
