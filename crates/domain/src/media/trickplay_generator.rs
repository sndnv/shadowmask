use std::future::Future;

use crate::catalog::VersionId;
use crate::error::TrickplayError;
use crate::media::TrickplayAsset;

pub trait TrickplayGenerator {
    fn generate(
        &self,
        input_path: &str,
        version: &VersionId,
        duration_ms: u64,
    ) -> impl Future<Output = Result<TrickplayAsset, TrickplayError>> + Send;
}
