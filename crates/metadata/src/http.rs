use std::time::Duration;

use domain::error::MetadataError;
use reqwest::header::HeaderMap;
use serde::de::DeserializeOwned;

use crate::rate_limiter::RateLimiter;

const DEFAULT_RETRY_AFTER: Duration = Duration::from_secs(5);
const BODY_EXCERPT_CHARS: usize = 200;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);

pub(crate) fn client() -> reqwest::Client {
    client_with(REQUEST_TIMEOUT)
}

fn client_with(timeout: Duration) -> reqwest::Client {
    reqwest::Client::builder()
        .connect_timeout(CONNECT_TIMEOUT)
        .timeout(timeout)
        .build()
        .expect("the HTTP client builds with the bundled TLS backend")
}

pub(crate) async fn send_ok(
    limiter: &RateLimiter,
    request: reqwest::RequestBuilder,
) -> Result<reqwest::Response, String> {
    limiter.acquire().await;
    let response = request.send().await.map_err(describe)?;
    let status = response.status();
    if status.is_success() {
        return Ok(response);
    }
    if status.as_u16() == 429 {
        let retry_after = parse_retry_after(response.headers()).unwrap_or(DEFAULT_RETRY_AFTER);
        limiter.penalize(retry_after).await;
    }
    let body = response.text().await.unwrap_or_default();
    Err(status_message(status, &body))
}

pub(crate) async fn get_json<T: DeserializeOwned>(
    limiter: &RateLimiter,
    request: reqwest::RequestBuilder,
) -> Result<T, MetadataError> {
    let response = send_ok(limiter, request).await.map_err(MetadataError::Backend)?;
    response.json::<T>().await.map_err(|e| MetadataError::Parse(e.without_url().to_string()))
}

fn describe(error: reqwest::Error) -> String {
    if error.is_timeout() {
        "the provider did not answer in time".to_owned()
    } else {
        error.without_url().to_string()
    }
}

fn status_message(status: reqwest::StatusCode, body: &str) -> String {
    let excerpt: String = body
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(BODY_EXCERPT_CHARS)
        .collect();
    if excerpt.is_empty() {
        format!("http status {status}")
    } else {
        format!("http status {status}: {excerpt}")
    }
}

fn parse_retry_after(headers: &HeaderMap) -> Option<Duration> {
    let value = headers.get(reqwest::header::RETRY_AFTER)?.to_str().ok()?;
    let secs: u64 = value.trim().parse().ok()?;
    Some(Duration::from_secs(secs))
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::header::{HeaderValue, RETRY_AFTER};

    fn headers(value: &str) -> HeaderMap {
        let mut map = HeaderMap::new();
        map.insert(RETRY_AFTER, HeaderValue::from_str(value).unwrap());
        map
    }

    #[test]
    fn parses_integer_seconds() {
        assert_eq!(parse_retry_after(&headers("12")), Some(Duration::from_secs(12)));
    }

    #[test]
    fn missing_header_is_none() {
        assert_eq!(parse_retry_after(&HeaderMap::new()), None);
    }

    #[test]
    fn non_numeric_header_is_none() {
        assert_eq!(parse_retry_after(&headers("Wed, 21 Oct 2015 07:28:00 GMT")), None);
    }

    #[test]
    fn a_status_message_carries_the_body() {
        let message = status_message(
            reqwest::StatusCode::NOT_ACCEPTABLE,
            r#"{"message":"You cannot consume this service"}"#,
        );
        assert_eq!(
            message,
            r#"http status 406 Not Acceptable: {"message":"You cannot consume this service"}"#
        );
    }

    #[test]
    fn a_status_message_keeps_the_body_on_one_bounded_line() {
        let body = format!("line one\n\n  line\ttwo\r\n{}", "x".repeat(500));
        let message = status_message(reqwest::StatusCode::BAD_GATEWAY, &body);
        let excerpt = message.strip_prefix("http status 502 Bad Gateway: ").unwrap();
        assert!(excerpt.starts_with("line one line two x"));
        assert_eq!(excerpt.chars().count(), BODY_EXCERPT_CHARS);
        assert!(!message.contains(['\n', '\r', '\t']));
    }

    #[tokio::test]
    async fn a_provider_that_does_not_answer_in_time_fails() {
        use wiremock::{Mock, MockServer, ResponseTemplate};
        let server = MockServer::start().await;
        Mock::given(wiremock::matchers::any())
            .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(5)))
            .mount(&server)
            .await;
        let limiter = RateLimiter::new(Duration::ZERO);

        let message = send_ok(&limiter, client_with(Duration::from_millis(100)).get(server.uri()))
            .await
            .unwrap_err();

        assert_eq!(message, "the provider did not answer in time");
    }

    #[test]
    fn the_shared_client_builds() {
        let _ = client();
    }

    #[test]
    fn a_status_message_without_a_body_is_the_status_alone() {
        let message = status_message(reqwest::StatusCode::INTERNAL_SERVER_ERROR, " \n ");
        assert_eq!(message, "http status 500 Internal Server Error");
    }
}
