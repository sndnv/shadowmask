use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use domain::catalog::{Episode, Movie, MovieId, TitleId};
use domain::common::{Page, PageRequest};
use domain::discovery::{ContinueWatchingItem, Hub, SearchKind, SearchResult};
use domain::error::DiscoveryError;
use domain::playback::{ResumeCard, progress_percent};
use domain::service::DiscoveryService;
use domain::session::{NowPlaying, PlaybackSession};
use domain::user::UserId;

use crate::page::paginate;

const MOCK_DURATION_MS: u64 = 3_600_000;

#[derive(Debug, Default)]
struct State {
    search_index: Vec<SearchResult>,
    continue_watching: HashMap<UserId, Vec<ContinueWatchingItem>>,
    next_episodes: HashMap<UserId, Vec<Episode>>,
    next_movies: HashMap<UserId, Vec<Movie>>,
    hubs: Vec<Hub>,
}

#[derive(Clone, Default)]
pub struct MockDiscoveryService {
    state: Arc<Mutex<State>>,
}

impl MockDiscoveryService {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_search_result(&self, result: SearchResult) {
        self.state.lock().unwrap().search_index.push(result);
    }

    pub fn add_continue_watching(&self, user: &UserId, item: ContinueWatchingItem) {
        self.state
            .lock()
            .unwrap()
            .continue_watching
            .entry(user.clone())
            .or_default()
            .push(item);
    }

    pub fn add_next_episode(&self, user: &UserId, episode: Episode) {
        self.state
            .lock()
            .unwrap()
            .next_episodes
            .entry(user.clone())
            .or_default()
            .push(episode);
    }

    pub fn add_next_movie(&self, user: &UserId, movie: Movie) {
        self.state
            .lock()
            .unwrap()
            .next_movies
            .entry(user.clone())
            .or_default()
            .push(movie);
    }

    pub fn add_hub(&self, hub: Hub) {
        self.state.lock().unwrap().hubs.push(hub);
    }
}

fn searchable(result: &SearchResult) -> &str {
    match result {
        SearchResult::Movie(m) => &m.title,
        SearchResult::Series(s) => &s.title,
        SearchResult::Episode(e) => &e.title,
        SearchResult::Person(p) => &p.name,
    }
}

impl DiscoveryService for MockDiscoveryService {
    async fn search(
        &self,
        _user: &UserId,
        query: &str,
        types: &[SearchKind],
        page: PageRequest,
    ) -> Result<Page<SearchResult>, DiscoveryError> {
        let needle = query.to_lowercase();
        let matches: Vec<SearchResult> = self
            .state
            .lock()
            .unwrap()
            .search_index
            .iter()
            .filter(|r| types.is_empty() || types.contains(&r.kind()))
            .filter(|r| searchable(r).to_lowercase().contains(&needle))
            .cloned()
            .collect();
        Ok(paginate(&matches, page))
    }

    async fn continue_watching(
        &self,
        user: &UserId,
    ) -> Result<Vec<ContinueWatchingItem>, DiscoveryError> {
        Ok(self
            .state
            .lock()
            .unwrap()
            .continue_watching
            .get(user)
            .cloned()
            .unwrap_or_default())
    }

    async fn now_playing(
        &self,
        sessions: Vec<PlaybackSession>,
    ) -> Result<Vec<NowPlaying>, DiscoveryError> {
        Ok(sessions
            .into_iter()
            .map(|session| {
                let card = ResumeCard {
                    title: TitleId::Movie(MovieId(session.version.0.clone())),
                    display_title: format!("Title for {}", session.version.0),
                    artwork: Vec::new(),
                    duration_ms: MOCK_DURATION_MS,
                    progress_percent: progress_percent(session.position_ms, MOCK_DURATION_MS),
                };
                NowPlaying { session, card }
            })
            .collect())
    }

    async fn next_episodes(&self, user: &UserId) -> Result<Vec<Episode>, DiscoveryError> {
        Ok(self
            .state
            .lock()
            .unwrap()
            .next_episodes
            .get(user)
            .cloned()
            .unwrap_or_default())
    }

    async fn next_movies(&self, user: &UserId) -> Result<Vec<Movie>, DiscoveryError> {
        Ok(self
            .state
            .lock()
            .unwrap()
            .next_movies
            .get(user)
            .cloned()
            .unwrap_or_default())
    }

    async fn home_hubs(&self, _user: &UserId) -> Result<Vec<Hub>, DiscoveryError> {
        Ok(self.state.lock().unwrap().hubs.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::catalog::{EpisodeId, Movie, MovieId, SeasonId, Series, SeriesId, VersionId};
    use domain::discovery::HubItem;
    use domain::metadata::{Person, PersonId};
    use domain::playback::PlaybackProgress;
    use jiff::Timestamp;

    fn user() -> UserId {
        UserId("u1".into())
    }

    fn movie(title: &str) -> Movie {
        Movie {
            id: MovieId(title.into()),
            title: title.into(),
            year: None,
            overview: None,
            runtime_minutes: None,
            content_rating: None,
            added_at: Timestamp::now(),
            artwork: Vec::new(),
        }
    }

    fn episode(id: &str) -> Episode {
        Episode {
            id: EpisodeId(id.into()),
            season: SeasonId("se1".into()),
            number: 1,
            title: format!("Episode {id}"),
            overview: None,
            runtime_minutes: None,
            air_date: None,
            added_at: Timestamp::now(),
            artwork: Vec::new(),
        }
    }

    #[tokio::test]
    async fn search_filters_and_paginates() {
        let svc = MockDiscoveryService::new();
        svc.add_search_result(SearchResult::Movie(movie("The Matrix")));
        svc.add_search_result(SearchResult::Movie(movie("Inception")));
        svc.add_search_result(SearchResult::Series(Series {
            id: SeriesId("s1".into()),
            title: "The Wire".into(),
            year: None,
            overview: None,
            content_rating: None,
            added_at: Timestamp::now(),
            artwork: Vec::new(),
        }));
        svc.add_search_result(SearchResult::Episode(episode("e1")));
        svc.add_search_result(SearchResult::Person(Person {
            id: PersonId("p1".into()),
            name: "Keanu Reeves".into(),
        }));

        let hits = svc
            .search(
                &user(),
                "matrix",
                &[],
                PageRequest {
                    offset: 0,
                    limit: 10,
                },
            )
            .await
            .unwrap();
        assert_eq!(hits.total, 1);

        let none = svc
            .search(
                &user(),
                "nothing",
                &[],
                PageRequest {
                    offset: 0,
                    limit: 10,
                },
            )
            .await
            .unwrap();
        assert_eq!(none.total, 0);

        let page = PageRequest {
            offset: 0,
            limit: 10,
        };
        let people = svc
            .search(&user(), "keanu", &[SearchKind::Person], page)
            .await
            .unwrap();
        assert_eq!(people.total, 1);
        let wrong_kind = svc
            .search(&user(), "keanu", &[SearchKind::Movie], page)
            .await
            .unwrap();
        assert_eq!(wrong_kind.total, 0);
    }

    #[tokio::test]
    async fn continue_watching_next_episodes_and_movies() {
        let svc = MockDiscoveryService::new();
        assert!(svc.continue_watching(&user()).await.unwrap().is_empty());
        assert!(svc.next_episodes(&user()).await.unwrap().is_empty());
        assert!(svc.next_movies(&user()).await.unwrap().is_empty());

        svc.add_continue_watching(
            &user(),
            ContinueWatchingItem {
                progress: PlaybackProgress {
                    user: user(),
                    version: VersionId("v1".into()),
                    position_ms: 100,
                    updated_at: Timestamp::now(),
                },
                card: ResumeCard {
                    title: TitleId::Movie(MovieId("m1".into())),
                    display_title: "Alpha".into(),
                    artwork: Vec::new(),
                    duration_ms: 1000,
                    progress_percent: 10,
                },
            },
        );
        svc.add_next_episode(&user(), episode("e1"));
        svc.add_next_movie(&user(), movie("Sequel"));

        assert_eq!(svc.continue_watching(&user()).await.unwrap().len(), 1);
        assert_eq!(svc.next_episodes(&user()).await.unwrap().len(), 1);
        assert_eq!(svc.next_movies(&user()).await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn now_playing_fabricates_representative_cards() {
        use domain::session::{DeliveryMode, PlaybackState, SelectedTracks, SessionId};

        let svc = MockDiscoveryService::new();
        assert!(svc.now_playing(Vec::new()).await.unwrap().is_empty());

        let session = PlaybackSession {
            id: SessionId("s1".into()),
            user: user(),
            device: None,
            version: VersionId("v1".into()),
            mode: DeliveryMode::Direct,
            position_ms: 1_800_000,
            state: PlaybackState::Playing,
            selected: SelectedTracks {
                audio_track: None,
                subtitle_track: None,
                subtitle_delivery: None,
            },
            started_at: Timestamp::now(),
            last_heartbeat_at: Timestamp::now(),
        };
        let now = svc.now_playing(vec![session]).await.unwrap();
        assert_eq!(now.len(), 1);
        assert_eq!(now[0].card.display_title, "Title for v1");
        assert_eq!(now[0].card.progress_percent, 50);
    }

    #[tokio::test]
    async fn home_hubs_returned() {
        let svc = MockDiscoveryService::new();
        assert!(svc.home_hubs(&user()).await.unwrap().is_empty());
        svc.add_hub(Hub {
            id: "recent".into(),
            title: "Recently Added".into(),
            items: vec![HubItem::Movie(movie("New Film"))],
        });
        assert_eq!(svc.home_hubs(&user()).await.unwrap().len(), 1);
    }
}
