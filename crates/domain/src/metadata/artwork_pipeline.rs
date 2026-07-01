use std::future::Future;

use crate::error::ArtworkError;
use crate::metadata::{ArtworkSpec, ProcessedArtwork};

pub trait ArtworkPipeline {
    fn process(
        &self,
        url: &str,
        spec: ArtworkSpec,
    ) -> impl Future<Output = Result<ProcessedArtwork, ArtworkError>> + Send;
}
