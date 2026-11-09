use std::future::Future;

use crate::error::ArtworkError;
use crate::metadata::{ArtworkSpec, ProcessedArtwork};

pub trait ArtworkPipeline {
    fn process_all(
        &self,
        url: &str,
        specs: &[ArtworkSpec],
    ) -> impl Future<Output = Result<Vec<ProcessedArtwork>, ArtworkError>> + Send;

    fn process_jpeg(
        &self,
        url: &str,
        specs: &[ArtworkSpec],
    ) -> impl Future<Output = Result<Vec<ProcessedArtwork>, ArtworkError>> + Send;

    fn process_bytes(
        &self,
        bytes: Vec<u8>,
        specs: &[ArtworkSpec],
    ) -> impl Future<Output = Result<Vec<ProcessedArtwork>, ArtworkError>> + Send;
}
