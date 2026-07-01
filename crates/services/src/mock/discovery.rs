use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use domain::catalog::{Episode, Movie};
use domain::common::{Page, PageRequest};
use domain::discovery::{ContinueWatchingItem, Hub, SearchResult};
use domain::error::DiscoveryError;
use domain::service::DiscoveryService;
use domain::user::UserId;

use crate::page::paginate;

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
        page: PageRequest,
    ) -> Result<Page<SearchResult>, DiscoveryError> {
        let needle = query.to_lowercase();
        let matches: Vec<SearchResult> = self
            .state
            .lock()
            .unwrap()
            .search_index
            .iter()
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
                PageRequest {
                    offset: 0,
                    limit: 10,
                },
            )
            .await
            .unwrap();
        assert_eq!(none.total, 0);
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
            },
        );
        svc.add_next_episode(&user(), episode("e1"));
        svc.add_next_movie(&user(), movie("Sequel"));

        assert_eq!(svc.continue_watching(&user()).await.unwrap().len(), 1);
        assert_eq!(svc.next_episodes(&user()).await.unwrap().len(), 1);
        assert_eq!(svc.next_movies(&user()).await.unwrap().len(), 1);
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
