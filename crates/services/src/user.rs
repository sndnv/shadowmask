use domain::common::{Page, PageRequest};
use domain::error::{RepositoryError, UserError};
use domain::library::LibraryId;
use domain::repository::UserRepository;
use domain::service::UserService;
use domain::user::{LibraryAccess, NewUser, User, UserId, UserProfileUpdate};
use jiff::Timestamp;
use uuid::Uuid;

use crate::password;

#[derive(Clone)]
pub struct UserServiceImpl<U> {
    users: U,
}

impl<U> UserServiceImpl<U> {
    pub fn new(users: U) -> Self {
        Self { users }
    }
}

fn backend(error: impl ToString) -> UserError {
    UserError::Repository(RepositoryError::Backend(error.to_string()))
}

impl<U> UserService for UserServiceImpl<U>
where
    U: UserRepository + Sync,
{
    async fn create(&self, input: NewUser) -> Result<User, UserError> {
        let password_hash = password::hash(&input.password).map_err(backend)?;
        let user = User {
            id: UserId(Uuid::new_v4().to_string()),
            username: input.username,
            password_hash,
            role: input.role,
            max_content_rating: None,
            preferred_audio: Vec::new(),
            preferred_subtitle: Vec::new(),
            concurrent_stream_limit: None,
            bitrate_cap: None,
            created_at: Timestamp::now(),
        };
        match self.users.create(user.clone()).await {
            Ok(()) => Ok(user),
            Err(RepositoryError::Conflict(_)) => Err(UserError::UsernameTaken),
            Err(other) => Err(other.into()),
        }
    }

    async fn get(&self, id: &UserId) -> Result<User, UserError> {
        self.users.get(id).await?.ok_or(UserError::NotFound)
    }

    async fn list(&self, page: PageRequest) -> Result<Page<User>, UserError> {
        Ok(self.users.list(page).await?)
    }

    async fn update_profile(
        &self,
        id: &UserId,
        update: UserProfileUpdate,
    ) -> Result<User, UserError> {
        let mut user = self.users.get(id).await?.ok_or(UserError::NotFound)?;
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
        self.users.update(user.clone()).await?;
        Ok(user)
    }

    async fn delete(&self, id: &UserId) -> Result<(), UserError> {
        self.users.get(id).await?.ok_or(UserError::NotFound)?;
        self.users.delete(id).await?;
        Ok(())
    }

    async fn library_access(&self, id: &UserId) -> Result<Vec<LibraryAccess>, UserError> {
        self.users.get(id).await?.ok_or(UserError::NotFound)?;
        Ok(self.users.list_library_access(id).await?)
    }

    async fn set_library_access(
        &self,
        id: &UserId,
        libraries: &[LibraryId],
    ) -> Result<(), UserError> {
        self.users.get(id).await?.ok_or(UserError::NotFound)?;
        self.users.set_library_access(id, libraries).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::MockUserRepo;
    use domain::common::LanguageCode;
    use domain::metadata::ContentRating;
    use domain::user::Role;

    fn service() -> UserServiceImpl<MockUserRepo> {
        UserServiceImpl::new(MockUserRepo::new())
    }

    fn new_user(name: &str) -> NewUser {
        NewUser {
            username: name.to_owned(),
            password: "secret".to_owned(),
            role: Role::User,
        }
    }

    fn page() -> PageRequest {
        PageRequest {
            offset: 0,
            limit: 10,
        }
    }

    #[tokio::test]
    async fn create_hashes_and_reads_back() {
        let svc = service();
        let created = svc.create(new_user("alice")).await.unwrap();
        assert_eq!(created.username, "alice");
        assert_ne!(created.password_hash, "secret");
        assert!(Uuid::parse_str(&created.id.0).is_ok());

        let fetched = svc.get(&created.id).await.unwrap();
        assert_eq!(fetched.id, created.id);
        assert_eq!(svc.list(page()).await.unwrap().total, 1);
    }

    #[tokio::test]
    async fn duplicate_username_maps_to_username_taken() {
        let svc = service();
        svc.create(new_user("bob")).await.unwrap();
        assert!(matches!(
            svc.create(new_user("bob")).await.unwrap_err(),
            UserError::UsernameTaken
        ));
    }

    #[tokio::test]
    async fn missing_paths_return_not_found() {
        let svc = service();
        let missing = UserId("nope".into());
        assert!(matches!(
            svc.get(&missing).await.unwrap_err(),
            UserError::NotFound
        ));
        assert!(matches!(
            svc.update_profile(&missing, UserProfileUpdate::default())
                .await
                .unwrap_err(),
            UserError::NotFound
        ));
        assert!(matches!(
            svc.delete(&missing).await.unwrap_err(),
            UserError::NotFound
        ));
        assert!(matches!(
            svc.library_access(&missing).await.unwrap_err(),
            UserError::NotFound
        ));
        assert!(matches!(
            svc.set_library_access(&missing, &[]).await.unwrap_err(),
            UserError::NotFound
        ));
    }

    #[tokio::test]
    async fn update_profile_applies_and_deletes() {
        let svc = service();
        let user = svc.create(new_user("carol")).await.unwrap();
        let updated = svc
            .update_profile(
                &user.id,
                UserProfileUpdate {
                    preferred_audio: Some(vec![LanguageCode("en".into())]),
                    preferred_subtitle: Some(vec![LanguageCode("fr".into())]),
                    max_content_rating: Some(ContentRating {
                        system: "MPAA".into(),
                        code: "R".into(),
                    }),
                    concurrent_stream_limit: Some(2),
                    bitrate_cap: Some(8_000_000),
                },
            )
            .await
            .unwrap();
        assert_eq!(updated.preferred_audio, vec![LanguageCode("en".into())]);
        assert_eq!(updated.concurrent_stream_limit, Some(2));
        assert_eq!(updated.bitrate_cap, Some(8_000_000));

        svc.delete(&user.id).await.unwrap();
        assert!(svc.list(page()).await.unwrap().items.is_empty());
    }

    #[tokio::test]
    async fn library_access_roundtrip() {
        let svc = service();
        let user = svc.create(new_user("erin")).await.unwrap();
        assert!(svc.library_access(&user.id).await.unwrap().is_empty());
        svc.set_library_access(&user.id, &[LibraryId("lib1".into())])
            .await
            .unwrap();
        assert_eq!(svc.library_access(&user.id).await.unwrap().len(), 1);
    }

    #[test]
    fn backend_maps_to_repository_error() {
        assert!(matches!(
            backend("boom"),
            UserError::Repository(RepositoryError::Backend(_))
        ));
    }

    #[tokio::test]
    async fn backend_errors_propagate() {
        let repo = MockUserRepo::new();
        repo.set_fail();
        let svc = UserServiceImpl::new(repo);
        assert!(matches!(
            svc.create(new_user("x")).await.unwrap_err(),
            UserError::Repository(_)
        ));
        assert!(matches!(
            svc.list(page()).await.unwrap_err(),
            UserError::Repository(_)
        ));
        assert!(matches!(
            svc.get(&UserId("x".into())).await.unwrap_err(),
            UserError::Repository(_)
        ));
    }
}
