use axum::body::Bytes;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use tracing::debug;

use domain::error::LibraryError;
use domain::library::LibraryId;
use domain::user::{Principal, Role, UserId};

use crate::dto::webhook::{TokenQuery, WebhookPayload};
use crate::error::ApiError;
use domain::service::LibraryService;

use crate::state::{AppServices, WebhookClient, WebhookState};

enum AuthorizationOutcome<'a> {
    Denied,
    OutOfScope,
    Authorized(&'a WebhookClient),
}

fn authorize<'a>(
    clients: &'a [WebhookClient],
    token: Option<&str>,
    library: &str,
) -> AuthorizationOutcome<'a> {
    let Some(token) = token.filter(|t| !t.is_empty()) else {
        return AuthorizationOutcome::Denied;
    };
    let Some(client) = clients.iter().find(|client| client.secret == token) else {
        return AuthorizationOutcome::Denied;
    };
    if client.libraries.iter().any(|scoped| scoped == library) {
        AuthorizationOutcome::Authorized(client)
    } else {
        AuthorizationOutcome::OutOfScope
    }
}

fn bearer(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(header::AUTHORIZATION)?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
}

fn is_test_event(body: &[u8]) -> bool {
    serde_json::from_slice::<WebhookPayload>(body)
        .ok()
        .and_then(|payload| payload.event_type)
        .is_some_and(|event| event.eq_ignore_ascii_case("test"))
}

pub async fn scan<S: AppServices>(
    State(state): State<WebhookState<S>>,
    Path(id): Path<String>,
    Query(query): Query<TokenQuery>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let token = query.token.as_deref().or_else(|| bearer(&headers));
    let client = match authorize(&state.clients, token, &id) {
        AuthorizationOutcome::Denied => {
            return ApiError::unauthorized("invalid webhook credentials").into_response();
        }
        AuthorizationOutcome::OutOfScope => {
            return ApiError::forbidden("client not permitted for this library").into_response();
        }
        AuthorizationOutcome::Authorized(client) => client,
    };
    if is_test_event(&body) {
        debug!(
            "Webhook client [{}] sent a test event for library [{id}]",
            client.name
        );
        return StatusCode::OK.into_response();
    }
    let principal = Principal {
        user: UserId(client.name.clone()),
        role: Role::Automation,
    };
    let library = LibraryId(id);
    match state
        .services
        .library()
        .trigger_scan(&principal, &library)
        .await
    {
        Ok(()) | Err(LibraryError::ScanInProgress) => {
            debug!(
                "Webhook client [{}] triggered scan for library [{}]",
                principal.user.0, library.0
            );
            StatusCode::ACCEPTED.into_response()
        }
        Err(err) => ApiError::from(err).into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clients() -> Vec<WebhookClient> {
        vec![WebhookClient {
            name: "sonarr".into(),
            secret: "sec".into(),
            libraries: vec!["lib1".into()],
        }]
    }

    #[test]
    fn authorize_matrix() {
        let clients = clients();
        assert!(matches!(
            authorize(&clients, Some("sec"), "lib1"),
            AuthorizationOutcome::Authorized(_)
        ));
        assert!(matches!(
            authorize(&clients, Some("sec"), "lib2"),
            AuthorizationOutcome::OutOfScope
        ));
        assert!(matches!(
            authorize(&clients, Some("nope"), "lib1"),
            AuthorizationOutcome::Denied
        ));
        assert!(matches!(
            authorize(&clients, None, "lib1"),
            AuthorizationOutcome::Denied
        ));
        assert!(matches!(
            authorize(&clients, Some(""), "lib1"),
            AuthorizationOutcome::Denied
        ));
    }

    #[test]
    fn bearer_strips_prefix() {
        let mut headers = HeaderMap::new();
        assert_eq!(bearer(&headers), None);
        headers.insert(header::AUTHORIZATION, "Basic zzz".parse().unwrap());
        assert_eq!(bearer(&headers), None);
        headers.insert(header::AUTHORIZATION, "Bearer tok".parse().unwrap());
        assert_eq!(bearer(&headers), Some("tok"));
    }

    #[test]
    fn is_test_event_detection() {
        assert!(is_test_event(br#"{"eventType":"Test"}"#));
        assert!(is_test_event(br#"{"eventType":"test"}"#));
        assert!(!is_test_event(br#"{"eventType":"Download"}"#));
        assert!(!is_test_event(br#"{"other":"x"}"#));
        assert!(!is_test_event(b""));
        assert!(!is_test_event(b"not json"));
    }
}
