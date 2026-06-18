use crate::catalog::{EpisodeId, MovieId};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TitleId {
    Movie(MovieId),
    Episode(EpisodeId),
}
