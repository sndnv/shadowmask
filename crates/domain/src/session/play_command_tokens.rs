use crate::error::PlayCommandTokenError;
use crate::session::{PlayCommandClaims, PlayCommandToken};

pub trait PlayCommandTokens {
    fn create(&self, claims: &PlayCommandClaims)
    -> Result<PlayCommandToken, PlayCommandTokenError>;
    fn verify(&self, token: &str) -> Result<PlayCommandClaims, PlayCommandTokenError>;
}
