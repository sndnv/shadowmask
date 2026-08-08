use crate::metadata::Artwork;

#[derive(Debug, Clone, Default)]
pub struct SeasonArtwork {
    pub number: u16,
    pub name: Option<String>,
    pub overview: Option<String>,
    pub artwork: Vec<Artwork>,
    pub episodes: Vec<EpisodeArtwork>,
}

#[derive(Debug, Clone, Default)]
pub struct EpisodeArtwork {
    pub number: u16,
    pub name: Option<String>,
    pub overview: Option<String>,
    pub air_date: Option<String>,
    pub runtime_minutes: Option<u32>,
    pub artwork: Vec<Artwork>,
}
