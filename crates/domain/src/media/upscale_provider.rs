use std::future::Future;

use crate::error::UpscaleError;
use crate::media::{UpscaleOutput, UpscaleSpec};

pub trait UpscaleProvider {
    fn upscale(
        &self,
        request: &UpscaleSpec,
    ) -> impl Future<Output = Result<UpscaleOutput, UpscaleError>> + Send;
}
