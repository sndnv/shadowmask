use crate::catalog::{Episode, SeasonId, SeriesId};

#[derive(Debug, Clone)]
pub struct EpisodeContext {
    pub episode: Episode,
    pub season: SeasonId,
    pub season_number: u16,
    pub season_title: Option<String>,
    pub series: SeriesId,
}
