use std::future::Future;

use crate::error::StillError;

pub trait StillExtractor {
    fn extract(
        &self,
        input_path: &str,
        offset_ms: u64,
    ) -> impl Future<Output = Result<Vec<u8>, StillError>> + Send;
}
