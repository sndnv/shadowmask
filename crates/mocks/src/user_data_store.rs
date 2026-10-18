use std::sync::{Arc, Mutex};

use domain::error::RepositoryError;
use domain::repository::UserDataStore;
use domain::user::UserId;

use crate::hold::{Hold, NextHold};

#[derive(Clone, Default)]
pub struct MockUserDataStore {
    purged: Arc<Mutex<Vec<UserId>>>,
    purge_hold: NextHold,
}

impl MockUserDataStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn purged(&self) -> Vec<UserId> {
        self.purged.lock().unwrap().clone()
    }

    pub fn hold_next_purge(&self) -> Hold {
        self.purge_hold.arm()
    }
}

impl UserDataStore for MockUserDataStore {
    async fn purge(&self, user: &UserId) -> Result<(), RepositoryError> {
        self.purge_hold.pass().await;
        self.purged.lock().unwrap().push(user.clone());
        Ok(())
    }
}
