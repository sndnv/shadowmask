#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PlaybackPrep {
    pub keyframes_us: Vec<u64>,
    pub extracted_subtitles: Vec<u32>,
    pub prepared_with: String,
}
