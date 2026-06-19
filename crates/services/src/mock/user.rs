use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use domain::error::UserError;
use domain::library::LibraryId;
use domain::service::UserService;
use domain::user::{LibraryAccess, NewUser, User, UserId, UserProfileUpdate};
use jiff::Timestamp;

#[derive(Debug, Default)]
struct State {
    users: Vec<User>,
    access: HashMap<UserId, Vec<LibraryId>>,
    next_id: u64,
}

#[derive(Clone, Default)]
pub struct MockUserService {
    state: Arc<Mutex<State>>,
}

impl MockUserService {
    pub fn new() -> Self {
        Self::default()
    }
}

impl UserService for MockUserService {
    async fn create(&self, input: NewUser) -> Result<User, UserError> {
        let mut state = self.state.lock().unwrap();
        if state.users.iter().any(|u| u.username == input.username) {
            return Err(UserError::UsernameTaken);
        }
        state.next_id += 1;
        let user = User {
            id: UserId(format!("user-{}", state.next_id)),
            username: input.username,
            password_hash: input.password,
            role: input.role,
            max_content_rating: None,
            preferred_audio: Vec::new(),
            preferred_subtitle: Vec::new(),
            concurrent_stream_limit: None,
            bitrate_cap: None,
            created_at: Timestamp::now(),
        };
        state.users.push(user.clone());
        Ok(user)
    }

    async fn get(&self, id: &UserId) -> Result<User, UserError> {
        self.state
            .lock()
            .unwrap()
            .users
            .iter()
            .find(|u| &u.id == id)
            .cloned()
            .ok_or(UserError::NotFound)
    }

    async fn list(&self) -> Result<Vec<User>, UserError> {
        Ok(self.state.lock().unwrap().users.clone())
    }

    async fn update_profile(
        &self,
        id: &UserId,
        update: UserProfileUpdate,
    ) -> Result<User, UserError> {
        let mut state = self.state.lock().unwrap();
        let user = state
            .users
            .iter_mut()
            .find(|u| &u.id == id)
            .ok_or(UserError::NotFound)?;
        if let Some(v) = update.preferred_audio {
            user.preferred_audio = v;
        }
        if let Some(v) = update.preferred_subtitle {
            user.preferred_subtitle = v;
        }
        if let Some(v) = update.max_content_rating {
            user.max_content_rating = Some(v);
        }
        if let Some(v) = update.concurrent_stream_limit {
            user.concurrent_stream_limit = Some(v);
        }
        if let Some(v) = update.bitrate_cap {
            user.bitrate_cap = Some(v);
        }
        Ok(user.clone())
    }

    async fn delete(&self, id: &UserId) -> Result<(), UserError> {
        let mut state = self.state.lock().unwrap();
        let before = state.users.len();
        state.users.retain(|u| &u.id != id);
        if state.users.len() == before {
            return Err(UserError::NotFound);
        }
        state.access.remove(id);
        Ok(())
    }

    async fn library_access(&self, id: &UserId) -> Result<Vec<LibraryAccess>, UserError> {
        let state = self.state.lock().unwrap();
        if !state.users.iter().any(|u| &u.id == id) {
            return Err(UserError::NotFound);
        }
        Ok(state
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
    ) -> Result<(), UserError> {
        let mut state = self.state.lock().unwrap();
        if !state.users.iter().any(|u| &u.id == id) {
            return Err(UserError::NotFound);
        }
        state.access.insert(id.clone(), libraries.to_vec());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::common::LanguageCode;
    use domain::metadata::ContentRating;
    use domain::user::Role;

    fn new_user(name: &str) -> NewUser {
        NewUser {
            username: name.to_string(),
            password: "secret".to_string(),
            role: Role::User,
        }
    }

    #[tokio::test]
    async fn create_get_list() {
        let svc = MockUserService::new();
        let created = svc.create(new_user("alice")).await.unwrap();
        assert_eq!(created.username, "alice");

        let fetched = svc.get(&created.id).await.unwrap();
        assert_eq!(fetched.id, created.id);
        assert_eq!(svc.list().await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn duplicate_username_rejected() {
        let svc = MockUserService::new();
        svc.create(new_user("bob")).await.unwrap();
        assert!(matches!(
            svc.create(new_user("bob")).await.unwrap_err(),
            UserError::UsernameTaken
        ));
    }

    #[tokio::test]
    async fn get_missing_is_not_found() {
        let svc = MockUserService::new();
        assert!(matches!(
            svc.get(&UserId("nope".into())).await.unwrap_err(),
            UserError::NotFound
        ));
    }

    #[tokio::test]
    async fn update_profile_applies_fields() {
        let svc = MockUserService::new();
        let user = svc.create(new_user("carol")).await.unwrap();
        let update = UserProfileUpdate {
            preferred_audio: Some(vec![LanguageCode("en".into())]),
            preferred_subtitle: Some(vec![LanguageCode("fr".into())]),
            max_content_rating: Some(ContentRating {
                system: "MPAA".into(),
                code: "PG-13".into(),
            }),
            concurrent_stream_limit: Some(3),
            bitrate_cap: Some(8_000_000),
        };
        let updated = svc.update_profile(&user.id, update).await.unwrap();
        assert_eq!(updated.preferred_audio, vec![LanguageCode("en".into())]);
        assert_eq!(updated.preferred_subtitle, vec![LanguageCode("fr".into())]);
        assert_eq!(
            updated.max_content_rating,
            Some(ContentRating {
                system: "MPAA".into(),
                code: "PG-13".into(),
            })
        );
        assert_eq!(updated.concurrent_stream_limit, Some(3));
        assert_eq!(updated.bitrate_cap, Some(8_000_000));

        assert!(matches!(
            svc.update_profile(&UserId("nope".into()), UserProfileUpdate::default())
                .await
                .unwrap_err(),
            UserError::NotFound
        ));
    }

    #[tokio::test]
    async fn delete_removes_user() {
        let svc = MockUserService::new();
        let user = svc.create(new_user("dave")).await.unwrap();
        svc.delete(&user.id).await.unwrap();
        assert!(svc.list().await.unwrap().is_empty());
        assert!(matches!(
            svc.delete(&user.id).await.unwrap_err(),
            UserError::NotFound
        ));
    }

    #[tokio::test]
    async fn library_access_roundtrip() {
        let svc = MockUserService::new();
        let user = svc.create(new_user("erin")).await.unwrap();
        assert!(svc.library_access(&user.id).await.unwrap().is_empty());

        svc.set_library_access(
            &user.id,
            &[LibraryId("lib1".into()), LibraryId("lib2".into())],
        )
        .await
        .unwrap();
        let access = svc.library_access(&user.id).await.unwrap();
        assert_eq!(access.len(), 2);

        assert!(matches!(
            svc.library_access(&UserId("nope".into()))
                .await
                .unwrap_err(),
            UserError::NotFound
        ));
        assert!(matches!(
            svc.set_library_access(&UserId("nope".into()), &[])
                .await
                .unwrap_err(),
            UserError::NotFound
        ));
    }
}
