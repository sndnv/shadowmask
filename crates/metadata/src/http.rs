use std::time::Duration;

use domain::error::MetadataError;
use reqwest::header::HeaderMap;
use serde::de::DeserializeOwned;

use crate::rate_limiter::RateLimiter;

const DEFAULT_RETRY_AFTER: Duration = Duration::from_secs(5);

pub(crate) async fn send_ok(
    limiter: &RateLimiter,
    request: reqwest::RequestBuilder,
) -> Result<reqwest::Response, String> {
    limiter.acquire().await;
    let response = request.send().await.map_err(|e| e.to_string())?;
    let status = response.status();
    if status.as_u16() == 429 {
        let retry_after = parse_retry_after(response.headers()).unwrap_or(DEFAULT_RETRY_AFTER);
        limiter.penalize(retry_after).await;
        return Err(format!("http status {status}"));
    }
    if !status.is_success() {
        return Err(format!("http status {status}"));
    }
    Ok(response)
}

pub(crate) async fn get_json<T: DeserializeOwned>(
    limiter: &RateLimiter,
    request: reqwest::RequestBuilder,
) -> Result<T, MetadataError> {
    let response = send_ok(limiter, request).await.map_err(MetadataError::Backend)?;
    response.json::<T>().await.map_err(|e| MetadataError::Parse(e.to_string()))
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
}
