#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PlaybackPrep {
    pub keyframes: Vec<u64>,
    pub extracted_subtitles: Vec<u32>,
}
