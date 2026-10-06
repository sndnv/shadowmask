use serde::Serialize;

use domain::catalog::{Series, SeriesCard};

use crate::dto::common::{ArtworkDto, ContentRatingDto};

#[derive(Debug, Serialize)]
pub struct SeriesResponse {
    pub id: String,
    pub title: String,
    pub year: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub season_count: Option<u16>,
    pub overview: Option<String>,
    pub content_rating: Option<ContentRatingDto>,
    pub manually_edited: bool,
    pub added_at: String,
    pub updated_at: String,
    pub artwork: ArtworkDto,
}

impl From<Series> for SeriesResponse {
    fn from(s: Series) -> Self {
        SeriesResponse {
            id: s.id.0,
            title: s.title,
            year: s.year,
            season_count: None,
            overview: s.overview,
            content_rating: s.content_rating.map(Into::into),
            manually_edited: s.manually_edited,
            added_at: s.added_at.to_string(),
            updated_at: s.updated_at.to_string(),
            artwork: ArtworkDto::from_refs(s.artwork),
        }
    }
}

impl From<SeriesCard> for SeriesResponse {
    fn from(card: SeriesCard) -> Self {
        SeriesResponse { season_count: Some(card.season_count), ..card.series.into() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use contracts::fixture::series;

    #[test]
    fn a_card_carries_its_season_count_and_a_bare_series_leaves_it_out() {
        let card = SeriesCard { series: series("s1"), season_count: 0 };

        let counted = serde_json::to_value(SeriesResponse::from(card)).unwrap();
        let bare = serde_json::to_value(SeriesResponse::from(series("s1"))).unwrap();

        assert_eq!(counted["season_count"], 0);
        assert!(bare.get("season_count").is_none());
    }
}
