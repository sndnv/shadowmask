use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use domain::catalog::{TitleSet, VersionId};
use domain::error::RepositoryError;
use domain::repository::UserDataStore;
use domain::user::UserId;

use crate::hold::{Hold, NextHold};

pub type Forgotten = (UserId, TitleSet, HashSet<VersionId>);

#[derive(Clone, Default)]
pub struct MockUserDataStore {
    purged: Arc<Mutex<Vec<UserId>>>,
    forgotten: Arc<Mutex<Vec<Forgotten>>>,
    fail_forget: Arc<AtomicBool>,
    purge_hold: NextHold,
}

impl MockUserDataStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn purged(&self) -> Vec<UserId> {
        self.purged.lock().unwrap().clone()
    }

    pub fn forgotten(&self) -> Vec<Forgotten> {
        self.forgotten.lock().unwrap().clone()
    }

    pub fn set_fail_forget(&self) {
        self.fail_forget.store(true, Ordering::Relaxed);
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

    async fn forget(
        &self,
        user: &UserId,
        titles: &TitleSet,
        versions: &HashSet<VersionId>,
    ) -> Result<(), RepositoryError> {
        if self.fail_forget.load(Ordering::Relaxed) {
            return Err(RepositoryError::Backend("mock forget failure".to_owned()));
        }
        self.forgotten.lock().unwrap().push((user.clone(), titles.clone(), versions.clone()));
        Ok(())
    }
}
