#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessedArtwork {
    pub bytes: Vec<u8>,
    pub width: u32,
    pub height: u32,
}
