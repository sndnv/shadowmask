use domain::library::LibraryId;
use domain::metadata::ContentRating;
use domain::user::{Principal, Role};

pub fn is_admin(caller: &Principal) -> bool {
    caller.role == Role::Admin
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

#[cfg(test)]
mod tests {
    use super::*;
    use domain::user::UserId;

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
}
