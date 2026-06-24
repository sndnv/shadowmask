use std::future::Future;

use crate::catalog::VersionId;
use crate::error::ProbeError;
use crate::media::ProbeResult;

pub trait MediaProbe {
    fn probe(
        &self,
        path: &str,
        version: &VersionId,
    ) -> impl Future<Output = Result<ProbeResult, ProbeError>> + Send;
}
