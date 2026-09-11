use crate::catalog::{EpisodeCard, Movie, Series};

#[derive(Debug, Clone)]
pub enum HubItem {
    Movie(Movie),
    Series { series: Series, episode_count: Option<u32> },
    Episode(Box<EpisodeCard>),
}

#[derive(Debug, Clone)]
pub struct Hub {
    pub id: String,
    pub title: String,
    pub items: Vec<HubItem>,
}
