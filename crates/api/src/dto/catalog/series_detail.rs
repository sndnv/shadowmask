use serde::Serialize;

use domain::catalog::SeriesDetail;

use crate::dto::catalog::detail::{
    CreditDto, ExternalIdDto, ExtraDto, GenreDto, RatingDto, StudioDto,
};
use crate::dto::common::{ArtworkDto, ContentRatingDto};

#[derive(Debug, Serialize)]
pub struct SeriesDetailResponse {
    pub id: String,
    pub title: String,
    pub year: Option<u16>,
    pub overview: Option<String>,
    pub content_rating: Option<ContentRatingDto>,
    pub added_at: String,
    pub artwork: ArtworkDto,
    pub genres: Vec<GenreDto>,
    pub credits: Vec<CreditDto>,
    pub studios: Vec<StudioDto>,
    pub ratings: Vec<RatingDto>,
    pub external_ids: Vec<ExternalIdDto>,
    pub extras: Vec<ExtraDto>,
}

impl From<SeriesDetail> for SeriesDetailResponse {
    fn from(d: SeriesDetail) -> Self {
        let series = d.series;
        SeriesDetailResponse {
            id: series.id.0,
            title: series.title,
            year: series.year,
            overview: series.overview,
            content_rating: series.content_rating.map(Into::into),
            added_at: series.added_at.to_string(),
            artwork: ArtworkDto::from_refs(series.artwork),
            genres: d.genres.into_iter().map(Into::into).collect(),
            credits: d.credits.into_iter().map(Into::into).collect(),
            studios: d.studios.into_iter().map(Into::into).collect(),
            ratings: d.ratings.into_iter().map(Into::into).collect(),
            external_ids: d.external_ids.into_iter().map(Into::into).collect(),
            extras: d.extras.into_iter().map(Into::into).collect(),
        }
    }
}
