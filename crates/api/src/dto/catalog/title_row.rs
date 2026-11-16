use serde::Serialize;

use domain::catalog::TitleRow;

#[derive(Debug, Serialize)]
pub struct TitleRowResponse {
    pub kind: &'static str,
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub series_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub season_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub series_title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub season_number: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub episode_number: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub year: Option<u16>,
    pub files: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub episodes: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seasons: Option<u32>,
    pub available: bool,
    pub readable: bool,
    pub added_at: String,
}

impl From<TitleRow> for TitleRowResponse {
    fn from(row: TitleRow) -> Self {
        TitleRowResponse {
            kind: row.kind.as_str(),
            id: row.id,
            series_id: row.series.map(|series| series.0),
            season_id: row.season.map(|season| season.0),
            title: row.title,
            series_title: row.series_title,
            season_number: row.season_number,
            episode_number: row.episode_number,
            year: row.year,
            files: row.files,
            episodes: row.episodes,
            seasons: row.seasons,
            available: row.available,
            readable: row.readable,
            added_at: row.added_at.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::catalog::{SeasonId, SeriesId, TitleRowKind};
    use jiff::Timestamp;

    #[test]
    fn an_episode_row_carries_its_parents_and_leaves_out_what_it_lacks() {
        let value = serde_json::to_value(TitleRowResponse::from(TitleRow {
            kind: TitleRowKind::Episode,
            id: "e1".into(),
            series: Some(SeriesId("s1".into())),
            season: Some(SeasonId("se1".into())),
            title: Some("Pilot".into()),
            series_title: Some("Neon Harbor".into()),
            season_number: Some(1),
            episode_number: Some(2),
            year: None,
            files: 1,
            episodes: None,
            seasons: None,
            available: false,
            readable: true,
            added_at: Timestamp::UNIX_EPOCH,
        }))
        .unwrap();
        assert_eq!(
            value,
            serde_json::json!({
                "kind": "episode",
                "id": "e1",
                "series_id": "s1",
                "season_id": "se1",
                "title": "Pilot",
                "series_title": "Neon Harbor",
                "season_number": 1,
                "episode_number": 2,
                "files": 1,
                "available": false,
                "readable": true,
                "added_at": "1970-01-01T00:00:00Z"
            })
        );
    }
}
