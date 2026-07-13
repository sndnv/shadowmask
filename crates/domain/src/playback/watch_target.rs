use crate::catalog::{EpisodeId, MovieId, SeasonId, SeriesId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WatchTarget {
    Movie(MovieId),
    Episode(EpisodeId),
    Season(SeasonId),
    Series(SeriesId),
}
