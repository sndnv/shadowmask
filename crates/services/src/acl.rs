use domain::catalog::TitleListFilter;
use domain::error::RepositoryError;
use domain::library::LibraryId;
use domain::metadata::ContentRating;
use domain::repository::UserRepository;
use domain::user::{Principal, Role, UserId};

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
    cap: Option<ContentRating>,
    access: Vec<LibraryId>,
}

pub async fn viewer<U>(users: &U, user: &UserId) -> Result<Viewer, RepositoryError>
where
    U: UserRepository + Sync,
{
    let account = users.get(user).await?;
    let access = users
        .list_library_access(user)
        .await?
        .into_iter()
        .map(|entry| entry.library)
        .collect();
    Ok(Viewer {
        cap: account.and_then(|account| account.max_content_rating),
        access,
    })
}

impl Viewer {
    pub fn permits(&self, rating: Option<&ContentRating>) -> bool {
        rating_permits(self.cap.as_ref(), rating)
    }

    pub fn sees_library(&self, library: &LibraryId) -> bool {
        can_access_library(&self.access, library)
    }

    pub fn filter(&self, library: Option<&LibraryId>) -> TitleListFilter {
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
    use domain::user::{User, UserId};
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
    async fn an_admin_is_scoped_to_their_grants_like_anyone_else() {
        let users = repo_with(account("boss", Role::Admin, None), &["lib1"]).await;

        let filter = viewer(&users, &UserId("boss".into()))
            .await
            .unwrap()
            .filter(None);

        assert_eq!(
            filter.libraries,
            Some(vec![LibraryId("lib1".into())]),
            "admin is permission to act, never permission to see"
        );
    }

    #[tokio::test]
    async fn an_admin_with_no_grants_sees_nothing_rather_than_everything() {
        let users = repo_with(account("boss", Role::Admin, None), &[]).await;

        let filter = viewer(&users, &UserId("boss".into()))
            .await
            .unwrap()
            .filter(None);

        assert_eq!(
            filter.libraries,
            Some(Vec::new()),
            "the old bypass returned an empty access list, which must not read as unscoped"
        );
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
    async fn an_admin_is_gated_by_both_their_grants_and_their_cap() {
        let users = repo_with(
            account("boss", Role::Admin, Some(rating("MPAA", "G"))),
            &["lib1"],
        )
        .await;

        let seen = viewer(&users, &UserId("boss".into())).await.unwrap();

        assert!(!seen.permits(Some(&rating("MPAA", "R"))));
        assert!(seen.sees_library(&LibraryId("lib1".into())));
        assert!(!seen.sees_library(&LibraryId("ungranted".into())));
    }

    #[tokio::test]
    async fn a_view_is_the_same_whoever_asks_for_it() {
        let users = repo_with(account("u1", Role::User, None), &["lib1"]).await;
        let elevated = Principal {
            user: UserId("u1".into()),
            role: Role::Admin,
        };

        let as_caller = viewer(&users, &elevated.user).await.unwrap();
        let as_target = viewer(&users, &UserId("u1".into())).await.unwrap();

        assert_eq!(
            as_caller.filter(None).libraries,
            as_target.filter(None).libraries,
            "the role on the request must not change what the account can see, or home and \
             the catalog answer differently for the same person"
        );
    }
}
