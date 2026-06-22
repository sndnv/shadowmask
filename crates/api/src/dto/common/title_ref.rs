use serde::{Deserialize, Serialize};

use domain::catalog::{EpisodeId, MovieId, TitleId};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type", content = "id")]
pub enum TitleRefDto {
    Movie(String),
    Episode(String),
}

impl From<TitleId> for TitleRefDto {
    fn from(t: TitleId) -> Self {
        match t {
            TitleId::Movie(id) => TitleRefDto::Movie(id.0),
            TitleId::Episode(id) => TitleRefDto::Episode(id.0),
        }
    }
}

impl From<TitleRefDto> for TitleId {
    fn from(t: TitleRefDto) -> Self {
        match t {
            TitleRefDto::Movie(id) => TitleId::Movie(MovieId(id)),
            TitleRefDto::Episode(id) => TitleId::Episode(EpisodeId(id)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_both_variants() {
        let movie = TitleId::Movie(MovieId("m1".into()));
        assert_eq!(TitleId::from(TitleRefDto::from(movie.clone())), movie);

        let episode = TitleId::Episode(EpisodeId("e1".into()));
        assert_eq!(TitleId::from(TitleRefDto::from(episode.clone())), episode);
    }
}
