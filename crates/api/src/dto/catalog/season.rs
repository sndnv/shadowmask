use serde::Serialize;

use domain::catalog::Season;

use crate::dto::common::ArtworkDto;

#[derive(Debug, Serialize)]
pub struct SeasonResponse {
    pub id: String,
    pub series_id: String,
    pub number: u16,
    pub title: Option<String>,
    pub overview: Option<String>,
    pub artwork: ArtworkDto,
}

impl From<Season> for SeasonResponse {
    fn from(s: Season) -> Self {
        SeasonResponse {
            id: s.id.0,
            series_id: s.series.0,
            number: s.number,
            title: s.title,
            overview: s.overview,
            artwork: ArtworkDto::from_refs(s.artwork),
        }
    }
}
