use domain::common::LanguageCode;
use domain::error::SubtitleError;
use domain::media::{
    FetchedSubtitle, SubtitleCandidate, SubtitleFormat, SubtitleProvider, SubtitleQuery,
};
use serde::Deserialize;
use serde::de::DeserializeOwned;

use crate::http::send_ok;

const DEFAULT_BASE_URL: &str = "https://api.opensubtitles.com/api/v1";
const USER_AGENT: &str = "shadowmask/0.1";

pub struct OpenSubtitlesClient {
    client: reqwest::Client,
    base_url: String,
    api_key: String,
}

impl OpenSubtitlesClient {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self::with_base_url(api_key, DEFAULT_BASE_URL)
    }

    pub fn with_base_url(api_key: impl Into<String>, base_url: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into(),
            api_key: api_key.into(),
        }
    }

    fn get(&self, url: &str) -> reqwest::RequestBuilder {
        self.client
            .get(url)
            .header("Api-Key", &self.api_key)
            .header("User-Agent", USER_AGENT)
    }

    fn post(&self, url: &str) -> reqwest::RequestBuilder {
        self.client
            .post(url)
            .header("Api-Key", &self.api_key)
            .header("User-Agent", USER_AGENT)
    }
}

fn format_from_name(name: Option<&str>) -> SubtitleFormat {
    name.and_then(|n| n.rsplit('.').next())
        .and_then(SubtitleFormat::from_extension)
        .unwrap_or(SubtitleFormat::Srt)
}

fn join_languages(languages: &[LanguageCode]) -> String {
    languages
        .iter()
        .map(|language| language.0.as_str())
        .collect::<Vec<_>>()
        .join(",")
}

async fn json<T: DeserializeOwned>(request: reqwest::RequestBuilder) -> Result<T, SubtitleError> {
    let response = send_ok(request).await.map_err(SubtitleError::Backend)?;
    response
        .json::<T>()
        .await
        .map_err(|e| SubtitleError::Parse(e.to_string()))
}

impl SubtitleProvider for OpenSubtitlesClient {
    async fn search(&self, query: &SubtitleQuery) -> Result<Vec<SubtitleCandidate>, SubtitleError> {
        let url = format!("{}/subtitles", self.base_url);
        let mut params: Vec<(&str, String)> = Vec::new();
        if let Some(imdb) = &query.imdb_id {
            params.push(("imdb_id", imdb.clone()));
        }
        if let Some(text) = &query.query {
            params.push(("query", text.clone()));
        }
        if !query.languages.is_empty() {
            params.push(("languages", join_languages(&query.languages)));
        }
        if let Some(season) = query.season {
            params.push(("season_number", season.to_string()));
        }
        if let Some(episode) = query.episode {
            params.push(("episode_number", episode.to_string()));
        }
        let response: RawSearchResponse = json(self.get(&url).query(&params)).await?;
        let candidates: Vec<SubtitleCandidate> = response
            .data
            .into_iter()
            .flat_map(|sub| {
                let language = sub.attributes.language.map(LanguageCode);
                let release = sub.attributes.release;
                sub.attributes
                    .files
                    .into_iter()
                    .map(move |file| SubtitleCandidate {
                        file_id: file.file_id.to_string(),
                        language: language.clone(),
                        format: format_from_name(file.file_name.as_deref()),
                        release_name: release.clone(),
                    })
            })
            .collect();
        if candidates.is_empty() {
            return Err(SubtitleError::NotFound);
        }
        Ok(candidates)
    }

    async fn download(&self, file_id: &str) -> Result<FetchedSubtitle, SubtitleError> {
        let url = format!("{}/download", self.base_url);
        let body = serde_json::json!({ "file_id": file_id });
        let download: RawDownload = json(self.post(&url).json(&body)).await?;
        let response = send_ok(self.client.get(&download.link))
            .await
            .map_err(SubtitleError::Backend)?;
        let content = response
            .text()
            .await
            .map_err(|e| SubtitleError::Parse(e.to_string()))?;
        Ok(FetchedSubtitle {
            content,
            format: format_from_name(download.file_name.as_deref()),
        })
    }
}

#[derive(Deserialize)]
struct RawSearchResponse {
    #[serde(default)]
    data: Vec<RawSub>,
}

#[derive(Deserialize)]
struct RawSub {
    attributes: RawAttributes,
}

#[derive(Deserialize)]
struct RawAttributes {
    #[serde(default)]
    language: Option<String>,
    #[serde(default)]
    release: Option<String>,
    #[serde(default)]
    files: Vec<RawFile>,
}

#[derive(Deserialize)]
struct RawFile {
    file_id: i64,
    #[serde(default)]
    file_name: Option<String>,
}

#[derive(Deserialize)]
struct RawDownload {
    link: String,
    #[serde(default)]
    file_name: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn query() -> SubtitleQuery {
        SubtitleQuery {
            imdb_id: Some("tt0133093".to_owned()),
            query: Some("the matrix".to_owned()),
            languages: vec![LanguageCode("en".to_owned())],
            season: Some(1),
            episode: Some(2),
        }
    }

    async fn serve_get(body: ResponseTemplate) -> MockServer {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(body)
            .mount(&server)
            .await;
        server
    }

    #[tokio::test]
    async fn search_returns_candidates() {
        let server = serve_get(ResponseTemplate::new(200).set_body_json(json!({
            "data": [
                {"attributes": {
                    "language": "en",
                    "release": "The.Matrix.1999.BluRay",
                    "files": [
                        {"file_id": 42, "file_name": "matrix.vtt"},
                        {"file_id": 43, "file_name": "matrix-no-ext"}
                    ]
                }}
            ]
        })))
        .await;
        let client = OpenSubtitlesClient::with_base_url("k", server.uri());
        let candidates = client.search(&query()).await.unwrap();
        assert_eq!(candidates.len(), 2);
        assert_eq!(candidates[0].file_id, "42");
        assert_eq!(candidates[0].language, Some(LanguageCode("en".to_owned())));
        assert_eq!(candidates[0].format, SubtitleFormat::Vtt);
        assert_eq!(
            candidates[0].release_name.as_deref(),
            Some("The.Matrix.1999.BluRay")
        );
        assert_eq!(candidates[1].file_id, "43");
        assert_eq!(candidates[1].format, SubtitleFormat::Srt);
    }

    #[tokio::test]
    async fn search_empty_is_not_found() {
        let server =
            serve_get(ResponseTemplate::new(200).set_body_json(json!({ "data": [] }))).await;
        let client = OpenSubtitlesClient::with_base_url("k", server.uri());
        assert!(matches!(
            client.search(&query()).await,
            Err(SubtitleError::NotFound)
        ));
    }

    #[tokio::test]
    async fn search_non_success_is_backend_error() {
        let server = serve_get(ResponseTemplate::new(500)).await;
        let client = OpenSubtitlesClient::with_base_url("k", server.uri());
        assert!(matches!(
            client.search(&query()).await,
            Err(SubtitleError::Backend(_))
        ));
    }

    #[tokio::test]
    async fn search_malformed_body_is_parse_error() {
        let server = serve_get(ResponseTemplate::new(200).set_body_string("not json")).await;
        let client = OpenSubtitlesClient::with_base_url("k", server.uri());
        assert!(matches!(
            client.search(&query()).await,
            Err(SubtitleError::Parse(_))
        ));
    }

    #[tokio::test]
    async fn search_transport_failure_is_backend_error() {
        let client = OpenSubtitlesClient::with_base_url("k", "http://127.0.0.1:1");
        assert!(matches!(
            client.search(&query()).await,
            Err(SubtitleError::Backend(_))
        ));
    }

    #[tokio::test]
    async fn download_fetches_and_maps_content() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/download"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "link": format!("{}/file.srt", server.uri()),
                "file_name": "matrix.srt"
            })))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/file.srt"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string("1\n00:00:01,000 --> 00:00:02,000\nHello\n"),
            )
            .mount(&server)
            .await;
        let client = OpenSubtitlesClient::with_base_url("k", server.uri());
        let subtitle = client.download("42").await.unwrap();
        assert_eq!(subtitle.format, SubtitleFormat::Srt);
        assert!(subtitle.content.contains("Hello"));
    }

    #[tokio::test]
    async fn download_non_success_is_backend_error() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/download"))
            .respond_with(ResponseTemplate::new(403))
            .mount(&server)
            .await;
        let client = OpenSubtitlesClient::with_base_url("k", server.uri());
        assert!(matches!(
            client.download("42").await,
            Err(SubtitleError::Backend(_))
        ));
    }

    #[tokio::test]
    async fn download_broken_link_is_backend_error() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/download"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "link": format!("{}/missing.srt", server.uri()),
                "file_name": "x.srt"
            })))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/missing.srt"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&server)
            .await;
        let client = OpenSubtitlesClient::with_base_url("k", server.uri());
        assert!(matches!(
            client.download("42").await,
            Err(SubtitleError::Backend(_))
        ));
    }

    #[tokio::test]
    async fn new_builds_with_default_endpoint() {
        let _client = OpenSubtitlesClient::new("k");
    }
}
