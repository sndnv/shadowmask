use std::future::Future;

use crate::error::SubtitleError;

pub trait SubtitleReader {
    fn load(&self, path: &str) -> impl Future<Output = Result<String, SubtitleError>> + Send;
}
