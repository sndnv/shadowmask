use crate::catalog::{ArtworkRef, Season};

#[derive(Debug, Clone)]
pub struct SeasonCard {
    pub season: Season,
    pub series_title: Option<String>,
    pub series_artwork: Vec<ArtworkRef>,
}
