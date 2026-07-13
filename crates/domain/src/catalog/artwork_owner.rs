use crate::catalog::{CollectionId, EpisodeId, MovieId, SeasonId, SeriesId};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ArtworkOwner {
    Movie(MovieId),
    Series(SeriesId),
    Season(SeasonId),
    Episode(EpisodeId),
    Collection(CollectionId),
}
