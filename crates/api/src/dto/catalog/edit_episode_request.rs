use jiff::Timestamp;
use jiff::civil::Date;
use jiff::tz::TimeZone;
use serde::Deserialize;

use domain::catalog::EpisodeEdit;

use crate::error::ApiError;

#[derive(Debug, Deserialize)]
pub struct EditEpisodeRequest {
    pub title: String,
    pub overview: Option<String>,
    pub runtime_minutes: Option<u32>,
    pub air_date: Option<String>,
}

fn parse_air_date(value: &str) -> Option<Timestamp> {
    if let Ok(stamp) = value.parse::<Timestamp>() {
        return Some(stamp);
    }
    let date: Date = value.parse().ok()?;
    date.to_zoned(TimeZone::UTC)
        .ok()
        .map(|zoned| zoned.timestamp())
}

impl TryFrom<EditEpisodeRequest> for EpisodeEdit {
    type Error = ApiError;

    fn try_from(req: EditEpisodeRequest) -> Result<Self, Self::Error> {
        let air_date = match req.air_date.as_deref() {
            Some(raw) => Some(
                parse_air_date(raw)
                    .ok_or_else(|| ApiError::bad_request(format!("unparseable air_date: {raw}")))?,
            ),
            None => None,
        };
        Ok(EpisodeEdit {
            title: req.title,
            overview: req.overview,
            runtime_minutes: req.runtime_minutes,
            air_date,
        })
    }
}

#[cfg(test)]
mod tests {
    use axum::response::IntoResponse;

    use super::*;

    fn request(value: serde_json::Value) -> EditEpisodeRequest {
        serde_json::from_value(value).unwrap()
    }

    fn edit(value: serde_json::Value) -> EpisodeEdit {
        let Ok(edit) = EpisodeEdit::try_from(request(value)) else {
            panic!("expected the request to convert");
        };
        edit
    }

    #[test]
    fn accepts_a_plain_calendar_date_as_midnight_utc() {
        let edit = edit(serde_json::json!({
            "title": "Pilot",
            "air_date": "2010-10-01",
        }));
        assert_eq!(edit.title, "Pilot");
        assert_eq!(edit.air_date.unwrap().to_string(), "2010-10-01T00:00:00Z");
    }

    #[test]
    fn accepts_the_timestamp_it_serves_back() {
        let edit = edit(serde_json::json!({
            "title": "Pilot",
            "air_date": "2010-10-01T00:00:00Z",
        }));
        assert_eq!(edit.air_date.unwrap().to_string(), "2010-10-01T00:00:00Z");
    }

    #[test]
    fn maps_the_remaining_fields_and_clears_omitted_ones() {
        let edit = edit(serde_json::json!({
            "title": "Pilot",
            "overview": "It begins.",
            "runtime_minutes": 42,
        }));
        assert_eq!(edit.overview.as_deref(), Some("It begins."));
        assert_eq!(edit.runtime_minutes, Some(42));
        assert!(edit.air_date.is_none());
    }

    #[test]
    fn rejects_an_unparseable_air_date() {
        let Err(err) = EpisodeEdit::try_from(request(serde_json::json!({
            "title": "Pilot",
            "air_date": "last thursday",
        }))) else {
            panic!("expected the air_date to be rejected");
        };
        assert_eq!(
            err.into_response().status(),
            axum::http::StatusCode::BAD_REQUEST
        );
    }

    #[test]
    fn rejects_a_missing_title() {
        let result: Result<EditEpisodeRequest, _> =
            serde_json::from_value(serde_json::json!({"overview": "no title"}));
        assert!(result.is_err());
    }
}
