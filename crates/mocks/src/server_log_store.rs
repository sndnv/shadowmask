use std::sync::{Arc, Mutex};

use domain::diagnostics::{ServerLogPage, ServerLogRequest, ServerLogStore};
use domain::error::ServerLogError;

#[derive(Clone, Default)]
pub struct MockServerLogStore {
    page: Arc<Mutex<ServerLogPage>>,
    refuses_cursors: bool,
    requests: Arc<Mutex<Vec<ServerLogRequest>>>,
}

impl MockServerLogStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_page(page: ServerLogPage) -> Self {
        Self { page: Arc::new(Mutex::new(page)), ..Self::default() }
    }

    pub fn refusing_cursors() -> Self {
        Self { refuses_cursors: true, ..Self::default() }
    }

    pub fn requests(&self) -> Vec<ServerLogRequest> {
        self.requests.lock().unwrap().clone()
    }
}

impl ServerLogStore for MockServerLogStore {
    async fn read(&self, request: ServerLogRequest) -> Result<ServerLogPage, ServerLogError> {
        self.requests.lock().unwrap().push(request);
        if self.refuses_cursors {
            return Err(ServerLogError::InvalidCursor);
        }
        Ok(self.page.lock().unwrap().clone())
    }
}

#[cfg(test)]
mod tests {
    use domain::diagnostics::ServerLogFrom;

    use super::*;

    fn newest() -> ServerLogRequest {
        ServerLogRequest { from: ServerLogFrom::Newest, limit: 5, min_level: None }
    }

    #[tokio::test]
    async fn records_each_request_and_answers_its_page() {
        let page = ServerLogPage { entries: vec!["a".into()], ..ServerLogPage::default() };
        let store = MockServerLogStore::with_page(page.clone());

        assert_eq!(store.read(newest()).await.unwrap(), page);
        assert_eq!(store.requests(), vec![newest()]);
        assert_eq!(
            MockServerLogStore::new().read(newest()).await.unwrap(),
            ServerLogPage::default()
        );
    }

    #[tokio::test]
    async fn can_refuse_every_cursor() {
        let store = MockServerLogStore::refusing_cursors();

        assert!(matches!(store.read(newest()).await, Err(ServerLogError::InvalidCursor)));
    }
}
