use crate::error::StreamTokenError;
use crate::session::{StreamClaims, StreamToken};

pub trait StreamTokens {
    fn create(&self, claims: &StreamClaims) -> Result<StreamToken, StreamTokenError>;
    fn verify(&self, token: &str) -> Result<StreamClaims, StreamTokenError>;
}
