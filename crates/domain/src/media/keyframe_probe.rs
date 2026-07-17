use std::future::Future;

use crate::error::ProbeError;

pub trait KeyframeProbe: Send + Sync {
    fn keyframes(&self, path: &str) -> impl Future<Output = Result<Vec<u64>, ProbeError>> + Send;
}
