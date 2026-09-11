use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use jiff::Timestamp;

use domain::catalog::VersionId;
use domain::error::DownloadTokenError;
use domain::session::{DownloadClaims, DownloadToken, DownloadTokens};
use domain::user::UserId;

const SEP: char = '~';

#[derive(Clone, Default)]
pub struct MockDownloadTokens {
    fail: Arc<AtomicBool>,
}

impl MockDownloadTokens {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_fail(&self) {
        self.fail.store(true, Ordering::Relaxed);
    }
}

impl DownloadTokens for MockDownloadTokens {
    fn create(&self, claims: &DownloadClaims) -> Result<DownloadToken, DownloadTokenError> {
        if self.fail.load(Ordering::Relaxed) {
            return Err(DownloadTokenError::Create("mock token failure".to_owned()));
        }
        Ok(DownloadToken(format!(
            "{}{SEP}{}{SEP}{}{SEP}{}",
            claims.user.0,
            claims.version.0,
            claims.expires_at.as_second(),
            claims.nonce,
        )))
    }

    fn verify(&self, token: &str) -> Result<DownloadClaims, DownloadTokenError> {
        let parts: Vec<&str> = token.split(SEP).collect();
        let [user, version, exp, nonce] = parts.as_slice() else {
            return Err(DownloadTokenError::Invalid);
        };
        let exp = exp.parse::<i64>().map_err(|_| DownloadTokenError::Invalid)?;
        let expires_at = Timestamp::from_second(exp).map_err(|_| DownloadTokenError::Invalid)?;
        if expires_at < Timestamp::now() {
            return Err(DownloadTokenError::Expired);
        }
        Ok(DownloadClaims {
            user: UserId((*user).to_owned()),
            version: VersionId((*version).to_owned()),
            expires_at,
            nonce: (*nonce).to_owned(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn claims(nonce: &str) -> DownloadClaims {
        DownloadClaims {
            user: UserId("u1".to_owned()),
            version: VersionId("v1".to_owned()),
            expires_at: Timestamp::from_second(Timestamp::now().as_second() + 3600).unwrap(),
            nonce: nonce.to_owned(),
        }
    }

    #[test]
    fn create_then_verify_roundtrips() {
        let tokens = MockDownloadTokens::new();
        let token = tokens.create(&claims("n1")).unwrap();
        assert_eq!(tokens.verify(&token.0).unwrap(), claims("n1"));
    }

    #[test]
    fn nonce_changes_the_token() {
        let tokens = MockDownloadTokens::new();
        let a = tokens.create(&claims("n1")).unwrap();
        let b = tokens.create(&claims("n2")).unwrap();
        assert_ne!(a.0, b.0);
    }

    #[test]
    fn verify_rejects_malformed_tokens() {
        let tokens = MockDownloadTokens::new();
        assert!(matches!(tokens.verify("too-few-fields"), Err(DownloadTokenError::Invalid)));
        let bad_exp = format!("u1{SEP}v1{SEP}not-a-number{SEP}n1");
        assert!(matches!(tokens.verify(&bad_exp), Err(DownloadTokenError::Invalid)));
        let out_of_range = format!("u1{SEP}v1{SEP}{}{SEP}n1", i64::MAX);
        assert!(matches!(tokens.verify(&out_of_range), Err(DownloadTokenError::Invalid)));
    }

    #[test]
    fn verify_rejects_an_expired_token() {
        let tokens = MockDownloadTokens::new();
        let past = Timestamp::now().as_second() - 3600;
        let expired = format!("u1{SEP}v1{SEP}{past}{SEP}n1");
        assert!(matches!(tokens.verify(&expired), Err(DownloadTokenError::Expired)));
    }

    #[test]
    fn create_failure_maps_to_error() {
        let tokens = MockDownloadTokens::new();
        tokens.set_fail();
        assert!(matches!(tokens.create(&claims("n1")), Err(DownloadTokenError::Create(_))));
    }
}
