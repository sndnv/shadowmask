#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioCodecCap {
    pub codec: String,
    pub max_channels: u8,
}
