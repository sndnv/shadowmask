pub mod admin;
pub mod auth;
pub mod catalog;
pub mod discovery;
pub mod image;
pub mod job_log;
pub mod library;
pub mod server;
pub mod sessions;
pub mod stream;
pub mod trickplay;
pub mod user_library;
pub mod users;
pub mod webhook;

use std::fmt::Display;

use tracing::debug;

use domain::user::{Principal, Role, UserId};

use crate::error::{ApiError, ApiResult};

pub(crate) fn require_admin_or_self(actor: &Principal, target: &UserId) -> ApiResult<()> {
    if actor.role == Role::Admin || &actor.user == target {
        Ok(())
    } else {
        Err(ApiError::forbidden("not permitted for this user"))
    }
}

pub(crate) fn deny_player(actor: &Principal) -> ApiResult<()> {
    if actor.role == Role::Player {
        Err(ApiError::forbidden(
            "player sessions can only play; manage your account from a full session",
        ))
    } else {
        Ok(())
    }
}

pub(crate) fn log_fail<'a, E: Display>(
    actor: &'a str,
    action: &'a str,
) -> impl FnOnce(E) -> E + 'a {
    move |err| {
        debug!("User [{actor}] failed to {action}: {err}");
        err
    }
}
