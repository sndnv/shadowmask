use domain::catalog::TitleListFilter;
use domain::error::RepositoryError;
use domain::library::LibraryId;
use domain::metadata::ContentRating;
use domain::repository::UserRepository;
use domain::user::{Principal, Role, User, UserId};

pub fn is_admin(caller: &Principal) -> bool {
    caller.role == Role::Admin
}

pub fn is_automation(caller: &Principal) -> bool {
    caller.role == Role::Automation
}

pub fn can_access_library(access: &[LibraryId], library: &LibraryId) -> bool {
    access.iter().any(|granted| granted == library)
}

pub fn rating_permits(cap: Option<&ContentRating>, item: Option<&ContentRating>) -> bool {
    match (
        cap.and_then(ContentRating::age_floor),
        item.and_then(ContentRating::age_floor),
    ) {
        (Some(cap), Some(item)) => item <= cap,
        _ => true,
    }
}

pub struct Viewer {
    admin: bool,
    cap: Option<ContentRating>,
    access: Vec<LibraryId>,
}

pub async fn viewer<U>(users: &U, user: &UserId) -> Result<Viewer, RepositoryError>
where
    U: UserRepository + Sync,
{
    let account = users.get(user).await?;
    let admin = account.as_ref().is_some_and(|a| a.role == Role::Admin);
    entitlements(users, user, admin, account).await
}

pub async fn caller_viewer<U>(users: &U, caller: &Principal) -> Result<Viewer, RepositoryError>
where
    U: UserRepository + Sync,
{
    let account = users.get(&caller.user).await?;
    entitlements(users, &caller.user, is_admin(caller), account).await
}

async fn entitlements<U>(
    users: &U,
    user: &UserId,
    admin: bool,
    account: Option<User>,
) -> Result<Viewer, RepositoryError>
where
    U: UserRepository + Sync,
{
    let access = if admin {
        Vec::new()
    } else {
        users
            .list_library_access(user)
            .await?
            .into_iter()
            .map(|entry| entry.library)
            .collect()
    };
    Ok(Viewer {
        admin,
        cap: account.and_then(|account| account.max_content_rating),
        access,
    })
}

impl Viewer {
    pub fn is_admin(&self) -> bool {
        self.admin
    }

    pub fn permits(&self, rating: Option<&ContentRating>) -> bool {
        self.admin || rating_permits(self.cap.as_ref(), rating)
    }

    pub fn sees_library(&self, library: &LibraryId) -> bool {
        self.admin || can_access_library(&self.access, library)
    }

    pub fn filter(&self, library: Option<&LibraryId>) -> TitleListFilter {
        if self.admin {
            return TitleListFilter {
                libraries: library.map(|lib| vec![lib.clone()]),
                ..TitleListFilter::default()
            };
        }
        let libraries = match library {
            Some(lib) if can_access_library(&self.access, lib) => vec![lib.clone()],
            Some(_) => Vec::new(),
            None => self.access.clone(),
        };
        TitleListFilter {
            libraries: Some(libraries),
            blocked_ratings: ContentRating::blocked_by(self.cap.as_ref()),
            ..TitleListFilter::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::user::UserId;
    use jiff::Timestamp;
    use mocks::MockUserRepo;

    fn account(id: &str, role: Role, cap: Option<ContentRating>) -> User {
        User {
            id: UserId(id.to_owned()),
            username: id.to_owned(),
            password_hash: "hash".into(),
            role,
            max_content_rating: cap,
            preferred_audio: Vec::new(),
            preferred_subtitle: Vec::new(),
            concurrent_stream_limit: None,
            bitrate_cap: None,
            active: true,
            created_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
        }
    }

    async fn repo_with(user: User, libraries: &[&str]) -> MockUserRepo {
        let id = user.id.clone();
        let users = MockUserRepo::new();
        users.insert(user);
        let grants: Vec<LibraryId> = libraries
            .iter()
            .map(|lib| LibraryId((*lib).to_owned()))
            .collect();
        users.set_library_access(&id, &grants).await.unwrap();
        users
    }

    fn principal(role: Role) -> Principal {
        Principal {
            user: UserId("u1".to_owned()),
            role,
        }
    }

    fn rating(system: &str, code: &str) -> ContentRating {
        ContentRating {
            system: system.to_owned(),
            code: code.to_owned(),
        }
    }

    #[test]
    fn admin_detection() {
        assert!(is_admin(&principal(Role::Admin)));
        assert!(!is_admin(&principal(Role::User)));
        assert!(!is_admin(&principal(Role::Player)));
        assert!(!is_admin(&principal(Role::Automation)));
    }

    #[test]
    fn automation_detection() {
        assert!(is_automation(&principal(Role::Automation)));
        assert!(!is_automation(&principal(Role::Admin)));
        assert!(!is_automation(&principal(Role::User)));
        assert!(!is_automation(&principal(Role::Player)));
    }

    #[test]
    fn library_access_membership() {
        let access = [LibraryId("lib1".into()), LibraryId("lib2".into())];
        assert!(can_access_library(&access, &LibraryId("lib1".into())));
        assert!(!can_access_library(&access, &LibraryId("lib3".into())));
        assert!(!can_access_library(&[], &LibraryId("lib1".into())));
    }

    #[test]
    fn rating_gate_allows_within_cap_and_unknowns() {
        let pg13 = rating("MPAA", "PG-13");
        let r = rating("MPAA", "R");
        let unknown = rating("XYZ", "42");

        assert!(rating_permits(Some(&r), Some(&pg13)));
        assert!(rating_permits(Some(&pg13), Some(&pg13)));
        assert!(!rating_permits(Some(&pg13), Some(&r)));

        assert!(rating_permits(None, Some(&r)));
        assert!(rating_permits(Some(&pg13), None));
        assert!(rating_permits(Some(&unknown), Some(&r)));
        assert!(rating_permits(Some(&pg13), Some(&unknown)));
    }

    #[tokio::test]
    async fn a_capped_viewer_is_scoped_to_granted_libraries_and_blocked_ratings() {
        let pg13 = rating("MPAA", "PG-13");
        let users = repo_with(
            account("u1", Role::User, Some(pg13.clone())),
            &["lib1", "lib2"],
        )
        .await;

        let filter = viewer(&users, &UserId("u1".into()))
            .await
            .unwrap()
            .filter(None);

        assert_eq!(
            filter.libraries,
            Some(vec![LibraryId("lib1".into()), LibraryId("lib2".into())])
        );
        assert!(filter.blocked_ratings.contains(&rating("mpaa", "r")));
        assert!(!filter.blocked_ratings.contains(&pg13));
    }

    #[tokio::test]
    async fn an_admin_viewer_is_scoped_to_nothing_at_all() {
        let users = repo_with(account("boss", Role::Admin, None), &[]).await;

        let filter = viewer(&users, &UserId("boss".into()))
            .await
            .unwrap()
            .filter(None);

        assert_eq!(filter.libraries, None, "an admin sees every library");
        assert!(filter.blocked_ratings.is_empty());
    }

    #[tokio::test]
    async fn requesting_one_library_narrows_a_viewer_who_holds_it() {
        let users = repo_with(account("u1", Role::User, None), &["lib1", "lib2"]).await;

        let filter = viewer(&users, &UserId("u1".into()))
            .await
            .unwrap()
            .filter(Some(&LibraryId("lib2".into())));

        assert_eq!(filter.libraries, Some(vec![LibraryId("lib2".into())]));
    }

    #[tokio::test]
    async fn requesting_an_ungranted_library_yields_nothing_rather_than_everything() {
        let users = repo_with(account("u1", Role::User, None), &["lib1"]).await;

        let filter = viewer(&users, &UserId("u1".into()))
            .await
            .unwrap()
            .filter(Some(&LibraryId("forbidden".into())));

        assert_eq!(
            filter.libraries,
            Some(Vec::new()),
            "an empty scope must not widen into an unscoped query"
        );
    }

    #[tokio::test]
    async fn a_missing_account_fails_closed() {
        let users = MockUserRepo::new();

        let seen = viewer(&users, &UserId("ghost".into())).await.unwrap();

        assert!(!seen.is_admin());
        assert_eq!(seen.filter(None).libraries, Some(Vec::new()));
        assert!(!seen.sees_library(&LibraryId("lib1".into())));
    }

    #[tokio::test]
    async fn a_viewer_gates_ratings_and_libraries_together() {
        let users = repo_with(
            account("u1", Role::User, Some(rating("MPAA", "PG-13"))),
            &["lib1"],
        )
        .await;

        let seen = viewer(&users, &UserId("u1".into())).await.unwrap();

        assert!(seen.permits(Some(&rating("MPAA", "PG"))));
        assert!(!seen.permits(Some(&rating("MPAA", "R"))));
        assert!(seen.sees_library(&LibraryId("lib1".into())));
        assert!(!seen.sees_library(&LibraryId("lib2".into())));
    }

    #[tokio::test]
    async fn an_admin_viewer_bypasses_both_gates() {
        let users = repo_with(account("boss", Role::Admin, Some(rating("MPAA", "G"))), &[]).await;

        let seen = viewer(&users, &UserId("boss".into())).await.unwrap();

        assert!(seen.permits(Some(&rating("MPAA", "R"))));
        assert!(seen.sees_library(&LibraryId("ungranted".into())));
    }

    #[tokio::test]
    async fn the_caller_view_reads_the_token_role_and_the_target_view_reads_the_row() {
        let users = repo_with(account("u1", Role::User, Some(rating("MPAA", "G"))), &[]).await;
        let stale = Principal {
            user: UserId("u1".into()),
            role: Role::Admin,
        };

        let as_caller = caller_viewer(&users, &stale).await.unwrap();
        let as_target = viewer(&users, &UserId("u1".into())).await.unwrap();

        assert!(
            as_caller.is_admin(),
            "a caller is judged by the role the request carries"
        );
        assert!(
            !as_target.is_admin(),
            "a target account is judged by its own row, which is what /users/{{id}}/hub needs"
        );
    }
}
