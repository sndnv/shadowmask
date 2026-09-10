use domain::common::{Page, PageRequest};
use domain::error::{RepositoryError, UserError};
use domain::library::LibraryId;
use domain::repository::{AuthTokenRepository, UserDataStore, UserRepository};
use domain::service::UserService;
use domain::user::{LibraryAccess, NewUser, Principal, Role, User, UserId, UserProfileUpdate};
use jiff::Timestamp;
use uuid::Uuid;

use crate::password;

#[derive(Clone)]
pub struct UserServiceImpl<U, T, D> {
    users: U,
    tokens: T,
    data: D,
}

impl<U, T, D> UserServiceImpl<U, T, D> {
    pub fn new(users: U, tokens: T, data: D) -> Self {
        Self {
            users,
            tokens,
            data,
        }
    }
}

fn backend(error: impl ToString) -> UserError {
    UserError::Repository(RepositoryError::Backend(error.to_string()))
}

impl<U, T, D> UserService for UserServiceImpl<U, T, D>
where
    U: UserRepository + Sync,
    T: AuthTokenRepository + Sync,
    D: UserDataStore + Sync,
{
    async fn create(&self, input: NewUser) -> Result<User, UserError> {
        if input.password.trim().is_empty() {
            return Err(UserError::EmptyPassword);
        }
        let password_hash = password::hash(&input.password).map_err(backend)?;
        let now = Timestamp::now();
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
            active: true,
            created_at: now,
            updated_at: now,
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
        user.updated_at = Timestamp::now();
        self.users.update(user.clone()).await?;
        Ok(user)
    }

    async fn delete(&self, caller: &Principal, id: &UserId) -> Result<(), UserError> {
        if &caller.user == id {
            return Err(UserError::CannotDeleteSelf);
        }
        self.users.get(id).await?.ok_or(UserError::NotFound)?;
        self.tokens.purge_user(id).await?;
        self.users.delete(id).await?;
        self.data.purge(id).await?;
        Ok(())
    }

    async fn set_active(
        &self,
        caller: &Principal,
        id: &UserId,
        active: bool,
    ) -> Result<User, UserError> {
        if !active && &caller.user == id {
            return Err(UserError::CannotDeactivateSelf);
        }
        let mut user = self.users.get(id).await?.ok_or(UserError::NotFound)?;
        if user.active == active {
            return Ok(user);
        }
        user.active = active;
        user.updated_at = Timestamp::now();
        self.users.update(user.clone()).await?;
        if !active {
            self.tokens.purge_user(id).await?;
        }
        Ok(user)
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

    async fn change_password(
        &self,
        actor: &Principal,
        target: &UserId,
        current: Option<&str>,
        new_password: &str,
    ) -> Result<(), UserError> {
        if new_password.trim().is_empty() {
            return Err(UserError::EmptyPassword);
        }
        let mut user = self.users.get(target).await?.ok_or(UserError::NotFound)?;
        let admin_reset = actor.role == Role::Admin && &actor.user != target;
        if !admin_reset {
            let current = current.ok_or(UserError::InvalidPassword)?;
            if !password::verify(current, &user.password_hash).map_err(backend)? {
                return Err(UserError::InvalidPassword);
            }
        }
        user.password_hash = password::hash(new_password).map_err(backend)?;
        user.updated_at = Timestamp::now();
        self.users.update(user).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::common::LanguageCode;
    use domain::metadata::ContentRating;
    use domain::user::Role;
    use mocks::{MockAuthTokenRepo, MockUserDataStore, MockUserRepo};

    fn service() -> UserServiceImpl<MockUserRepo, MockAuthTokenRepo, MockUserDataStore> {
        UserServiceImpl::new(
            MockUserRepo::new(),
            MockAuthTokenRepo::new(),
            MockUserDataStore::new(),
        )
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
            svc.delete(&principal("admin", Role::Admin), &missing)
                .await
                .unwrap_err(),
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

        svc.delete(&principal("admin", Role::Admin), &user.id)
            .await
            .unwrap();
        assert!(svc.list(page()).await.unwrap().items.is_empty());
    }

    #[tokio::test]
    async fn delete_refuses_the_caller_s_own_account() {
        let svc = service();
        let user = svc.create(new_user("gail")).await.unwrap();
        let actor = principal(&user.id.0, Role::Admin);

        assert!(matches!(
            svc.delete(&actor, &user.id).await.unwrap_err(),
            UserError::CannotDeleteSelf
        ));
        assert_eq!(svc.list(page()).await.unwrap().items.len(), 1);
    }

    #[tokio::test]
    async fn deleting_an_account_takes_its_credentials_and_its_files_with_it() {
        let users = MockUserRepo::new();
        let tokens = MockAuthTokenRepo::new();
        let data = MockUserDataStore::new();
        let svc = UserServiceImpl::new(users, tokens.clone(), data.clone());
        let user = svc.create(new_user("hal")).await.unwrap();
        let admin = principal("admin", Role::Admin);

        svc.delete(&admin, &user.id).await.unwrap();

        assert_eq!(tokens.purged(), vec![user.id.clone()]);
        assert_eq!(data.purged(), vec![user.id]);
    }

    #[tokio::test]
    async fn deactivating_revokes_credentials_but_keeps_the_account_and_its_files() {
        let users = MockUserRepo::new();
        let tokens = MockAuthTokenRepo::new();
        let data = MockUserDataStore::new();
        let svc = UserServiceImpl::new(users, tokens.clone(), data.clone());
        let user = svc.create(new_user("ivy")).await.unwrap();
        let admin = principal("admin", Role::Admin);

        let off = svc.set_active(&admin, &user.id, false).await.unwrap();

        assert!(!off.active);
        assert_eq!(tokens.purged(), vec![user.id.clone()]);
        assert!(data.purged().is_empty());
        assert!(!svc.get(&user.id).await.unwrap().active);
    }

    #[tokio::test]
    async fn reactivating_does_not_revoke_anything() {
        let users = MockUserRepo::new();
        let tokens = MockAuthTokenRepo::new();
        let svc = UserServiceImpl::new(users, tokens.clone(), MockUserDataStore::new());
        let user = svc.create(new_user("jo")).await.unwrap();
        let admin = principal("admin", Role::Admin);
        svc.set_active(&admin, &user.id, false).await.unwrap();

        let on = svc.set_active(&admin, &user.id, true).await.unwrap();

        assert!(on.active);
        assert_eq!(tokens.purged().len(), 1);
    }

    #[tokio::test]
    async fn setting_active_to_what_it_already_is_changes_nothing() {
        let users = MockUserRepo::new();
        let tokens = MockAuthTokenRepo::new();
        let svc = UserServiceImpl::new(users, tokens.clone(), MockUserDataStore::new());
        let user = svc.create(new_user("kim")).await.unwrap();
        let admin = principal("admin", Role::Admin);

        let same = svc.set_active(&admin, &user.id, true).await.unwrap();

        assert!(same.active);
        assert!(tokens.purged().is_empty());
    }

    #[tokio::test]
    async fn an_admin_cannot_deactivate_themselves_but_can_reactivate() {
        let svc = service();
        let user = svc.create(new_user("len")).await.unwrap();
        let actor = principal(&user.id.0, Role::Admin);

        assert!(matches!(
            svc.set_active(&actor, &user.id, false).await.unwrap_err(),
            UserError::CannotDeactivateSelf
        ));
        assert!(svc.set_active(&actor, &user.id, true).await.is_ok());
    }

    #[tokio::test]
    async fn setting_active_on_a_missing_account_is_not_found() {
        let svc = service();
        let admin = principal("admin", Role::Admin);

        assert!(matches!(
            svc.set_active(&admin, &UserId("ghost".into()), false)
                .await
                .unwrap_err(),
            UserError::NotFound
        ));
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

    fn principal(id: &str, role: Role) -> Principal {
        Principal {
            user: UserId(id.into()),
            role,
        }
    }

    #[tokio::test]
    async fn self_change_password_verifies_current() {
        let svc = service();
        let user = svc.create(new_user("frank")).await.unwrap();
        let actor = principal(&user.id.0, Role::User);
        svc.change_password(&actor, &user.id, Some("secret"), "fresh")
            .await
            .unwrap();
        let stored = svc.get(&user.id).await.unwrap();
        assert!(password::verify("fresh", &stored.password_hash).unwrap());
    }

    #[tokio::test]
    async fn self_change_password_rejects_wrong_or_missing_current() {
        let svc = service();
        let user = svc.create(new_user("grace")).await.unwrap();
        let actor = principal(&user.id.0, Role::User);
        assert!(matches!(
            svc.change_password(&actor, &user.id, Some("nope"), "fresh")
                .await
                .unwrap_err(),
            UserError::InvalidPassword
        ));
        assert!(matches!(
            svc.change_password(&actor, &user.id, None, "fresh")
                .await
                .unwrap_err(),
            UserError::InvalidPassword
        ));
    }

    #[tokio::test]
    async fn admin_reset_skips_current_password() {
        let svc = service();
        let user = svc.create(new_user("heidi")).await.unwrap();
        let admin = principal("admin", Role::Admin);
        svc.change_password(&admin, &user.id, None, "reset")
            .await
            .unwrap();
        let stored = svc.get(&user.id).await.unwrap();
        assert!(password::verify("reset", &stored.password_hash).unwrap());
    }

    #[tokio::test]
    async fn admin_changing_own_password_still_verifies() {
        let svc = service();
        let admin_user = svc.create(new_user("ivan")).await.unwrap();
        let admin = principal(&admin_user.id.0, Role::Admin);
        assert!(matches!(
            svc.change_password(&admin, &admin_user.id, Some("wrong"), "fresh")
                .await
                .unwrap_err(),
            UserError::InvalidPassword
        ));
        svc.change_password(&admin, &admin_user.id, Some("secret"), "fresh")
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn change_password_missing_user_is_not_found() {
        let svc = service();
        let admin = principal("admin", Role::Admin);
        assert!(matches!(
            svc.change_password(&admin, &UserId("nope".into()), None, "x")
                .await
                .unwrap_err(),
            UserError::NotFound
        ));
    }

    #[tokio::test]
    async fn create_rejects_an_empty_password() {
        let svc = service();
        for blank in ["", "   "] {
            let mut input = new_user("alice");
            input.password = blank.to_owned();
            assert!(matches!(
                svc.create(input).await.unwrap_err(),
                UserError::EmptyPassword
            ));
        }
        assert!(svc.list(page()).await.unwrap().items.is_empty());
    }

    #[tokio::test]
    async fn a_password_keeps_the_spaces_around_it() {
        let svc = service();
        for secret in ["  my-pass", "pass  ", "  my-pass  "] {
            let mut input = new_user(secret);
            input.password = secret.to_owned();
            let user = svc.create(input).await.unwrap();
            assert!(password::verify(secret, &user.password_hash).unwrap());
            assert!(!password::verify(secret.trim(), &user.password_hash).unwrap());
        }
    }

    #[tokio::test]
    async fn change_password_keeps_the_spaces_around_it() {
        let svc = service();
        let user = svc.create(new_user("alice")).await.unwrap();
        let admin = principal("admin", Role::Admin);
        svc.change_password(&admin, &user.id, None, "  my-pass  ")
            .await
            .unwrap();
        let stored = svc.get(&user.id).await.unwrap();
        assert!(password::verify("  my-pass  ", &stored.password_hash).unwrap());
        assert!(!password::verify("my-pass", &stored.password_hash).unwrap());
    }

    #[tokio::test]
    async fn change_password_rejects_an_empty_password() {
        let svc = service();
        let user = svc.create(new_user("alice")).await.unwrap();
        let hash = user.password_hash.clone();
        let admin = principal("admin", Role::Admin);
        for blank in ["", "   "] {
            assert!(matches!(
                svc.change_password(&admin, &user.id, None, blank)
                    .await
                    .unwrap_err(),
                UserError::EmptyPassword
            ));
        }
        assert_eq!(svc.get(&user.id).await.unwrap().password_hash, hash);
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
        let svc = UserServiceImpl::new(repo, MockAuthTokenRepo::new(), MockUserDataStore::new());
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
        assert!(matches!(
            svc.change_password(
                &principal("admin", Role::Admin),
                &UserId("x".into()),
                None,
                "new"
            )
            .await
            .unwrap_err(),
            UserError::Repository(_)
        ));

        let id = UserId("x".into());
        let boss = principal("admin", Role::Admin);

        macro_rules! is_repository_error {
            ($call:expr) => {
                assert!(matches!($call.await.unwrap_err(), UserError::Repository(_)))
            };
        }

        is_repository_error!(svc.update_profile(
            &id,
            UserProfileUpdate {
                preferred_audio: None,
                preferred_subtitle: None,
                max_content_rating: None,
                concurrent_stream_limit: None,
                bitrate_cap: None,
            }
        ));
        is_repository_error!(svc.delete(&boss, &id));
        is_repository_error!(svc.set_active(&boss, &id, false));
        is_repository_error!(svc.library_access(&id));
        is_repository_error!(svc.set_library_access(&id, &[LibraryId("lib1".into())]));
    }
}
