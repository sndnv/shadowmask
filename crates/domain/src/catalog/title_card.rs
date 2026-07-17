use crate::catalog::{EpisodeCard, Movie};

#[derive(Debug, Clone)]
pub enum TitleCard {
    Movie(Movie),
    Episode(EpisodeCard),
}
