use std::path::PathBuf;

use crate::error::StreamError;
use crate::session::StreamClaims;

pub trait StreamSource {
    fn master_playlist(&self, claims: &StreamClaims) -> Result<String, StreamError>;
    fn media_path(
        &self,
        claims: &StreamClaims,
        variant: &str,
        file: &str,
    ) -> Result<PathBuf, StreamError>;
    fn direct_file(&self, claims: &StreamClaims) -> Result<PathBuf, StreamError>;
}
