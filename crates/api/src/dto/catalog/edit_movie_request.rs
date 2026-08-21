use serde::Deserialize;

use domain::catalog::MovieEdit;

use crate::dto::common::ContentRatingDto;

#[derive(Debug, Deserialize)]
pub struct EditMovieRequest {
    pub title: String,
    pub year: Option<u16>,
    pub overview: Option<String>,
    pub runtime_minutes: Option<u32>,
    pub content_rating: Option<ContentRatingDto>,
}

impl From<EditMovieRequest> for MovieEdit {
    fn from(req: EditMovieRequest) -> Self {
        MovieEdit {
            title: req.title,
            year: req.year,
            overview: req.overview,
            runtime_minutes: req.runtime_minutes,
            content_rating: req.content_rating.map(Into::into),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_every_editable_field() {
        let request: EditMovieRequest = serde_json::from_value(serde_json::json!({
            "title": "The Matrix",
            "year": 1999,
            "overview": "Neo learns the truth.",
            "runtime_minutes": 136,
            "content_rating": {"system": "MPAA", "code": "R"},
        }))
        .unwrap();
        let edit = MovieEdit::from(request);
        assert_eq!(edit.title, "The Matrix");
        assert_eq!(edit.year, Some(1999));
        assert_eq!(edit.overview.as_deref(), Some("Neo learns the truth."));
        assert_eq!(edit.runtime_minutes, Some(136));
        let rating = edit.content_rating.unwrap();
        assert_eq!(rating.system, "MPAA");
        assert_eq!(rating.code, "R");
    }

    #[test]
    fn clears_every_optional_field_when_omitted() {
        let request: EditMovieRequest =
            serde_json::from_value(serde_json::json!({"title": "Untitled"})).unwrap();
        let edit = MovieEdit::from(request);
        assert_eq!(edit.title, "Untitled");
        assert!(edit.year.is_none());
        assert!(edit.overview.is_none());
        assert!(edit.runtime_minutes.is_none());
        assert!(edit.content_rating.is_none());
    }

    #[test]
    fn rejects_a_missing_title() {
        let result: Result<EditMovieRequest, _> =
            serde_json::from_value(serde_json::json!({"year": 1999}));
        assert!(result.is_err());
    }
}
