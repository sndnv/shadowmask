use domain::error::MetadataError;
use domain::metadata::{
    Artwork, ArtworkKind, ContentRating, ExternalId, MediaKind, MetadataMatch, MetadataProvider,
    MetadataQuery, Rating, TitleMetadata,
};
use serde::Deserialize;

use crate::http::get_json;
use crate::util::{non_na, parse_leading_f32, parse_leading_u32, parse_year};

const DEFAULT_BASE_URL: &str = "https://www.omdbapi.com";

pub struct OmdbClient {
    client: reqwest::Client,
    base_url: String,
    api_key: String,
}

impl OmdbClient {
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
}

fn omdb_type(kind: MediaKind) -> &'static str {
    match kind {
        MediaKind::Movie => "movie",
        MediaKind::Series => "series",
    }
}

fn rating_system(kind: &str) -> &'static str {
    if kind.eq_ignore_ascii_case("series") {
        "US-TV"
    } else {
        "MPAA"
    }
}

impl MetadataProvider for OmdbClient {
    async fn search(&self, query: &MetadataQuery) -> Result<Vec<MetadataMatch>, MetadataError> {
        let kind = query.kind;
        let mut params: Vec<(&str, String)> = vec![
            ("apikey", self.api_key.clone()),
            ("s", query.title.clone()),
            ("type", omdb_type(kind).to_owned()),
        ];
        if let Some(year) = query.year {
            params.push(("y", year.to_string()));
        }
        let response: RawSearchResponse =
            get_json(self.client.get(self.base_url.as_str()).query(&params)).await?;
        if response.response != "True" {
            return Err(MetadataError::NotFound);
        }
        let matches = response
            .search
            .unwrap_or_default()
            .into_iter()
            .map(|item| MetadataMatch {
                external_id: ExternalId {
                    source: "imdb".to_owned(),
                    value: item.imdb_id,
                },
                title: item.title,
                year: parse_year(&item.year),
                kind,
            })
            .collect();
        Ok(matches)
    }

    async fn fetch(&self, id: &ExternalId) -> Result<TitleMetadata, MetadataError> {
        let params = [("apikey", self.api_key.as_str()), ("i", id.value.as_str())];
        let detail: RawDetail =
            get_json(self.client.get(self.base_url.as_str()).query(&params)).await?;
        if detail.response != "True" {
            return Err(MetadataError::NotFound);
        }
        let content_rating = non_na(&detail.rated).map(|code| ContentRating {
            system: rating_system(&detail.kind).to_owned(),
            code,
        });
        let ratings = detail
            .ratings
            .into_iter()
            .filter_map(|raw| {
                parse_leading_f32(&raw.value).map(|value| Rating {
                    source: raw.source,
                    value,
                })
            })
            .collect();
        let genres = non_na(&detail.genre)
            .map(|genre| {
                genre
                    .split(',')
                    .map(|g| g.trim().to_owned())
                    .filter(|g| !g.is_empty())
                    .collect()
            })
            .unwrap_or_default();
        let artwork = non_na(&detail.poster)
            .map(|url| {
                vec![Artwork {
                    kind: ArtworkKind::Poster,
                    language: None,
                    source: "omdb".to_owned(),
                    url,
                }]
            })
            .unwrap_or_default();
        Ok(TitleMetadata {
            title: detail.title,
            year: parse_year(&detail.year),
            overview: non_na(&detail.plot),
            runtime_minutes: parse_leading_u32(&detail.runtime),
            content_rating,
            ratings,
            genres,
            artwork,
            external_ids: vec![ExternalId {
                source: "imdb".to_owned(),
                value: detail.imdb_id,
            }],
        })
    }
}

#[derive(Deserialize)]
struct RawSearchResponse {
    #[serde(rename = "Response")]
    response: String,
    #[serde(rename = "Search")]
    search: Option<Vec<RawSearchItem>>,
}

#[derive(Deserialize)]
struct RawSearchItem {
    #[serde(rename = "Title")]
    title: String,
    #[serde(rename = "Year")]
    year: String,
    #[serde(rename = "imdbID")]
    imdb_id: String,
}

#[derive(Deserialize)]
struct RawDetail {
    #[serde(rename = "Response")]
    response: String,
    #[serde(rename = "Title", default)]
    title: String,
    #[serde(rename = "Year", default)]
    year: String,
    #[serde(rename = "Rated", default)]
    rated: String,
    #[serde(rename = "Runtime", default)]
    runtime: String,
    #[serde(rename = "Genre", default)]
    genre: String,
    #[serde(rename = "Plot", default)]
    plot: String,
    #[serde(rename = "Poster", default)]
    poster: String,
    #[serde(rename = "Type", default)]
    kind: String,
    #[serde(rename = "imdbID", default)]
    imdb_id: String,
    #[serde(rename = "Ratings", default)]
    ratings: Vec<RawRating>,
}

#[derive(Deserialize)]
struct RawRating {
    #[serde(rename = "Source")]
    source: String,
    #[serde(rename = "Value")]
    value: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use wiremock::matchers::method;
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn movie_query(title: &str, year: Option<u16>) -> MetadataQuery {
        MetadataQuery {
            title: title.to_owned(),
            year,
            kind: MediaKind::Movie,
        }
    }

    async fn mock_server(body: ResponseTemplate) -> MockServer {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(body)
            .mount(&server)
            .await;
        server
    }

    #[tokio::test]
    async fn search_returns_matches() {
        let server = mock_server(ResponseTemplate::new(200).set_body_json(json!({
            "Response": "True",
            "Search": [
                {"Title": "The Matrix", "Year": "1999", "imdbID": "tt0133093"}
            ]
        })))
        .await;
        let client = OmdbClient::with_base_url("k", server.uri());
        let matches = client
            .search(&movie_query("the matrix", Some(1999)))
            .await
            .unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].title, "The Matrix");
        assert_eq!(matches[0].year, Some(1999));
        assert_eq!(matches[0].kind, MediaKind::Movie);
        assert_eq!(matches[0].external_id.source, "imdb");
        assert_eq!(matches[0].external_id.value, "tt0133093");
    }

    #[tokio::test]
    async fn search_not_found_is_error() {
        let server = mock_server(ResponseTemplate::new(200).set_body_json(json!({
            "Response": "False",
            "Error": "Movie not found!"
        })))
        .await;
        let client = OmdbClient::with_base_url("k", server.uri());
        let query = MetadataQuery {
            title: "nope".to_owned(),
            year: None,
            kind: MediaKind::Series,
        };
        assert!(matches!(
            client.search(&query).await,
            Err(MetadataError::NotFound)
        ));
    }

    #[tokio::test]
    async fn fetch_maps_full_detail() {
        let server = mock_server(ResponseTemplate::new(200).set_body_json(json!({
            "Response": "True",
            "Type": "movie",
            "Title": "The Matrix",
            "Year": "1999",
            "Rated": "R",
            "Runtime": "136 min",
            "Genre": "Action, Sci-Fi",
            "Plot": "A hacker learns the truth.",
            "Poster": "https://example.test/p.jpg",
            "imdbID": "tt0133093",
            "Ratings": [
                {"Source": "Internet Movie Database", "Value": "8.7/10"},
                {"Source": "Rotten Tomatoes", "Value": "88%"}
            ]
        })))
        .await;
        let client = OmdbClient::with_base_url("k", server.uri());
        let id = ExternalId {
            source: "imdb".to_owned(),
            value: "tt0133093".to_owned(),
        };
        let meta = client.fetch(&id).await.unwrap();
        assert_eq!(meta.title, "The Matrix");
        assert_eq!(meta.year, Some(1999));
        assert_eq!(meta.runtime_minutes, Some(136));
        assert_eq!(meta.overview.as_deref(), Some("A hacker learns the truth."));
        assert_eq!(
            meta.content_rating,
            Some(ContentRating {
                system: "MPAA".to_owned(),
                code: "R".to_owned()
            })
        );
        assert_eq!(meta.genres, vec!["Action", "Sci-Fi"]);
        assert_eq!(meta.ratings.len(), 2);
        assert_eq!(meta.ratings[0].value, 8.7);
        assert_eq!(meta.ratings[1].value, 88.0);
        assert_eq!(meta.artwork.len(), 1);
        assert_eq!(meta.artwork[0].kind, ArtworkKind::Poster);
        assert_eq!(meta.artwork[0].url, "https://example.test/p.jpg");
        assert_eq!(meta.external_ids[0].value, "tt0133093");
    }

    #[tokio::test]
    async fn fetch_series_uses_us_tv_system() {
        let server = mock_server(ResponseTemplate::new(200).set_body_json(json!({
            "Response": "True",
            "Type": "series",
            "Title": "Breaking Bad",
            "Year": "2008",
            "Rated": "TV-MA",
            "imdbID": "tt0903747"
        })))
        .await;
        let client = OmdbClient::with_base_url("k", server.uri());
        let id = ExternalId {
            source: "imdb".to_owned(),
            value: "tt0903747".to_owned(),
        };
        let meta = client.fetch(&id).await.unwrap();
        assert_eq!(
            meta.content_rating,
            Some(ContentRating {
                system: "US-TV".to_owned(),
                code: "TV-MA".to_owned()
            })
        );
    }

    #[tokio::test]
    async fn fetch_handles_na_fields() {
        let server = mock_server(ResponseTemplate::new(200).set_body_json(json!({
            "Response": "True",
            "Type": "movie",
            "Title": "Obscure",
            "Year": "N/A",
            "Rated": "N/A",
            "Runtime": "N/A",
            "Genre": "N/A",
            "Plot": "N/A",
            "Poster": "N/A",
            "imdbID": "tt9999999",
            "Ratings": [{"Source": "x", "Value": "N/A"}]
        })))
        .await;
        let client = OmdbClient::with_base_url("k", server.uri());
        let id = ExternalId {
            source: "imdb".to_owned(),
            value: "tt9999999".to_owned(),
        };
        let meta = client.fetch(&id).await.unwrap();
        assert_eq!(meta.year, None);
        assert_eq!(meta.runtime_minutes, None);
        assert_eq!(meta.overview, None);
        assert_eq!(meta.content_rating, None);
        assert!(meta.genres.is_empty());
        assert!(meta.artwork.is_empty());
        assert!(meta.ratings.is_empty());
    }

    #[tokio::test]
    async fn fetch_not_found_is_error() {
        let server = mock_server(ResponseTemplate::new(200).set_body_json(json!({
            "Response": "False",
            "Error": "Incorrect IMDb ID."
        })))
        .await;
        let client = OmdbClient::with_base_url("k", server.uri());
        let id = ExternalId {
            source: "imdb".to_owned(),
            value: "tt0".to_owned(),
        };
        assert!(matches!(
            client.fetch(&id).await,
            Err(MetadataError::NotFound)
        ));
    }

    #[tokio::test]
    async fn non_success_status_is_backend_error() {
        let server = mock_server(ResponseTemplate::new(500)).await;
        let client = OmdbClient::with_base_url("k", server.uri());
        assert!(matches!(
            client.search(&movie_query("x", None)).await,
            Err(MetadataError::Backend(_))
        ));
    }

    #[tokio::test]
    async fn malformed_body_is_parse_error() {
        let server = mock_server(ResponseTemplate::new(200).set_body_string("not json")).await;
        let client = OmdbClient::with_base_url("k", server.uri());
        assert!(matches!(
            client.search(&movie_query("x", None)).await,
            Err(MetadataError::Parse(_))
        ));
    }

    #[tokio::test]
    async fn transport_failure_is_backend_error() {
        let client = OmdbClient::with_base_url("k", "http://127.0.0.1:1");
        assert!(matches!(
            client.search(&movie_query("x", None)).await,
            Err(MetadataError::Backend(_))
        ));
    }

    #[tokio::test]
    async fn new_builds_with_default_endpoint() {
        let client = OmdbClient::new("k");
        assert_eq!(client.base_url, DEFAULT_BASE_URL);
        assert_eq!(client.api_key, "k");
    }
}
