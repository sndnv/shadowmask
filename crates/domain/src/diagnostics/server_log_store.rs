use std::future::Future;

use crate::diagnostics::{ServerLogPage, ServerLogRequest};
use crate::error::ServerLogError;

pub trait ServerLogStore: Send + Sync {
    fn read(
        &self,
        request: ServerLogRequest,
    ) -> impl Future<Output = Result<ServerLogPage, ServerLogError>> + Send;
}
