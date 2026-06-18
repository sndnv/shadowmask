#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SubtitleFormat {
    Srt,
    Ass,
    Vtt,
    Pgs,
    VobSub,
}
