use crate::catalog::{EpisodeId, MovieId};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TitleId {
    Movie(MovieId),
    Episode(EpisodeId),
}

impl TitleId {
    pub fn id(&self) -> &str {
        match self {
            TitleId::Movie(id) => &id.0,
            TitleId::Episode(id) => &id.0,
        }
    }
}
