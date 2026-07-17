use ::api::dto::user::RoleDto;
use domain::common::LanguageCode;
use domain::error::UserError;
use domain::library::LibraryId;
use domain::metadata::ContentRating;
use domain::service::{LibraryService, UserService};
use domain::user::{NewUser, Principal, UserProfileUpdate};
use serde::Deserialize;

use super::executor::BootstrapEntityProvider;
use super::{BootstrapError, Created, backend, bootstrap_admin, require_unique};

#[derive(Debug, Deserialize)]
struct ContentRatingEntry {
    system: String,
    code: String,
}

impl From<ContentRatingEntry> for ContentRating {
    fn from(entry: ContentRatingEntry) -> Self {
        ContentRating {
            system: entry.system,
            code: entry.code,
        }
    }
}

#[derive(Debug, Deserialize)]
struct UserEntry {
    username: String,
    password: String,
    role: RoleDto,
    #[serde(default)]
    libraries: Vec<String>,
    #[serde(default)]
    max_content_rating: Option<ContentRatingEntry>,
    #[serde(default)]
    concurrent_stream_limit: Option<u32>,
    #[serde(default)]
    bitrate_cap: Option<u64>,
    #[serde(default)]
    preferred_audio: Option<Vec<String>>,
    #[serde(default)]
    preferred_subtitle: Option<Vec<String>>,
}

pub struct ParsedUser {
    new_user: NewUser,
    libraries: Vec<String>,
    profile: Option<UserProfileUpdate>,
}

fn to_languages(codes: Vec<String>) -> Vec<LanguageCode> {
    codes.into_iter().map(LanguageCode).collect()
}

impl From<UserEntry> for ParsedUser {
    fn from(entry: UserEntry) -> Self {
        let profile = UserProfileUpdate {
            preferred_audio: entry.preferred_audio.map(to_languages),
            preferred_subtitle: entry.preferred_subtitle.map(to_languages),
            max_content_rating: entry.max_content_rating.map(Into::into),
            concurrent_stream_limit: entry.concurrent_stream_limit,
            bitrate_cap: entry.bitrate_cap,
        };
        let has_profile = profile.preferred_audio.is_some()
            || profile.preferred_subtitle.is_some()
            || profile.max_content_rating.is_some()
            || profile.concurrent_stream_limit.is_some()
            || profile.bitrate_cap.is_some();
        ParsedUser {
            new_user: NewUser {
                username: entry.username,
                password: entry.password,
                role: entry.role.into(),
            },
            libraries: entry.libraries,
            profile: has_profile.then_some(profile),
        }
    }
}

pub struct UserBootstrapProvider<U, L> {
    users: U,
    libraries: L,
    admin: Principal,
}

impl<U, L> UserBootstrapProvider<U, L> {
    pub fn new(users: U, libraries: L) -> Self {
        Self {
            users,
            libraries,
            admin: bootstrap_admin(),
        }
    }
}

impl<U, L> UserBootstrapProvider<U, L>
where
    L: LibraryService + Send + Sync,
{
    async fn resolve_libraries(&self, names: &[String]) -> Result<Vec<LibraryId>, BootstrapError> {
        if names.is_empty() {
            return Ok(Vec::new());
        }
        let existing = self
            .libraries
            .libraries(&self.admin)
            .await
            .map_err(|error| backend("user", error))?;
        let mut ids = Vec::with_capacity(names.len());
        for name in names {
            let library = existing
                .iter()
                .find(|library| &library.name == name)
                .ok_or_else(|| BootstrapError::Invalid {
                    entity: "user",
                    reason: format!("references unknown library {name:?}"),
                })?;
            ids.push(library.id.clone());
        }
        Ok(ids)
    }
}

impl<U, L> BootstrapEntityProvider for UserBootstrapProvider<U, L>
where
    U: UserService + Send + Sync,
    L: LibraryService + Send + Sync,
{
    type Entity = ParsedUser;

    fn name(&self) -> &'static str {
        "users"
    }

    fn load(&self, value: &toml::Value) -> Result<ParsedUser, BootstrapError> {
        let entry: UserEntry =
            value
                .clone()
                .try_into()
                .map_err(|error| BootstrapError::Invalid {
                    entity: "user",
                    reason: error.to_string(),
                })?;
        Ok(entry.into())
    }

    fn validate(&self, entities: &[ParsedUser]) -> Result<(), BootstrapError> {
        require_unique(entities, "user", "username", |user| {
            user.new_user.username.clone()
        })
    }

    async fn create(&self, entity: ParsedUser) -> Result<Created, BootstrapError> {
        let ParsedUser {
            new_user,
            libraries,
            profile,
        } = entity;
        let library_ids = self.resolve_libraries(&libraries).await?;

        let user = match self.users.create(new_user).await {
            Ok(user) => user,
            Err(UserError::UsernameTaken) => return Ok(Created::Skipped),
            Err(error) => return Err(backend("user", error)),
        };

        if !library_ids.is_empty() {
            self.users
                .set_library_access(&user.id, &library_ids)
                .await
                .map_err(|error| backend("user", error))?;
        }
        if let Some(update) = profile {
            self.users
                .update_profile(&user.id, update)
                .await
                .map_err(|error| backend("user", error))?;
        }
        Ok(Created::New)
    }

    fn render(&self, entity: &ParsedUser) -> String {
        format!(
            "username={} role={:?} libraries={:?}",
            entity.new_user.username, entity.new_user.role, entity.libraries
        )
    }

    fn extract_id(&self, entity: &ParsedUser) -> String {
        entity.new_user.username.clone()
    }
}

#[cfg(test)]
mod tests {
    use domain::library::{Library, LibraryId, LibraryKind, WatcherStrategy};
    use domain::user::Role;
    use jiff::Timestamp;
    use services::mock::{MockLibraryService, MockUserService};

    use super::super::executor::run_one;
    use super::*;

    fn library(name: &str) -> Library {
        Library {
            id: LibraryId(format!("lib-{name}")),
            name: name.to_owned(),
            kind: LibraryKind::Movie,
            roots: vec!["/media".to_owned()],
            watcher: WatcherStrategy::Local,
            scan_schedule: None,
            metadata_sources: Vec::new(),
            created_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
        }
    }

    fn write(dir: &std::path::Path, body: &str) {
        std::fs::write(dir.join("users.toml"), body).unwrap();
    }

    #[tokio::test]
    async fn creates_admin_with_library_access_and_profile() {
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            "[[users]]\nusername = \"admin\"\npassword = \"secret\"\nrole = \"admin\"\nlibraries = [\"Movies\"]\nconcurrent_stream_limit = 3\n\n[users.max_content_rating]\nsystem = \"mpaa\"\ncode = \"PG-13\"\n",
        );
        let users = MockUserService::new();
        let libraries = MockLibraryService::new();
        libraries.add_library(library("Movies"));
        let provider = UserBootstrapProvider::new(users.clone(), libraries.clone());

        let result = run_one(&provider, dir.path()).await;
        assert_eq!(result.created, 1);

        let listed = users
            .list(domain::common::PageRequest {
                offset: 0,
                limit: 10,
            })
            .await
            .unwrap();
        let admin = &listed.items[0];
        assert_eq!(admin.username, "admin");
        assert_eq!(admin.role, Role::Admin);
        assert_eq!(admin.concurrent_stream_limit, Some(3));
        assert_eq!(
            admin.max_content_rating,
            Some(ContentRating {
                system: "mpaa".to_owned(),
                code: "PG-13".to_owned(),
            })
        );
        let access = users.library_access(&admin.id).await.unwrap();
        assert_eq!(access.len(), 1);
        assert_eq!(access[0].library, LibraryId("lib-Movies".to_owned()));
    }

    #[tokio::test]
    async fn skips_existing_username_without_touching_it() {
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            "[[users]]\nusername = \"admin\"\npassword = \"new\"\nrole = \"admin\"\n",
        );
        let users = MockUserService::new();
        users
            .create(NewUser {
                username: "admin".to_owned(),
                password: "original".to_owned(),
                role: Role::Admin,
            })
            .await
            .unwrap();
        let provider = UserBootstrapProvider::new(users.clone(), MockLibraryService::new());

        let result = run_one(&provider, dir.path()).await;
        assert_eq!(result.created, 0);
        assert_eq!(result.skipped, 1);
        let listed = users
            .list(domain::common::PageRequest {
                offset: 0,
                limit: 10,
            })
            .await
            .unwrap();
        assert_eq!(listed.total, 1);
        assert_eq!(listed.items[0].password_hash, "original");
    }

    #[tokio::test]
    async fn unknown_library_reference_fails_entry() {
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            "[[users]]\nusername = \"admin\"\npassword = \"secret\"\nrole = \"admin\"\nlibraries = [\"Nope\"]\n",
        );
        let users = MockUserService::new();
        let provider = UserBootstrapProvider::new(users.clone(), MockLibraryService::new());

        let result = run_one(&provider, dir.path()).await;
        assert_eq!(result.found, 1);
        assert_eq!(result.created, 0);
        assert!(
            users
                .list(domain::common::PageRequest {
                    offset: 0,
                    limit: 10,
                })
                .await
                .unwrap()
                .items
                .is_empty()
        );
    }

    #[tokio::test]
    async fn duplicate_usernames_fail_validation() {
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            "[[users]]\nusername = \"admin\"\npassword = \"a\"\nrole = \"admin\"\n\n[[users]]\nusername = \"admin\"\npassword = \"b\"\nrole = \"user\"\n",
        );
        let users = MockUserService::new();
        let provider = UserBootstrapProvider::new(users.clone(), MockLibraryService::new());

        let result = run_one(&provider, dir.path()).await;
        assert_eq!(result.created, 0);
    }
}
