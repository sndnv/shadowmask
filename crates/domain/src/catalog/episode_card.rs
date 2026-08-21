use crate::catalog::{ArtworkRef, Episode, SeriesId};

#[derive(Debug, Clone)]
pub struct EpisodeCard {
    pub episode: Episode,
    pub series: Option<SeriesId>,
    pub series_title: Option<String>,
    pub series_artwork: Vec<ArtworkRef>,
    pub season_number: Option<u16>,
    pub season_title: Option<String>,
}
