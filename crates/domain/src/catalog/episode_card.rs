use crate::catalog::{Episode, SeriesId};

#[derive(Debug, Clone)]
pub struct EpisodeCard {
    pub episode: Episode,
    pub series: Option<SeriesId>,
    pub series_title: Option<String>,
    pub season_number: Option<u16>,
}
