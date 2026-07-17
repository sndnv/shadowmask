use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use domain::common::{Page, PageRequest};
use domain::error::RepositoryError;
use domain::library::LibraryId;
use domain::repository::UserRepository;
use domain::user::{LibraryAccess, User, UserId};

use crate::page::paginate;

#[derive(Default)]
struct State {
    users: Vec<User>,
    access: HashMap<UserId, Vec<LibraryId>>,
}

#[derive(Clone, Default)]
pub struct MockUserRepo {
    state: Arc<Mutex<State>>,
    fail: Arc<AtomicBool>,
}

impl MockUserRepo {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&self, user: User) {
        self.state.lock().unwrap().users.push(user);
    }

    pub fn set_fail(&self) {
        self.fail.store(true, Ordering::Relaxed);
    }

    fn guard(&self) -> Result<(), RepositoryError> {
        if self.fail.load(Ordering::Relaxed) {
            Err(RepositoryError::Backend("mock user failure".to_owned()))
        } else {
            Ok(())
        }
    }
}

impl UserRepository for MockUserRepo {
    async fn create(&self, user: User) -> Result<(), RepositoryError> {
        self.guard()?;
        let mut state = self.state.lock().unwrap();
        if state.users.iter().any(|u| u.username == user.username) {
            return Err(RepositoryError::Conflict("username taken".to_owned()));
        }
        state.users.push(user);
        Ok(())
    }

    async fn get(&self, id: &UserId) -> Result<Option<User>, RepositoryError> {
        self.guard()?;
        Ok(self
            .state
            .lock()
            .unwrap()
            .users
            .iter()
            .find(|u| &u.id == id)
            .cloned())
    }

    async fn find_by_username(&self, username: &str) -> Result<Option<User>, RepositoryError> {
        self.guard()?;
        Ok(self
            .state
            .lock()
            .unwrap()
            .users
            .iter()
            .find(|u| u.username == username)
            .cloned())
    }

    async fn list(&self, page: PageRequest) -> Result<Page<User>, RepositoryError> {
        self.guard()?;
        Ok(paginate(&self.state.lock().unwrap().users, page))
    }

    async fn update(&self, user: User) -> Result<(), RepositoryError> {
        self.guard()?;
        let mut state = self.state.lock().unwrap();
        if let Some(existing) = state.users.iter_mut().find(|u| u.id == user.id) {
            *existing = user;
        }
        Ok(())
    }

    async fn delete(&self, id: &UserId) -> Result<(), RepositoryError> {
        self.guard()?;
        let mut state = self.state.lock().unwrap();
        state.users.retain(|u| &u.id != id);
        state.access.remove(id);
        Ok(())
    }

    async fn list_library_access(
        &self,
        id: &UserId,
    ) -> Result<Vec<LibraryAccess>, RepositoryError> {
        self.guard()?;
        Ok(self
            .state
            .lock()
            .unwrap()
            .access
            .get(id)
            .into_iter()
            .flatten()
            .map(|library| LibraryAccess {
                user: id.clone(),
                library: library.clone(),
            })
            .collect())
    }

    async fn set_library_access(
        &self,
        id: &UserId,
        libraries: &[LibraryId],
    ) -> Result<(), RepositoryError> {
        self.guard()?;
        self.state
            .lock()
            .unwrap()
            .access
            .insert(id.clone(), libraries.to_vec());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::user::Role;
    use jiff::Timestamp;

    fn user(id: &str, username: &str) -> User {
        User {
            id: UserId(id.to_owned()),
            username: username.to_owned(),
            password_hash: "hash".to_owned(),
            role: Role::User,
            max_content_rating: None,
            preferred_audio: Vec::new(),
            preferred_subtitle: Vec::new(),
            concurrent_stream_limit: None,
            bitrate_cap: None,
            created_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
        }
    }

    fn page() -> PageRequest {
        PageRequest {
            offset: 0,
            limit: 10,
        }
    }

    #[tokio::test]
    async fn create_read_update_delete_and_access() {
        let repo = MockUserRepo::new();
        repo.create(user("u1", "alice")).await.unwrap();
        assert!(matches!(
            repo.create(user("u2", "alice")).await.unwrap_err(),
            RepositoryError::Conflict(_)
        ));

        assert!(repo.get(&UserId("u1".into())).await.unwrap().is_some());
        assert!(repo.get(&UserId("x".into())).await.unwrap().is_none());
        assert!(repo.find_by_username("alice").await.unwrap().is_some());
        assert!(repo.find_by_username("ghost").await.unwrap().is_none());
        assert_eq!(repo.list(page()).await.unwrap().total, 1);

        let mut updated = user("u1", "alice");
        updated.bitrate_cap = Some(42);
        repo.update(updated).await.unwrap();
        repo.update(user("missing", "nobody")).await.unwrap();
        assert_eq!(
            repo.get(&UserId("u1".into()))
                .await
                .unwrap()
                .unwrap()
                .bitrate_cap,
            Some(42)
        );

        assert!(
            repo.list_library_access(&UserId("u1".into()))
                .await
                .unwrap()
                .is_empty()
        );
        repo.set_library_access(&UserId("u1".into()), &[LibraryId("lib1".into())])
            .await
            .unwrap();
        assert_eq!(
            repo.list_library_access(&UserId("u1".into()))
                .await
                .unwrap()
                .len(),
            1
        );

        repo.delete(&UserId("u1".into())).await.unwrap();
        assert!(repo.get(&UserId("u1".into())).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn surfaces_backend_failure() {
        let repo = MockUserRepo::new();
        repo.set_fail();
        assert!(repo.create(user("u1", "alice")).await.is_err());
        assert!(repo.get(&UserId("u1".into())).await.is_err());
        assert!(repo.find_by_username("alice").await.is_err());
        assert!(repo.list(page()).await.is_err());
        assert!(repo.update(user("u1", "alice")).await.is_err());
        assert!(repo.delete(&UserId("u1".into())).await.is_err());
        assert!(
            repo.list_library_access(&UserId("u1".into()))
                .await
                .is_err()
        );
        assert!(
            repo.set_library_access(&UserId("u1".into()), &[])
                .await
                .is_err()
        );
    }
}
