#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoCodecCap {
    pub codec: String,
    pub max_level: Option<String>,
    pub max_bit_depth: u8,
    pub smooth: bool,
}
