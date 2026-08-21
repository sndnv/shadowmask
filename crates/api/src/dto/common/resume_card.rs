use serde::Serialize;

use domain::playback::ResumeCard;

use super::{ArtworkDto, TitleRefDto};

#[derive(Debug, Serialize)]
pub struct ResumeCardDto {
    pub title: TitleRefDto,
    pub display_title: String,
    pub artwork: ArtworkDto,
    pub duration_ms: u64,
    pub progress_percent: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub year: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub series_title: Option<String>,
    #[serde(skip_serializing_if = "ArtworkDto::is_empty")]
    pub series_artwork: ArtworkDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub season_number: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub episode_number: Option<u16>,
}

impl From<ResumeCard> for ResumeCardDto {
    fn from(c: ResumeCard) -> Self {
        ResumeCardDto {
            title: c.title.into(),
            display_title: c.display_title,
            artwork: ArtworkDto::from_refs(c.artwork),
            duration_ms: c.duration_ms,
            progress_percent: c.progress_percent,
            year: c.year,
            series_title: c.series_title,
            series_artwork: ArtworkDto::from_refs(c.series_artwork),
            season_number: c.season_number,
            episode_number: c.episode_number,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::catalog::{ArtworkId, ArtworkRef, ArtworkWidth, EpisodeId, MovieId, TitleId};
    use domain::metadata::ArtworkKind;
    use serde_json::json;

    fn poster(id: &str) -> ArtworkRef {
        ArtworkRef {
            id: ArtworkId(id.into()),
            kind: ArtworkKind::Poster,
            widths: vec![ArtworkWidth::new(180, format!("/art/{id}/180.jpg"))],
        }
    }

    #[test]
    fn serializes_card_with_title_and_artwork() {
        let card = ResumeCard {
            title: TitleId::Movie(MovieId("m1".into())),
            display_title: "Alpha".into(),
            artwork: vec![poster("p1")],
            duration_ms: 1000,
            progress_percent: 42,
            year: Some(2020),
            series_title: None,
            series_artwork: Vec::new(),
            season_number: None,
            episode_number: None,
        };
        assert_eq!(
            serde_json::to_value(ResumeCardDto::from(card)).unwrap(),
            json!({
                "title": {"type": "movie", "id": "m1"},
                "display_title": "Alpha",
                "artwork": {"posters": [{"base": "/images/p1", "widths": [180]}]},
                "duration_ms": 1000,
                "progress_percent": 42,
                "year": 2020,
            })
        );
    }

    #[test]
    fn an_empty_series_artwork_is_omitted_so_a_client_can_fall_back() {
        let card = ResumeCard {
            title: TitleId::Episode(EpisodeId("e1".into())),
            display_title: "Pilot".into(),
            artwork: vec![poster("p1")],
            duration_ms: 1000,
            progress_percent: 5,
            year: None,
            series_title: Some("Show ABC".into()),
            series_artwork: Vec::new(),
            season_number: Some(2),
            episode_number: Some(4),
        };

        let value = serde_json::to_value(ResumeCardDto::from(card)).unwrap();

        assert!(
            value.get("series_artwork").is_none(),
            "an always-present empty object makes the client's null fallback unreachable, \
             so an episode whose series has no poster renders a placeholder instead of its own art"
        );
    }

    #[test]
    fn serializes_episode_card_with_series_context() {
        let card = ResumeCard {
            title: TitleId::Episode(EpisodeId("e1".into())),
            display_title: "Pilot".into(),
            artwork: Vec::new(),
            duration_ms: 1000,
            progress_percent: 5,
            year: None,
            series_title: Some("Show ABC".into()),
            series_artwork: vec![poster("sp1")],
            season_number: Some(2),
            episode_number: Some(4),
        };
        let value = serde_json::to_value(ResumeCardDto::from(card)).unwrap();
        assert_eq!(value["series_title"], "Show ABC");
        assert_eq!(value["season_number"], 2);
        assert_eq!(value["episode_number"], 4);
        assert_eq!(value["series_artwork"]["posters"][0]["base"], "/images/sp1");
        assert!(value.get("year").is_none());
    }
}
