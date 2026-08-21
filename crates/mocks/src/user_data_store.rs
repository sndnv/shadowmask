use std::sync::{Arc, Mutex};

use domain::error::RepositoryError;
use domain::repository::UserDataStore;
use domain::user::UserId;

#[derive(Clone, Default)]
pub struct MockUserDataStore {
    purged: Arc<Mutex<Vec<UserId>>>,
}

impl MockUserDataStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn purged(&self) -> Vec<UserId> {
        self.purged.lock().unwrap().clone()
    }
}

impl UserDataStore for MockUserDataStore {
    async fn purge(&self, user: &UserId) -> Result<(), RepositoryError> {
        self.purged.lock().unwrap().push(user.clone());
        Ok(())
    }
}
