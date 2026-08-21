use serde::Deserialize;

use domain::catalog::SeriesEdit;

use crate::dto::common::ContentRatingDto;

#[derive(Debug, Deserialize)]
pub struct EditSeriesRequest {
    pub title: String,
    pub year: Option<u16>,
    pub overview: Option<String>,
    pub content_rating: Option<ContentRatingDto>,
}

impl From<EditSeriesRequest> for SeriesEdit {
    fn from(req: EditSeriesRequest) -> Self {
        SeriesEdit {
            title: req.title,
            year: req.year,
            overview: req.overview,
            content_rating: req.content_rating.map(Into::into),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_every_editable_field() {
        let request: EditSeriesRequest = serde_json::from_value(serde_json::json!({
            "title": "The Wire",
            "year": 2002,
            "overview": "Baltimore.",
            "content_rating": {"system": "US-TV", "code": "TV-MA"},
        }))
        .unwrap();
        let edit = SeriesEdit::from(request);
        assert_eq!(edit.title, "The Wire");
        assert_eq!(edit.year, Some(2002));
        assert_eq!(edit.overview.as_deref(), Some("Baltimore."));
        let rating = edit.content_rating.unwrap();
        assert_eq!(rating.system, "US-TV");
        assert_eq!(rating.code, "TV-MA");
    }

    #[test]
    fn clears_every_optional_field_when_omitted() {
        let request: EditSeriesRequest =
            serde_json::from_value(serde_json::json!({"title": "Untitled"})).unwrap();
        let edit = SeriesEdit::from(request);
        assert_eq!(edit.title, "Untitled");
        assert!(edit.year.is_none());
        assert!(edit.overview.is_none());
        assert!(edit.content_rating.is_none());
    }

    #[test]
    fn rejects_a_missing_title() {
        let result: Result<EditSeriesRequest, _> =
            serde_json::from_value(serde_json::json!({"year": 2002}));
        assert!(result.is_err());
    }
}
