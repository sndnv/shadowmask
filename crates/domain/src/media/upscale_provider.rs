use std::future::Future;

use crate::error::UpscaleError;
use crate::media::{UpscaleOutput, UpscaleRequest};

pub trait UpscaleProvider {
    fn upscale(
        &self,
        request: &UpscaleRequest,
    ) -> impl Future<Output = Result<UpscaleOutput, UpscaleError>> + Send;
}
