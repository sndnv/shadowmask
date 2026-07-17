use std::sync::{Arc, Mutex};

use domain::common::{Page, PageRequest};
use domain::discovery::{SearchKind, SearchResult, search};
use domain::error::RepositoryError;
use domain::repository::SearchIndex;

#[derive(Clone, Default)]
pub struct MockSearchIndex {
    state: Arc<Mutex<State>>,
}

#[derive(Default)]
struct State {
    entries: Vec<SearchResult>,
    rebuilds: u32,
    fail: bool,
}

impl MockSearchIndex {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&self, result: SearchResult) {
        self.state.lock().unwrap().entries.push(result);
    }

    pub fn set_fail(&self) {
        self.state.lock().unwrap().fail = true;
    }

    pub fn rebuild_count(&self) -> u32 {
        self.state.lock().unwrap().rebuilds
    }
}

impl SearchIndex for MockSearchIndex {
    async fn search(
        &self,
        query: &str,
        types: &[SearchKind],
        page: PageRequest,
    ) -> Result<Page<SearchResult>, RepositoryError> {
        let state = self.state.lock().unwrap();
        if state.fail {
            return Err(RepositoryError::Backend("mock search failure".to_owned()));
        }
        Ok(search(&state.entries, query, types, page))
    }

    async fn rebuild(&self) -> Result<(), RepositoryError> {
        let mut state = self.state.lock().unwrap();
        if state.fail {
            return Err(RepositoryError::Backend("mock rebuild failure".to_owned()));
        }
        state.rebuilds += 1;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use domain::catalog::{Movie, MovieId};
    use jiff::Timestamp;

    use super::*;

    fn movie(title: &str) -> SearchResult {
        SearchResult::Movie(Movie {
            id: MovieId(title.to_owned()),
            title: title.to_owned(),
            year: None,
            overview: None,
            runtime_minutes: None,
            content_rating: None,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
            artwork: Vec::new(),
        })
    }

    fn page() -> PageRequest {
        PageRequest {
            offset: 0,
            limit: 10,
        }
    }

    #[tokio::test]
    async fn searches_added_entries_and_counts_rebuilds() {
        let index = MockSearchIndex::new();
        index.add(movie("Matrix"));
        index.add(movie("Inception"));

        let hits = index.search("matrix", &[], page()).await.unwrap();
        assert_eq!(hits.total, 1);

        assert_eq!(index.rebuild_count(), 0);
        index.rebuild().await.unwrap();
        assert_eq!(index.rebuild_count(), 1);
    }

    #[tokio::test]
    async fn surfaces_failures() {
        let index = MockSearchIndex::new();
        index.set_fail();
        assert!(index.search("x", &[], page()).await.is_err());
        assert!(index.rebuild().await.is_err());
    }
}
