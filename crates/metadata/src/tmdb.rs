use domain::error::MetadataError;
use domain::metadata::{
    Artwork, ArtworkKind, ExternalId, MediaKind, MetadataMatch, MetadataProvider, MetadataQuery,
    TitleMetadata,
};
use serde::Deserialize;

use crate::http::get_json;
use crate::util::{non_empty, parse_year};

const DEFAULT_BASE_URL: &str = "https://api.themoviedb.org/3";
const DEFAULT_IMAGE_BASE_URL: &str = "https://image.tmdb.org/t/p/original";

pub struct TmdbClient {
    client: reqwest::Client,
    base_url: String,
    image_base_url: String,
    api_key: String,
}

impl TmdbClient {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self::with_base_urls(api_key, DEFAULT_BASE_URL, DEFAULT_IMAGE_BASE_URL)
    }

    pub fn with_base_urls(
        api_key: impl Into<String>,
        base_url: impl Into<String>,
        image_base_url: impl Into<String>,
    ) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into(),
            image_base_url: image_base_url.into(),
            api_key: api_key.into(),
        }
    }

    fn image_url(&self, path: &str) -> String {
        format!("{}{}", self.image_base_url, path)
    }
}

fn tmdb_endpoint(kind: MediaKind) -> &'static str {
    match kind {
        MediaKind::Movie => "movie",
        MediaKind::Series => "tv",
    }
}

impl MetadataProvider for TmdbClient {
    async fn search(&self, query: &MetadataQuery) -> Result<Vec<MetadataMatch>, MetadataError> {
        let kind = query.kind;
        let endpoint = tmdb_endpoint(kind);
        let url = format!("{}/search/{}", self.base_url, endpoint);
        let year_param = if endpoint == "movie" {
            "year"
        } else {
            "first_air_date_year"
        };
        let mut params: Vec<(&str, String)> = vec![
            ("api_key", self.api_key.clone()),
            ("query", query.title.clone()),
        ];
        if let Some(year) = query.year {
            params.push((year_param, year.to_string()));
        }
        let response: RawSearchResponse =
            get_json(self.client.get(url.as_str()).query(&params)).await?;
        if response.results.is_empty() {
            return Err(MetadataError::NotFound);
        }
        let matches = response
            .results
            .into_iter()
            .map(|item| MetadataMatch {
                external_id: ExternalId {
                    source: "tmdb".to_owned(),
                    value: format!("{}/{}", endpoint, item.id),
                },
                title: item.title.or(item.name).unwrap_or_default(),
                year: item
                    .release_date
                    .or(item.first_air_date)
                    .as_deref()
                    .and_then(parse_year),
                kind,
            })
            .collect();
        Ok(matches)
    }

    async fn fetch(&self, id: &ExternalId) -> Result<TitleMetadata, MetadataError> {
        let (endpoint, tmdb_id) = id.value.split_once('/').unwrap_or(("movie", &id.value));
        let url = format!("{}/{}/{}", self.base_url, endpoint, tmdb_id);
        let params = [("api_key", self.api_key.as_str())];
        let detail: RawDetail = get_json(self.client.get(url.as_str()).query(&params)).await?;

        let mut artwork = Vec::new();
        if let Some(path) = detail.poster_path.as_deref() {
            artwork.push(Artwork {
                kind: ArtworkKind::Poster,
                language: None,
                source: "tmdb".to_owned(),
                url: self.image_url(path),
            });
        }
        if let Some(path) = detail.backdrop_path.as_deref() {
            artwork.push(Artwork {
                kind: ArtworkKind::Backdrop,
                language: None,
                source: "tmdb".to_owned(),
                url: self.image_url(path),
            });
        }
        let runtime_minutes = detail
            .runtime
            .or_else(|| detail.episode_run_time.first().copied());
        Ok(TitleMetadata {
            title: detail.title.or(detail.name).unwrap_or_default(),
            year: detail
                .release_date
                .or(detail.first_air_date)
                .as_deref()
                .and_then(parse_year),
            overview: detail.overview.and_then(|o| non_empty(&o)),
            runtime_minutes,
            content_rating: None,
            ratings: Vec::new(),
            genres: detail.genres.into_iter().map(|g| g.name).collect(),
            artwork,
            external_ids: vec![ExternalId {
                source: "tmdb".to_owned(),
                value: id.value.clone(),
            }],
        })
    }
}

#[derive(Deserialize)]
struct RawSearchResponse {
    #[serde(default)]
    results: Vec<RawSearchItem>,
}

#[derive(Deserialize)]
struct RawSearchItem {
    id: u64,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    release_date: Option<String>,
    #[serde(default)]
    first_air_date: Option<String>,
}

#[derive(Deserialize)]
struct RawDetail {
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    overview: Option<String>,
    #[serde(default)]
    runtime: Option<u32>,
    #[serde(default)]
    episode_run_time: Vec<u32>,
    #[serde(default)]
    release_date: Option<String>,
    #[serde(default)]
    first_air_date: Option<String>,
    #[serde(default)]
    poster_path: Option<String>,
    #[serde(default)]
    backdrop_path: Option<String>,
    #[serde(default)]
    genres: Vec<RawGenre>,
}

#[derive(Deserialize)]
struct RawGenre {
    name: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    async fn mock_path(route: &str, body: ResponseTemplate) -> MockServer {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path(route.to_owned()))
            .respond_with(body)
            .mount(&server)
            .await;
        server
    }

    fn client(server: &MockServer) -> TmdbClient {
        TmdbClient::with_base_urls("k", server.uri(), "http://img.test")
    }

    #[tokio::test]
    async fn search_movie_returns_matches() {
        let server = mock_path(
            "/search/movie",
            ResponseTemplate::new(200).set_body_json(json!({
                "results": [
                    {"id": 603, "title": "The Matrix", "release_date": "1999-03-31"}
                ]
            })),
        )
        .await;
        let query = MetadataQuery {
            title: "the matrix".to_owned(),
            year: Some(1999),
            kind: MediaKind::Movie,
        };
        let matches = client(&server).search(&query).await.unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].title, "The Matrix");
        assert_eq!(matches[0].year, Some(1999));
        assert_eq!(matches[0].external_id.value, "movie/603");
        assert_eq!(matches[0].kind, MediaKind::Movie);
    }

    #[tokio::test]
    async fn search_tv_returns_matches() {
        let server = mock_path(
            "/search/tv",
            ResponseTemplate::new(200).set_body_json(json!({
                "results": [
                    {"id": 1399, "name": "Game of Thrones", "first_air_date": "2011-04-17"}
                ]
            })),
        )
        .await;
        let query = MetadataQuery {
            title: "game of thrones".to_owned(),
            year: None,
            kind: MediaKind::Series,
        };
        let matches = client(&server).search(&query).await.unwrap();
        assert_eq!(matches[0].title, "Game of Thrones");
        assert_eq!(matches[0].year, Some(2011));
        assert_eq!(matches[0].external_id.value, "tv/1399");
    }

    #[tokio::test]
    async fn search_empty_is_not_found() {
        let server = mock_path(
            "/search/movie",
            ResponseTemplate::new(200).set_body_json(json!({ "results": [] })),
        )
        .await;
        let query = MetadataQuery {
            title: "nope".to_owned(),
            year: None,
            kind: MediaKind::Movie,
        };
        assert!(matches!(
            client(&server).search(&query).await,
            Err(MetadataError::NotFound)
        ));
    }

    #[tokio::test]
    async fn fetch_movie_maps_detail() {
        let server = mock_path(
            "/movie/603",
            ResponseTemplate::new(200).set_body_json(json!({
                "title": "The Matrix",
                "overview": "Welcome to the real world.",
                "runtime": 136,
                "release_date": "1999-03-31",
                "poster_path": "/poster.jpg",
                "backdrop_path": "/back.jpg",
                "genres": [{"id": 28, "name": "Action"}, {"id": 878, "name": "Science Fiction"}]
            })),
        )
        .await;
        let id = ExternalId {
            source: "tmdb".to_owned(),
            value: "movie/603".to_owned(),
        };
        let meta = client(&server).fetch(&id).await.unwrap();
        assert_eq!(meta.title, "The Matrix");
        assert_eq!(meta.year, Some(1999));
        assert_eq!(meta.runtime_minutes, Some(136));
        assert_eq!(meta.overview.as_deref(), Some("Welcome to the real world."));
        assert_eq!(meta.content_rating, None);
        assert_eq!(meta.genres, vec!["Action", "Science Fiction"]);
        assert_eq!(meta.artwork.len(), 2);
        assert_eq!(meta.artwork[0].kind, ArtworkKind::Poster);
        assert_eq!(meta.artwork[0].url, "http://img.test/poster.jpg");
        assert_eq!(meta.artwork[1].kind, ArtworkKind::Backdrop);
        assert_eq!(meta.artwork[1].url, "http://img.test/back.jpg");
        assert_eq!(meta.external_ids[0].value, "movie/603");
    }

    #[tokio::test]
    async fn fetch_tv_uses_episode_run_time_and_name() {
        let server = mock_path(
            "/tv/1399",
            ResponseTemplate::new(200).set_body_json(json!({
                "name": "Game of Thrones",
                "overview": "",
                "episode_run_time": [50],
                "first_air_date": "2011-04-17",
                "poster_path": "/got.jpg"
            })),
        )
        .await;
        let id = ExternalId {
            source: "tmdb".to_owned(),
            value: "tv/1399".to_owned(),
        };
        let meta = client(&server).fetch(&id).await.unwrap();
        assert_eq!(meta.title, "Game of Thrones");
        assert_eq!(meta.year, Some(2011));
        assert_eq!(meta.runtime_minutes, Some(50));
        assert_eq!(meta.overview, None);
        assert_eq!(meta.artwork.len(), 1);
        assert_eq!(meta.artwork[0].kind, ArtworkKind::Poster);
        assert!(meta.genres.is_empty());
    }

    #[tokio::test]
    async fn fetch_bare_id_defaults_to_movie_endpoint() {
        let server = mock_path(
            "/movie/12345",
            ResponseTemplate::new(200).set_body_json(json!({})),
        )
        .await;
        let id = ExternalId {
            source: "tmdb".to_owned(),
            value: "12345".to_owned(),
        };
        let meta = client(&server).fetch(&id).await.unwrap();
        assert_eq!(meta.title, "");
        assert_eq!(meta.year, None);
        assert_eq!(meta.runtime_minutes, None);
        assert_eq!(meta.overview, None);
        assert!(meta.artwork.is_empty());
        assert!(meta.genres.is_empty());
        assert_eq!(meta.external_ids[0].value, "12345");
    }

    #[tokio::test]
    async fn new_builds_with_default_endpoints() {
        let _client = TmdbClient::new("k");
    }
}
