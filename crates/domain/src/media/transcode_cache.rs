use std::future::Future;

use crate::error::CacheError;

pub trait TranscodeCacheMaintenance {
    fn evict(&self, max_bytes: u64) -> impl Future<Output = Result<u64, CacheError>> + Send;
}
