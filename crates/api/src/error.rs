use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;

use domain::error::{
    AuthError, CatalogError, DiscoveryError, LibraryError, SessionError, StreamError,
    StreamTokenError, UserError,
};
use domain::session::PlaybackSession;

use crate::dto::session::PlaybackSessionResponse;

pub type ApiResult<T> = Result<T, ApiError>;

pub struct ApiError {
    status: StatusCode,
    code: &'static str,
    message: String,
    active: Option<Vec<PlaybackSession>>,
}

impl ApiError {
    fn new(status: StatusCode, code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status,
            code,
            message: message.into(),
            active: None,
        }
    }

    pub fn unauthorized(message: impl Into<String>) -> Self {
        Self::new(StatusCode::UNAUTHORIZED, "unauthorized", message)
    }

    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::new(StatusCode::FORBIDDEN, "forbidden", message)
    }

    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, "bad_request", message)
    }

    fn internal() -> Self {
        Self::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal",
            "internal server error",
        )
    }
}

#[derive(Serialize)]
struct ErrorDetail {
    code: &'static str,
    message: String,
}

#[derive(Serialize)]
struct ErrorEnvelope {
    error: ErrorDetail,
    #[serde(skip_serializing_if = "Option::is_none")]
    active: Option<Vec<PlaybackSessionResponse>>,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = ErrorEnvelope {
            error: ErrorDetail {
                code: self.code,
                message: self.message,
            },
            active: self
                .active
                .map(|v| v.into_iter().map(PlaybackSessionResponse::from).collect()),
        };
        (self.status, Json(body)).into_response()
    }
}

impl From<AuthError> for ApiError {
    fn from(err: AuthError) -> Self {
        let msg = err.to_string();
        match err {
            AuthError::InvalidCredentials => {
                ApiError::new(StatusCode::UNAUTHORIZED, "invalid_credentials", msg)
            }
            AuthError::TokenExpired => {
                ApiError::new(StatusCode::UNAUTHORIZED, "token_expired", msg)
            }
            AuthError::InvalidToken => {
                ApiError::new(StatusCode::UNAUTHORIZED, "invalid_token", msg)
            }
            AuthError::UnknownLinkCode => {
                ApiError::new(StatusCode::NOT_FOUND, "unknown_link_code", msg)
            }
            AuthError::Repository(_) => ApiError::internal(),
        }
    }
}

impl From<CatalogError> for ApiError {
    fn from(err: CatalogError) -> Self {
        let msg = err.to_string();
        match err {
            CatalogError::NotFound => ApiError::new(StatusCode::NOT_FOUND, "not_found", msg),
            CatalogError::Repository(_) => ApiError::internal(),
        }
    }
}

impl From<SessionError> for ApiError {
    fn from(err: SessionError) -> Self {
        let msg = err.to_string();
        match err {
            SessionError::NotFound => {
                ApiError::new(StatusCode::NOT_FOUND, "session_not_found", msg)
            }
            SessionError::VersionNotFound => {
                ApiError::new(StatusCode::NOT_FOUND, "version_not_found", msg)
            }
            SessionError::NegotiationFailed => {
                ApiError::new(StatusCode::CONFLICT, "negotiation_failed", msg)
            }
            SessionError::ConcurrentLimit { active } => {
                let mut e = ApiError::new(StatusCode::CONFLICT, "concurrent_limit", msg);
                e.active = Some(active);
                e
            }
            SessionError::Repository(_) => ApiError::internal(),
        }
    }
}

impl From<LibraryError> for ApiError {
    fn from(err: LibraryError) -> Self {
        let msg = err.to_string();
        match err {
            LibraryError::NotFound => ApiError::new(StatusCode::NOT_FOUND, "not_found", msg),
            LibraryError::ScanInProgress => {
                ApiError::new(StatusCode::CONFLICT, "scan_in_progress", msg)
            }
            LibraryError::Walk(_) | LibraryError::Repository(_) => ApiError::internal(),
        }
    }
}

impl From<UserError> for ApiError {
    fn from(err: UserError) -> Self {
        let msg = err.to_string();
        match err {
            UserError::NotFound => ApiError::new(StatusCode::NOT_FOUND, "not_found", msg),
            UserError::UsernameTaken => ApiError::new(StatusCode::CONFLICT, "username_taken", msg),
            UserError::AccessDenied => ApiError::new(StatusCode::FORBIDDEN, "access_denied", msg),
            UserError::Repository(_) => ApiError::internal(),
        }
    }
}

impl From<DiscoveryError> for ApiError {
    fn from(err: DiscoveryError) -> Self {
        match err {
            DiscoveryError::Repository(_) => ApiError::internal(),
        }
    }
}

impl From<StreamTokenError> for ApiError {
    fn from(err: StreamTokenError) -> Self {
        let msg = err.to_string();
        match err {
            StreamTokenError::Expired => {
                ApiError::new(StatusCode::FORBIDDEN, "stream_token_expired", msg)
            }
            StreamTokenError::Invalid => {
                ApiError::new(StatusCode::FORBIDDEN, "invalid_stream_token", msg)
            }
            StreamTokenError::Create(_) => ApiError::internal(),
        }
    }
}

impl From<StreamError> for ApiError {
    fn from(err: StreamError) -> Self {
        let msg = err.to_string();
        match err {
            StreamError::NotLive => ApiError::new(StatusCode::FORBIDDEN, "stream_not_live", msg),
            StreamError::Invalid => {
                ApiError::new(StatusCode::BAD_REQUEST, "bad_stream_request", msg)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::error::{RepositoryError, WalkError};
    use domain::session::{DeliveryMode, PlaybackState, SelectedTracks, SessionId};
    use domain::user::UserId;
    use jiff::Timestamp;

    fn repo() -> RepositoryError {
        RepositoryError::Backend("boom".into())
    }

    fn session() -> PlaybackSession {
        let now = Timestamp::now();
        PlaybackSession {
            id: SessionId("s1".into()),
            user: UserId("u1".into()),
            device: None,
            version: domain::catalog::VersionId("v1".into()),
            mode: DeliveryMode::Direct,
            position_ms: 0,
            state: PlaybackState::Playing,
            selected: SelectedTracks {
                audio_track: None,
                subtitle_track: None,
                subtitle_delivery: None,
            },
            started_at: now,
            last_heartbeat_at: now,
        }
    }

    #[test]
    fn constructors_have_expected_status_and_code() {
        assert_eq!(ApiError::unauthorized("x").status, StatusCode::UNAUTHORIZED);
        assert_eq!(ApiError::unauthorized("x").code, "unauthorized");
        assert_eq!(ApiError::forbidden("x").status, StatusCode::FORBIDDEN);
        assert_eq!(ApiError::forbidden("x").code, "forbidden");
        assert_eq!(ApiError::bad_request("x").status, StatusCode::BAD_REQUEST);
        assert_eq!(ApiError::bad_request("x").code, "bad_request");
    }

    #[test]
    fn auth_error_mappings() {
        assert_eq!(
            ApiError::from(AuthError::InvalidCredentials).code,
            "invalid_credentials"
        );
        assert_eq!(
            ApiError::from(AuthError::TokenExpired).code,
            "token_expired"
        );
        assert_eq!(
            ApiError::from(AuthError::InvalidToken).code,
            "invalid_token"
        );
        let nf = ApiError::from(AuthError::UnknownLinkCode);
        assert_eq!(nf.status, StatusCode::NOT_FOUND);
        assert_eq!(nf.code, "unknown_link_code");
        let r = ApiError::from(AuthError::Repository(repo()));
        assert_eq!(r.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(r.code, "internal");
    }

    #[test]
    fn catalog_error_mappings() {
        let nf = ApiError::from(CatalogError::NotFound);
        assert_eq!(nf.status, StatusCode::NOT_FOUND);
        assert_eq!(nf.code, "not_found");
        assert_eq!(
            ApiError::from(CatalogError::Repository(repo())).status,
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }

    #[test]
    fn session_error_mappings() {
        assert_eq!(
            ApiError::from(SessionError::NotFound).code,
            "session_not_found"
        );
        assert_eq!(
            ApiError::from(SessionError::VersionNotFound).code,
            "version_not_found"
        );
        let neg = ApiError::from(SessionError::NegotiationFailed);
        assert_eq!(neg.status, StatusCode::CONFLICT);
        assert_eq!(neg.code, "negotiation_failed");
        let limit = ApiError::from(SessionError::ConcurrentLimit {
            active: vec![session()],
        });
        assert_eq!(limit.status, StatusCode::CONFLICT);
        assert_eq!(limit.code, "concurrent_limit");
        assert!(limit.active.is_some());
        assert_eq!(
            ApiError::from(SessionError::Repository(repo())).status,
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }

    #[test]
    fn library_error_mappings() {
        assert_eq!(ApiError::from(LibraryError::NotFound).code, "not_found");
        let conflict = ApiError::from(LibraryError::ScanInProgress);
        assert_eq!(conflict.status, StatusCode::CONFLICT);
        assert_eq!(conflict.code, "scan_in_progress");
        assert_eq!(
            ApiError::from(LibraryError::Repository(repo())).status,
            StatusCode::INTERNAL_SERVER_ERROR
        );
        assert_eq!(
            ApiError::from(LibraryError::Walk(WalkError::RootNotFound("/x".into()))).status,
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }

    #[test]
    fn user_error_mappings() {
        assert_eq!(ApiError::from(UserError::NotFound).code, "not_found");
        let taken = ApiError::from(UserError::UsernameTaken);
        assert_eq!(taken.status, StatusCode::CONFLICT);
        assert_eq!(taken.code, "username_taken");
        let denied = ApiError::from(UserError::AccessDenied);
        assert_eq!(denied.status, StatusCode::FORBIDDEN);
        assert_eq!(denied.code, "access_denied");
        assert_eq!(
            ApiError::from(UserError::Repository(repo())).status,
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }

    #[test]
    fn discovery_error_maps_to_internal() {
        assert_eq!(
            ApiError::from(DiscoveryError::Repository(repo())).status,
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }

    #[test]
    fn stream_token_error_mappings() {
        let expired = ApiError::from(StreamTokenError::Expired);
        assert_eq!(expired.status, StatusCode::FORBIDDEN);
        assert_eq!(expired.code, "stream_token_expired");
        let invalid = ApiError::from(StreamTokenError::Invalid);
        assert_eq!(invalid.status, StatusCode::FORBIDDEN);
        assert_eq!(invalid.code, "invalid_stream_token");
        let create = ApiError::from(StreamTokenError::Create("boom".into()));
        assert_eq!(create.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(create.code, "internal");
    }

    #[test]
    fn stream_error_mappings() {
        let not_live = ApiError::from(StreamError::NotLive);
        assert_eq!(not_live.status, StatusCode::FORBIDDEN);
        assert_eq!(not_live.code, "stream_not_live");
        let invalid = ApiError::from(StreamError::Invalid);
        assert_eq!(invalid.status, StatusCode::BAD_REQUEST);
        assert_eq!(invalid.code, "bad_stream_request");
    }

    #[test]
    fn into_response_plain_and_with_active() {
        let plain = ApiError::bad_request("nope").into_response();
        assert_eq!(plain.status(), StatusCode::BAD_REQUEST);

        let with_active = ApiError::from(SessionError::ConcurrentLimit {
            active: vec![session()],
        })
        .into_response();
        assert_eq!(with_active.status(), StatusCode::CONFLICT);
    }
}
