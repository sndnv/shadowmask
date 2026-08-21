use crate::error::DownloadTokenError;
use crate::session::{DownloadClaims, DownloadToken};

pub trait DownloadTokens {
    fn create(&self, claims: &DownloadClaims) -> Result<DownloadToken, DownloadTokenError>;
    fn verify(&self, token: &str) -> Result<DownloadClaims, DownloadTokenError>;
}
