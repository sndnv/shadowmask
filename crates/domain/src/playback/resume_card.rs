use crate::catalog::{ArtworkRef, TitleId};

#[derive(Debug, Clone)]
pub struct ResumeCard {
    pub title: TitleId,
    pub display_title: String,
    pub artwork: Vec<ArtworkRef>,
    pub duration_ms: u64,
    pub progress_percent: u8,
    pub year: Option<u16>,
    pub series_title: Option<String>,
    pub series_artwork: Vec<ArtworkRef>,
    pub season_number: Option<u16>,
    pub episode_number: Option<u16>,
}

pub fn progress_percent(position_ms: u64, duration_ms: u64) -> u8 {
    (position_ms.saturating_mul(100) / duration_ms.max(1)).min(100) as u8
}

#[cfg(test)]
mod tests {
    use super::progress_percent;

    #[test]
    fn computes_bounded_percent() {
        assert_eq!(progress_percent(0, 1000), 0);
        assert_eq!(progress_percent(500, 1000), 50);
        assert_eq!(progress_percent(1000, 1000), 100);
        assert_eq!(progress_percent(2000, 1000), 100);
    }

    #[test]
    fn guards_zero_duration() {
        assert_eq!(progress_percent(500, 0), 100);
    }
}
