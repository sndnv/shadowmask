use serde::{Deserialize, Serialize};

use domain::catalog::TitleCard;

use crate::dto::catalog::{EpisodeResponse, MovieResponse};
use crate::dto::common::TitleRefDto;

#[derive(Debug, Deserialize)]
pub struct TitleBatchRequest {
    pub titles: Vec<TitleRefDto>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum TitleCardResponse {
    Movie(Box<MovieResponse>),
    Episode(Box<EpisodeResponse>),
}

impl From<TitleCard> for TitleCardResponse {
    fn from(card: TitleCard) -> Self {
        match card {
            TitleCard::Movie(m) => TitleCardResponse::Movie(Box::new(m.into())),
            TitleCard::Episode(e) => TitleCardResponse::Episode(Box::new(e.into())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::catalog::{
        ArtworkId, ArtworkRef, ArtworkWidth, Episode, EpisodeCard, EpisodeId, Movie, MovieId,
        SeasonId, SeriesId, TitleId,
    };
    use domain::metadata::ArtworkKind;
    use jiff::Timestamp;

    #[test]
    fn deserializes_title_list() {
        let req: TitleBatchRequest =
            serde_json::from_str(r#"{"titles":[{"type":"episode","id":"e1"}]}"#).unwrap();
        assert_eq!(req.titles.len(), 1);
        assert_eq!(
            TitleId::from(req.titles[0].clone()),
            TitleId::Episode(EpisodeId("e1".into()))
        );
    }

    #[test]
    fn serializes_movie_card_tagged() {
        let card = TitleCardResponse::from(TitleCard::Movie(Movie {
            id: MovieId("m1".into()),
            title: "Alpha".into(),
            sort_title: "alpha".into(),
            year: Some(2020),
            overview: None,
            runtime_minutes: None,
            content_rating: None,
            manually_edited: false,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
            artwork: Vec::new(),
        }));
        let value = serde_json::to_value(card).unwrap();
        assert_eq!(value["type"], "movie");
        assert_eq!(value["id"], "m1");
        assert_eq!(value["title"], "Alpha");
    }

    #[test]
    fn serializes_episode_card_tagged() {
        let card = TitleCardResponse::from(TitleCard::Episode(EpisodeCard {
            episode: Episode {
                id: EpisodeId("e1".into()),
                season: SeasonId("se1".into()),
                number: 3,
                title: "Pilot".into(),
                overview: None,
                runtime_minutes: None,
                air_date: None,
                manually_edited: false,
                added_at: Timestamp::UNIX_EPOCH,
                updated_at: Timestamp::UNIX_EPOCH,
                artwork: Vec::new(),
            },
            series: Some(SeriesId("sr1".into())),
            series_title: Some("Show ABC".into()),
            series_artwork: vec![ArtworkRef {
                id: ArtworkId("sp1".into()),
                kind: ArtworkKind::Poster,
                widths: vec![ArtworkWidth::new(180, "/art/sp1/180.jpg")],
            }],
            season_number: Some(1),
            season_title: Some("Specials".into()),
        }));
        let value = serde_json::to_value(card).unwrap();
        assert_eq!(value["type"], "episode");
        assert_eq!(value["id"], "e1");
        assert_eq!(value["season_id"], "se1");
        assert_eq!(value["series_id"], "sr1");
        assert_eq!(value["series_title"], "Show ABC");
        assert_eq!(value["season_number"], 1);
        assert_eq!(value["season_title"], "Specials");
        assert_eq!(value["series_artwork"]["posters"][0]["base"], "/images/sp1");
    }
}
