use serde::Serialize;

use domain::catalog::{Episode, EpisodeCard};

use crate::dto::common::ArtworkDto;

#[derive(Debug, Serialize)]
pub struct EpisodeResponse {
    pub id: String,
    pub season_id: String,
    pub number: u16,
    pub title: String,
    pub overview: Option<String>,
    pub runtime_minutes: Option<u32>,
    pub air_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub series_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub series_title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub season_number: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub season_title: Option<String>,
    pub manually_edited: bool,
    pub added_at: String,
    pub updated_at: String,
    pub artwork: ArtworkDto,
    #[serde(skip_serializing_if = "ArtworkDto::is_empty")]
    pub series_artwork: ArtworkDto,
}

impl From<Episode> for EpisodeResponse {
    fn from(e: Episode) -> Self {
        EpisodeResponse {
            id: e.id.0,
            season_id: e.season.0,
            number: e.number,
            title: e.title,
            overview: e.overview,
            runtime_minutes: e.runtime_minutes,
            air_date: e.air_date.map(|d| d.to_string()),
            series_id: None,
            series_title: None,
            season_number: None,
            season_title: None,
            manually_edited: e.manually_edited,
            added_at: e.added_at.to_string(),
            updated_at: e.updated_at.to_string(),
            artwork: ArtworkDto::from_refs(e.artwork),
            series_artwork: ArtworkDto::default(),
        }
    }
}

impl From<EpisodeCard> for EpisodeResponse {
    fn from(card: EpisodeCard) -> Self {
        EpisodeResponse {
            series_id: card.series.map(|s| s.0),
            series_title: card.series_title,
            series_artwork: ArtworkDto::from_refs(card.series_artwork),
            season_number: card.season_number,
            season_title: card.season_title,
            ..EpisodeResponse::from(card.episode)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::catalog::{ArtworkId, ArtworkRef, ArtworkWidth, EpisodeId, SeasonId, SeriesId};
    use domain::metadata::ArtworkKind;
    use jiff::Timestamp;

    fn episode() -> Episode {
        Episode {
            id: EpisodeId("e1".into()),
            season: SeasonId("se1".into()),
            number: 1,
            title: "Pilot".into(),
            overview: None,
            runtime_minutes: None,
            air_date: None,
            manually_edited: false,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
            artwork: vec![ArtworkRef {
                id: ArtworkId("own".into()),
                kind: ArtworkKind::Backdrop,
                widths: vec![ArtworkWidth::new(480, "/art/own/480.jpg")],
            }],
        }
    }

    fn card(series_artwork: Vec<ArtworkRef>) -> EpisodeCard {
        EpisodeCard {
            episode: episode(),
            series: Some(SeriesId("s1".into())),
            series_title: Some("Show ABC".into()),
            series_artwork,
            season_number: Some(1),
            season_title: None,
        }
    }

    #[test]
    fn an_empty_series_artwork_is_omitted_so_a_client_can_fall_back() {
        let value = serde_json::to_value(EpisodeResponse::from(card(Vec::new()))).unwrap();

        assert!(
            value.get("series_artwork").is_none(),
            "an always-present empty object makes the client's null fallback unreachable, \
             so an episode whose series has no poster renders a placeholder instead of its own art"
        );
        assert_eq!(
            value["artwork"]["backdrops"][0]["base"], "/images/own",
            "the episode's own artwork is what the fallback reaches for"
        );
        assert_eq!(value["series_title"], "Show ABC");
    }

    #[test]
    fn a_series_that_has_artwork_still_sends_it() {
        let value = serde_json::to_value(EpisodeResponse::from(card(vec![ArtworkRef {
            id: ArtworkId("sp1".into()),
            kind: ArtworkKind::Poster,
            widths: vec![ArtworkWidth::new(180, "/art/sp1/180.jpg")],
        }])))
        .unwrap();

        assert_eq!(value["series_artwork"]["posters"][0]["base"], "/images/sp1");
    }
}
