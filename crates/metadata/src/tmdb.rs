use std::sync::Arc;
use std::time::Duration;

use domain::error::MetadataError;
use domain::metadata::{
    Artwork, ArtworkKind, CollectionMeta, CreditInfo, CreditRole, EpisodeArtwork, ExternalId,
    MediaKind, MetadataMatch, MetadataProvider, MetadataQuery, PersonMetadata, SeasonArtwork,
    TitleMetadata,
};
use serde::Deserialize;

use crate::http::get_json;
use crate::rate_limiter::RateLimiter;
use crate::util::{non_empty, parse_year};

const DEFAULT_BASE_URL: &str = "https://api.themoviedb.org/3";
const DEFAULT_IMAGE_BASE_URL: &str = "https://image.tmdb.org/t/p/original";

#[derive(Clone)]
pub struct TmdbClient {
    client: reqwest::Client,
    base_url: String,
    image_base_url: String,
    api_key: String,
    limiter: Arc<RateLimiter>,
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
            limiter: Arc::new(RateLimiter::new(Duration::ZERO)),
        }
    }

    pub fn with_min_interval(mut self, min_interval: Duration) -> Self {
        self.limiter = Arc::new(RateLimiter::new(min_interval));
        self
    }

    fn image_url(&self, path: &str) -> String {
        format!("{}{}", self.image_base_url, path)
    }

    async fn resolve_ref(&self, id: &ExternalId) -> Result<(String, String), MetadataError> {
        if id.source == "imdb" || id.value.starts_with("tt") {
            let url = format!("{}/find/{}", self.base_url, id.value);
            let params = [
                ("api_key", self.api_key.as_str()),
                ("external_source", "imdb_id"),
            ];
            let found: RawFindResponse =
                get_json(&self.limiter, self.client.get(url.as_str()).query(&params)).await?;
            if let Some(item) = found.movie_results.first() {
                Ok(("movie".to_owned(), item.id.to_string()))
            } else if let Some(item) = found.tv_results.first() {
                Ok(("tv".to_owned(), item.id.to_string()))
            } else {
                Err(MetadataError::NotFound)
            }
        } else {
            let (endpoint, tmdb_id) = id.value.split_once('/').unwrap_or(("movie", &id.value));
            Ok((endpoint.to_owned(), tmdb_id.to_owned()))
        }
    }
}

fn tmdb_endpoint(kind: MediaKind) -> &'static str {
    match kind {
        MediaKind::Movie => "movie",
        MediaKind::Series => "tv",
    }
}

fn person_external_id(id: u64) -> ExternalId {
    ExternalId {
        source: "tmdb".to_owned(),
        value: format!("person/{id}"),
    }
}

fn crew_role(job: &str) -> Option<CreditRole> {
    match job {
        "Director" => Some(CreditRole::Director),
        "Writer" | "Screenplay" | "Story" => Some(CreditRole::Writer),
        _ => None,
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
            get_json(&self.limiter, self.client.get(url.as_str()).query(&params)).await?;
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
        let (endpoint, tmdb_id) = self.resolve_ref(id).await?;
        let url = format!("{}/{}/{}", self.base_url, endpoint, tmdb_id);
        let params = [
            ("api_key", self.api_key.as_str()),
            ("append_to_response", "credits,external_ids"),
        ];
        let detail: RawDetail =
            get_json(&self.limiter, self.client.get(url.as_str()).query(&params)).await?;

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
        let credits = detail.credits.unwrap_or_default();
        let mut cast: Vec<CreditInfo> = credits
            .cast
            .into_iter()
            .map(|member| CreditInfo {
                external_person_id: person_external_id(member.id),
                name: member.name,
                role: CreditRole::Actor,
                character: member.character.and_then(|c| non_empty(&c)),
                order: member.order,
            })
            .collect();
        cast.extend(credits.crew.into_iter().filter_map(|member| {
            crew_role(&member.job).map(|role| CreditInfo {
                external_person_id: person_external_id(member.id),
                name: member.name,
                role,
                character: None,
                order: 0,
            })
        }));
        let collection = detail.belongs_to_collection.map(|c| {
            let mut artwork = Vec::new();
            if let Some(path) = c.poster_path.as_deref() {
                artwork.push(Artwork {
                    kind: ArtworkKind::Poster,
                    language: None,
                    source: "tmdb".to_owned(),
                    url: self.image_url(path),
                });
            }
            if let Some(path) = c.backdrop_path.as_deref() {
                artwork.push(Artwork {
                    kind: ArtworkKind::Backdrop,
                    language: None,
                    source: "tmdb".to_owned(),
                    url: self.image_url(path),
                });
            }
            CollectionMeta {
                external_id: ExternalId {
                    source: "tmdb".to_owned(),
                    value: format!("collection/{}", c.id),
                },
                name: c.name,
                artwork,
            }
        });
        let tmdb_value = if id.source == "imdb" || id.value.starts_with("tt") {
            format!("{endpoint}/{tmdb_id}")
        } else {
            id.value.clone()
        };
        let mut external_ids = vec![ExternalId {
            source: "tmdb".to_owned(),
            value: tmdb_value,
        }];
        if let Some(imdb_id) = detail
            .external_ids
            .and_then(|ids| ids.imdb_id)
            .and_then(|value| non_empty(&value))
        {
            external_ids.push(ExternalId {
                source: "imdb".to_owned(),
                value: imdb_id,
            });
        }
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
            cast,
            studios: detail
                .production_companies
                .into_iter()
                .map(|c| c.name)
                .collect(),
            artwork,
            external_ids,
            collection,
        })
    }

    async fn fetch_season(
        &self,
        id: &ExternalId,
        season: u16,
    ) -> Result<SeasonArtwork, MetadataError> {
        let (endpoint, tmdb_id) = self.resolve_ref(id).await?;
        let url = format!(
            "{}/{}/{}/season/{}",
            self.base_url, endpoint, tmdb_id, season
        );
        let params = [("api_key", self.api_key.as_str())];
        let detail: RawSeasonDetail =
            get_json(&self.limiter, self.client.get(url.as_str()).query(&params)).await?;

        let mut artwork = Vec::new();
        if let Some(path) = detail.poster_path.as_deref() {
            artwork.push(Artwork {
                kind: ArtworkKind::Poster,
                language: None,
                source: "tmdb".to_owned(),
                url: self.image_url(path),
            });
        }
        let episodes = detail
            .episodes
            .into_iter()
            .map(|episode| {
                let mut artwork = Vec::new();
                if let Some(path) = episode.still_path.as_deref() {
                    artwork.push(Artwork {
                        kind: ArtworkKind::Backdrop,
                        language: None,
                        source: "tmdb".to_owned(),
                        url: self.image_url(path),
                    });
                }
                EpisodeArtwork {
                    number: episode.episode_number,
                    name: episode.name.and_then(|value| non_empty(&value)),
                    overview: episode.overview.and_then(|value| non_empty(&value)),
                    air_date: episode.air_date.and_then(|value| non_empty(&value)),
                    runtime_minutes: episode.runtime,
                    artwork,
                }
            })
            .collect();
        Ok(SeasonArtwork {
            number: season,
            name: detail.name.and_then(|value| non_empty(&value)),
            overview: detail.overview.and_then(|value| non_empty(&value)),
            artwork,
            episodes,
        })
    }

    async fn fetch_person(&self, id: &ExternalId) -> Result<PersonMetadata, MetadataError> {
        let url = format!("{}/{}", self.base_url, id.value);
        let params = [("api_key", self.api_key.as_str())];
        let detail: RawPerson =
            get_json(&self.limiter, self.client.get(url.as_str()).query(&params)).await?;

        let mut artwork = Vec::new();
        if let Some(path) = detail.profile_path.as_deref() {
            artwork.push(Artwork {
                kind: ArtworkKind::Poster,
                language: None,
                source: "tmdb".to_owned(),
                url: self.image_url(path),
            });
        }
        Ok(PersonMetadata {
            name: detail.name.unwrap_or_default(),
            biography: detail.biography.and_then(|value| non_empty(&value)),
            birthday: detail.birthday.and_then(|value| non_empty(&value)),
            deathday: detail.deathday.and_then(|value| non_empty(&value)),
            place_of_birth: detail.place_of_birth.and_then(|value| non_empty(&value)),
            also_known_as: detail.also_known_as,
            artwork,
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
struct RawFindResponse {
    #[serde(default)]
    movie_results: Vec<RawFindItem>,
    #[serde(default)]
    tv_results: Vec<RawFindItem>,
}

#[derive(Deserialize)]
struct RawFindItem {
    id: u64,
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
    #[serde(default)]
    production_companies: Vec<RawCompany>,
    #[serde(default)]
    credits: Option<RawCredits>,
    #[serde(default)]
    belongs_to_collection: Option<RawCollection>,
    #[serde(default)]
    external_ids: Option<RawExternalIds>,
}

#[derive(Deserialize)]
struct RawExternalIds {
    #[serde(default)]
    imdb_id: Option<String>,
}

#[derive(Deserialize)]
struct RawSeasonDetail {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    overview: Option<String>,
    #[serde(default)]
    poster_path: Option<String>,
    #[serde(default)]
    episodes: Vec<RawEpisode>,
}

#[derive(Deserialize)]
struct RawEpisode {
    #[serde(default)]
    episode_number: u16,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    overview: Option<String>,
    #[serde(default)]
    air_date: Option<String>,
    #[serde(default)]
    runtime: Option<u32>,
    #[serde(default)]
    still_path: Option<String>,
}

#[derive(Deserialize)]
struct RawPerson {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    biography: Option<String>,
    #[serde(default)]
    birthday: Option<String>,
    #[serde(default)]
    deathday: Option<String>,
    #[serde(default)]
    place_of_birth: Option<String>,
    #[serde(default)]
    also_known_as: Vec<String>,
    #[serde(default)]
    profile_path: Option<String>,
}

#[derive(Deserialize)]
struct RawCollection {
    id: u64,
    #[serde(default)]
    name: String,
    #[serde(default)]
    poster_path: Option<String>,
    #[serde(default)]
    backdrop_path: Option<String>,
}

#[derive(Deserialize)]
struct RawGenre {
    name: String,
}

#[derive(Deserialize)]
struct RawCompany {
    #[serde(default)]
    name: String,
}

#[derive(Deserialize, Default)]
struct RawCredits {
    #[serde(default)]
    cast: Vec<RawCast>,
    #[serde(default)]
    crew: Vec<RawCrew>,
}

#[derive(Deserialize)]
struct RawCast {
    id: u64,
    #[serde(default)]
    name: String,
    #[serde(default)]
    character: Option<String>,
    #[serde(default)]
    order: u32,
}

#[derive(Deserialize)]
struct RawCrew {
    id: u64,
    #[serde(default)]
    name: String,
    #[serde(default)]
    job: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn with_min_interval_sets_the_limiter() {
        let _ = TmdbClient::new("k").with_min_interval(Duration::from_millis(10));
    }

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
                "genres": [{"id": 28, "name": "Action"}, {"id": 878, "name": "Science Fiction"}],
                "external_ids": {"imdb_id": "tt0133093"}
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
        assert_eq!(meta.external_ids[0].source, "tmdb");
        assert_eq!(meta.external_ids[0].value, "movie/603");
        assert_eq!(meta.external_ids[1].source, "imdb");
        assert_eq!(meta.external_ids[1].value, "tt0133093");
        assert!(meta.collection.is_none());
    }

    #[tokio::test]
    async fn fetch_movie_maps_belongs_to_collection() {
        let server = mock_path(
            "/movie/603",
            ResponseTemplate::new(200).set_body_json(json!({
                "title": "The Matrix",
                "belongs_to_collection": {
                    "id": 2344,
                    "name": "The Matrix Collection",
                    "poster_path": "/coll-poster.jpg",
                    "backdrop_path": "/coll-back.jpg"
                }
            })),
        )
        .await;
        let id = ExternalId {
            source: "tmdb".to_owned(),
            value: "movie/603".to_owned(),
        };
        let meta = client(&server).fetch(&id).await.unwrap();
        let collection = meta.collection.expect("collection");
        assert_eq!(collection.name, "The Matrix Collection");
        assert_eq!(collection.external_id.source, "tmdb");
        assert_eq!(collection.external_id.value, "collection/2344");
        assert_eq!(collection.artwork.len(), 2);
        assert_eq!(collection.artwork[0].kind, ArtworkKind::Poster);
        assert_eq!(collection.artwork[0].url, "http://img.test/coll-poster.jpg");
        assert_eq!(collection.artwork[1].kind, ArtworkKind::Backdrop);
        assert_eq!(collection.artwork[1].url, "http://img.test/coll-back.jpg");
    }

    #[tokio::test]
    async fn fetch_movie_maps_credits_and_studios() {
        let server = mock_path(
            "/movie/603",
            ResponseTemplate::new(200).set_body_json(json!({
                "title": "The Matrix",
                "production_companies": [
                    {"id": 1, "name": "Warner Bros."},
                    {"id": 2, "name": "Village Roadshow"}
                ],
                "credits": {
                    "cast": [
                        {"id": 6384, "name": "Keanu Reeves", "character": "Neo", "order": 0},
                        {"id": 2975, "name": "Laurence Fishburne", "character": "", "order": 1}
                    ],
                    "crew": [
                        {"id": 9339, "name": "Lana Wachowski", "job": "Director"},
                        {"id": 9340, "name": "Lilly Wachowski", "job": "Writer"},
                        {"id": 9341, "name": "Someone", "job": "Screenplay"},
                        {"id": 9342, "name": "Producer Person", "job": "Producer"}
                    ]
                }
            })),
        )
        .await;
        let id = ExternalId {
            source: "tmdb".to_owned(),
            value: "movie/603".to_owned(),
        };
        let meta = client(&server).fetch(&id).await.unwrap();
        assert_eq!(meta.studios, vec!["Warner Bros.", "Village Roadshow"]);
        assert_eq!(meta.cast.len(), 5);
        assert_eq!(meta.cast[0].name, "Keanu Reeves");
        assert_eq!(meta.cast[0].role, CreditRole::Actor);
        assert_eq!(meta.cast[0].character.as_deref(), Some("Neo"));
        assert_eq!(meta.cast[0].order, 0);
        assert_eq!(meta.cast[0].external_person_id.value, "person/6384");
        assert_eq!(meta.cast[1].character, None);
        assert_eq!(meta.cast[2].role, CreditRole::Director);
        assert_eq!(meta.cast[2].character, None);
        assert_eq!(meta.cast[3].role, CreditRole::Writer);
        assert_eq!(meta.cast[4].role, CreditRole::Writer);
        assert!(meta.cast.iter().all(|c| c.name != "Producer Person"));
    }

    #[tokio::test]
    async fn fetch_missing_credits_yields_empty_cast_and_studios() {
        let server = mock_path(
            "/movie/1",
            ResponseTemplate::new(200).set_body_json(json!({"title": "Bare"})),
        )
        .await;
        let id = ExternalId {
            source: "tmdb".to_owned(),
            value: "movie/1".to_owned(),
        };
        let meta = client(&server).fetch(&id).await.unwrap();
        assert!(meta.cast.is_empty());
        assert!(meta.studios.is_empty());
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
    async fn fetch_resolves_imdb_id_to_movie() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/find/tt0133093"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "movie_results": [{"id": 603}],
                "tv_results": []
            })))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/movie/603"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "title": "The Matrix",
                "external_ids": {"imdb_id": "tt0133093"}
            })))
            .mount(&server)
            .await;
        let id = ExternalId {
            source: "imdb".to_owned(),
            value: "tt0133093".to_owned(),
        };
        let meta = client(&server).fetch(&id).await.unwrap();
        assert_eq!(meta.title, "The Matrix");
        assert_eq!(meta.external_ids[0].source, "tmdb");
        assert_eq!(meta.external_ids[0].value, "movie/603");
        assert!(
            meta.external_ids
                .iter()
                .any(|e| e.source == "imdb" && e.value == "tt0133093")
        );
    }

    #[tokio::test]
    async fn fetch_resolves_imdb_id_to_tv_by_value_prefix() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/find/tt0944947"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "movie_results": [],
                "tv_results": [{"id": 1399}]
            })))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/tv/1399"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({"name": "Game of Thrones"})),
            )
            .mount(&server)
            .await;
        let id = ExternalId {
            source: "external".to_owned(),
            value: "tt0944947".to_owned(),
        };
        let meta = client(&server).fetch(&id).await.unwrap();
        assert_eq!(meta.title, "Game of Thrones");
        assert_eq!(meta.external_ids[0].value, "tv/1399");
    }

    #[tokio::test]
    async fn fetch_imdb_id_not_found_when_no_results() {
        let server = mock_path(
            "/find/tt0000000",
            ResponseTemplate::new(200)
                .set_body_json(json!({"movie_results": [], "tv_results": []})),
        )
        .await;
        let id = ExternalId {
            source: "imdb".to_owned(),
            value: "tt0000000".to_owned(),
        };
        assert!(matches!(
            client(&server).fetch(&id).await,
            Err(MetadataError::NotFound)
        ));
    }

    #[tokio::test]
    async fn fetch_season_maps_poster_and_episode_stills() {
        let server = mock_path(
            "/tv/1399/season/1",
            ResponseTemplate::new(200).set_body_json(json!({
                "season_number": 1,
                "name": "Season 1",
                "overview": "The first season.",
                "poster_path": "/season1.jpg",
                "episodes": [
                    {
                        "episode_number": 1,
                        "name": "Pilot",
                        "overview": "It begins.",
                        "air_date": "2010-10-01",
                        "runtime": 42,
                        "still_path": "/e1.jpg"
                    },
                    {"episode_number": 2, "still_path": null}
                ]
            })),
        )
        .await;
        let id = ExternalId {
            source: "tmdb".to_owned(),
            value: "tv/1399".to_owned(),
        };
        let season = client(&server).fetch_season(&id, 1).await.unwrap();
        assert_eq!(season.number, 1);
        assert_eq!(season.name.as_deref(), Some("Season 1"));
        assert_eq!(season.overview.as_deref(), Some("The first season."));
        assert_eq!(season.artwork.len(), 1);
        assert_eq!(season.artwork[0].kind, ArtworkKind::Poster);
        assert_eq!(season.artwork[0].url, "http://img.test/season1.jpg");
        assert_eq!(season.episodes.len(), 2);
        assert_eq!(season.episodes[0].number, 1);
        assert_eq!(season.episodes[0].name.as_deref(), Some("Pilot"));
        assert_eq!(season.episodes[0].overview.as_deref(), Some("It begins."));
        assert_eq!(season.episodes[0].air_date.as_deref(), Some("2010-10-01"));
        assert_eq!(season.episodes[0].runtime_minutes, Some(42));
        assert_eq!(season.episodes[0].artwork.len(), 1);
        assert_eq!(season.episodes[0].artwork[0].kind, ArtworkKind::Backdrop);
        assert_eq!(season.episodes[0].artwork[0].url, "http://img.test/e1.jpg");
        assert_eq!(season.episodes[1].number, 2);
        assert_eq!(season.episodes[1].name, None);
        assert!(season.episodes[1].artwork.is_empty());
    }

    #[tokio::test]
    async fn fetch_person_maps_detail_and_photo() {
        let server = mock_path(
            "/person/6384",
            ResponseTemplate::new(200).set_body_json(json!({
                "name": "Keanu Reeves",
                "biography": "A Canadian actor.",
                "birthday": "1964-09-02",
                "deathday": null,
                "place_of_birth": "Beirut, Lebanon",
                "also_known_as": ["Keanu Charles Reeves"],
                "profile_path": "/keanu.jpg"
            })),
        )
        .await;
        let id = ExternalId {
            source: "tmdb".to_owned(),
            value: "person/6384".to_owned(),
        };
        let person = client(&server).fetch_person(&id).await.unwrap();
        assert_eq!(person.name, "Keanu Reeves");
        assert_eq!(person.biography.as_deref(), Some("A Canadian actor."));
        assert_eq!(person.birthday.as_deref(), Some("1964-09-02"));
        assert_eq!(person.deathday, None);
        assert_eq!(person.place_of_birth.as_deref(), Some("Beirut, Lebanon"));
        assert_eq!(person.also_known_as, vec!["Keanu Charles Reeves"]);
        assert_eq!(person.artwork.len(), 1);
        assert_eq!(person.artwork[0].kind, ArtworkKind::Poster);
        assert_eq!(person.artwork[0].url, "http://img.test/keanu.jpg");
    }

    #[tokio::test]
    async fn new_builds_with_default_endpoints() {
        let _client = TmdbClient::new("k");
    }
}
