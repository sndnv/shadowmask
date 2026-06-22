use axum::extract::{Request, State};
use axum::http::header::AUTHORIZATION;
use axum::middleware::Next;
use axum::response::Response;

use domain::service::AuthService;

use crate::error::ApiError;

pub async fn jwt<S>(
    State(state): State<S>,
    mut request: Request,
    next: Next,
) -> Result<Response, ApiError>
where
    S: AuthService + Clone + Send + Sync + 'static,
{
    let token = bearer(&request).ok_or_else(|| ApiError::unauthorized("missing bearer token"))?;
    let principal = state.authenticate(&token).await?;
    request.extensions_mut().insert(principal);
    Ok(next.run(request).await)
}

fn bearer(request: &Request) -> Option<String> {
    request
        .headers()
        .get(AUTHORIZATION)?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
        .map(str::to_string)
}
