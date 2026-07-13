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
}

impl From<ResumeCard> for ResumeCardDto {
    fn from(c: ResumeCard) -> Self {
        ResumeCardDto {
            title: c.title.into(),
            display_title: c.display_title,
            artwork: ArtworkDto::from_refs(c.artwork),
            duration_ms: c.duration_ms,
            progress_percent: c.progress_percent,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::catalog::{ArtworkId, ArtworkRef, MovieId, TitleId};
    use domain::metadata::ArtworkKind;
    use serde_json::json;

    #[test]
    fn serializes_card_with_title_and_artwork() {
        let card = ResumeCard {
            title: TitleId::Movie(MovieId("m1".into())),
            display_title: "Alpha".into(),
            artwork: vec![ArtworkRef {
                id: ArtworkId("p1".into()),
                kind: ArtworkKind::Poster,
                widths: vec![180],
            }],
            duration_ms: 1000,
            progress_percent: 42,
        };
        assert_eq!(
            serde_json::to_value(ResumeCardDto::from(card)).unwrap(),
            json!({
                "title": {"type": "movie", "id": "m1"},
                "display_title": "Alpha",
                "artwork": {"poster": {"base": "/images/p1", "widths": [180]}},
                "duration_ms": 1000,
                "progress_percent": 42,
            })
        );
    }
}
