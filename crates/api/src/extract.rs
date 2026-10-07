use axum::extract::FromRequestParts;
use axum::http::request::Parts;

use domain::user::{Principal, Role};
use services::acl;

use crate::error::ApiError;

#[derive(Debug)]
pub struct AuthUser(pub Principal);

impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<Principal>()
            .cloned()
            .map(AuthUser)
            .ok_or_else(|| ApiError::unauthorized("missing authentication"))
    }
}

#[derive(Debug)]
pub struct RequireAdmin(pub Principal);

impl<S> FromRequestParts<S> for RequireAdmin
where
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let principal = AuthUser::from_request_parts(parts, state).await?.0;
        if principal.role == Role::Admin {
            Ok(RequireAdmin(principal))
        } else {
            Err(ApiError::forbidden("admin role required"))
        }
    }
}

#[derive(Debug)]
pub struct RequireVersionWork(pub Principal);

impl<S> FromRequestParts<S> for RequireVersionWork
where
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let principal = AuthUser::from_request_parts(parts, state).await?.0;
        if acl::works_on_versions(&principal) {
            Ok(RequireVersionWork(principal))
        } else {
            Err(ApiError::forbidden("admin role or an admin's linked device required"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{Request, StatusCode};
    use axum::response::IntoResponse;
    use domain::user::UserId;

    fn parts_with(principal: Option<Principal>) -> Parts {
        let mut request = Request::new(());
        if let Some(principal) = principal {
            request.extensions_mut().insert(principal);
        }
        request.into_parts().0
    }

    fn principal(role: Role) -> Principal {
        Principal { user: UserId("u1".into()), role, account_admin: role == Role::Admin }
    }

    #[tokio::test]
    async fn version_work_takes_an_admin_or_an_admins_device() {
        let device = |account_admin| Principal { account_admin, ..principal(Role::Player) };
        for allowed in [principal(Role::Admin), device(true)] {
            let mut parts = parts_with(Some(allowed));
            assert!(RequireVersionWork::from_request_parts(&mut parts, &()).await.is_ok());
        }

        for refused in [device(false), principal(Role::User)] {
            let mut parts = parts_with(Some(refused));
            let forbidden =
                RequireVersionWork::from_request_parts(&mut parts, &()).await.unwrap_err();
            assert_eq!(forbidden.into_response().status(), StatusCode::FORBIDDEN);
        }

        let mut parts = parts_with(None);
        let unauthorized =
            RequireVersionWork::from_request_parts(&mut parts, &()).await.unwrap_err();
        assert_eq!(unauthorized.into_response().status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn auth_user_present_then_missing() {
        let mut parts = parts_with(Some(principal(Role::User)));
        assert!(AuthUser::from_request_parts(&mut parts, &()).await.is_ok());

        let mut parts = parts_with(None);
        let rejection = AuthUser::from_request_parts(&mut parts, &()).await.unwrap_err();
        assert_eq!(rejection.into_response().status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn require_admin_admin_user_and_missing() {
        let mut parts = parts_with(Some(principal(Role::Admin)));
        assert!(RequireAdmin::from_request_parts(&mut parts, &()).await.is_ok());

        let mut parts = parts_with(Some(principal(Role::User)));
        let forbidden = RequireAdmin::from_request_parts(&mut parts, &()).await.unwrap_err();
        assert_eq!(forbidden.into_response().status(), StatusCode::FORBIDDEN);

        let mut parts = parts_with(None);
        let unauthorized = RequireAdmin::from_request_parts(&mut parts, &()).await.unwrap_err();
        assert_eq!(unauthorized.into_response().status(), StatusCode::UNAUTHORIZED);
    }
}
