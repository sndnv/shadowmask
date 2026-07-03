use std::future::Future;

use crate::error::ProbeError;
use crate::media::ProbeResult;

pub trait MediaProbe {
    fn probe(&self, path: &str) -> impl Future<Output = Result<ProbeResult, ProbeError>> + Send;
}
