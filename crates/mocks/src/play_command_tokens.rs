use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use jiff::Timestamp;

use domain::catalog::VersionId;
use domain::error::PlayCommandTokenError;
use domain::session::{PlayCommandClaims, PlayCommandToken, PlayCommandTokens};
use domain::user::{DeviceId, UserId};

const SEP: char = '~';

#[derive(Clone, Default)]
pub struct MockPlayCommandTokens {
    fail: Arc<AtomicBool>,
}

impl MockPlayCommandTokens {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_fail(&self) {
        self.fail.store(true, Ordering::Relaxed);
    }
}

impl PlayCommandTokens for MockPlayCommandTokens {
    fn create(
        &self,
        claims: &PlayCommandClaims,
    ) -> Result<PlayCommandToken, PlayCommandTokenError> {
        if self.fail.load(Ordering::Relaxed) {
            return Err(PlayCommandTokenError::Create("mock token failure".to_owned()));
        }
        Ok(PlayCommandToken(format!(
            "{}{SEP}{}{SEP}{}{SEP}{}{SEP}{}",
            claims.user.0,
            claims.device.0,
            claims.version.0,
            claims.expires_at.as_second(),
            claims.nonce,
        )))
    }

    fn verify(&self, token: &str) -> Result<PlayCommandClaims, PlayCommandTokenError> {
        let parts: Vec<&str> = token.split(SEP).collect();
        let [user, device, version, exp, nonce] = parts.as_slice() else {
            return Err(PlayCommandTokenError::Invalid);
        };
        let exp = exp.parse::<i64>().map_err(|_| PlayCommandTokenError::Invalid)?;
        let expires_at = Timestamp::from_second(exp).map_err(|_| PlayCommandTokenError::Invalid)?;
        if expires_at < Timestamp::now() {
            return Err(PlayCommandTokenError::Expired);
        }
        Ok(PlayCommandClaims {
            user: UserId((*user).to_owned()),
            device: DeviceId((*device).to_owned()),
            version: VersionId((*version).to_owned()),
            expires_at,
            nonce: (*nonce).to_owned(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn claims() -> PlayCommandClaims {
        PlayCommandClaims {
            user: UserId("u1".to_owned()),
            device: DeviceId("d1".to_owned()),
            version: VersionId("v1".to_owned()),
            expires_at: Timestamp::from_second(Timestamp::now().as_second() + 60).unwrap(),
            nonce: "n1".to_owned(),
        }
    }

    #[test]
    fn create_then_verify_roundtrips() {
        let tokens = MockPlayCommandTokens::new();
        let token = tokens.create(&claims()).unwrap();
        assert_eq!(tokens.verify(&token.0).unwrap(), claims());
    }

    #[test]
    fn verify_rejects_malformed_tokens() {
        let tokens = MockPlayCommandTokens::new();
        assert!(matches!(tokens.verify("too-few-fields"), Err(PlayCommandTokenError::Invalid)));
        let bad_exp = format!("u1{SEP}d1{SEP}v1{SEP}not-a-number{SEP}n1");
        assert!(matches!(tokens.verify(&bad_exp), Err(PlayCommandTokenError::Invalid)));
        let out_of_range = format!("u1{SEP}d1{SEP}v1{SEP}{}{SEP}n1", i64::MAX);
        assert!(matches!(tokens.verify(&out_of_range), Err(PlayCommandTokenError::Invalid)));
    }

    #[test]
    fn verify_rejects_an_expired_token() {
        let tokens = MockPlayCommandTokens::new();
        let past = Timestamp::now().as_second() - 3600;
        let expired = format!("u1{SEP}d1{SEP}v1{SEP}{past}{SEP}n1");
        assert!(matches!(tokens.verify(&expired), Err(PlayCommandTokenError::Expired)));
    }

    #[test]
    fn create_failure_maps_to_error() {
        let tokens = MockPlayCommandTokens::new();
        tokens.set_fail();
        assert!(matches!(tokens.create(&claims()), Err(PlayCommandTokenError::Create(_))));
    }
}
