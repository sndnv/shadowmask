use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use jiff::Timestamp;

use domain::catalog::VersionId;
use domain::error::StreamTokenError;
use domain::session::{SessionId, StreamClaims, StreamGeneration, StreamToken, StreamTokens};
use domain::user::UserId;

const SEP: char = '\u{1f}';

#[derive(Clone, Default)]
pub struct MockStreamTokens {
    fail: Arc<AtomicBool>,
}

impl MockStreamTokens {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_fail(&self) {
        self.fail.store(true, Ordering::Relaxed);
    }
}

impl StreamTokens for MockStreamTokens {
    fn create(&self, claims: &StreamClaims) -> Result<StreamToken, StreamTokenError> {
        if self.fail.load(Ordering::Relaxed) {
            return Err(StreamTokenError::Create("mock token failure".to_owned()));
        }
        Ok(StreamToken(format!(
            "{}{SEP}{}{SEP}{}{SEP}{}{SEP}{}{SEP}{}",
            claims.session.0,
            claims.generation.0,
            claims.user.0,
            claims.version.0,
            claims.expires_at.as_second(),
            claims.nonce,
        )))
    }

    fn verify(&self, token: &str) -> Result<StreamClaims, StreamTokenError> {
        let parts: Vec<&str> = token.split(SEP).collect();
        let [session, generation, user, version, exp, nonce] = parts.as_slice() else {
            return Err(StreamTokenError::Invalid);
        };
        let generation = generation
            .parse::<u32>()
            .map_err(|_| StreamTokenError::Invalid)?;
        let exp = exp.parse::<i64>().map_err(|_| StreamTokenError::Invalid)?;
        let expires_at = Timestamp::from_second(exp).map_err(|_| StreamTokenError::Invalid)?;
        Ok(StreamClaims {
            session: SessionId((*session).to_owned()),
            generation: StreamGeneration(generation),
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

    fn claims(nonce: &str) -> StreamClaims {
        StreamClaims {
            session: SessionId("s1".to_owned()),
            generation: StreamGeneration(2),
            user: UserId("u1".to_owned()),
            version: VersionId("v1".to_owned()),
            expires_at: Timestamp::from_second(1_700_000_000).unwrap(),
            nonce: nonce.to_owned(),
        }
    }

    #[test]
    fn create_then_verify_roundtrips() {
        let tokens = MockStreamTokens::new();
        let token = tokens.create(&claims("n1")).unwrap();
        assert_eq!(tokens.verify(&token.0).unwrap(), claims("n1"));
    }

    #[test]
    fn nonce_changes_the_token() {
        let tokens = MockStreamTokens::new();
        let a = tokens.create(&claims("n1")).unwrap();
        let b = tokens.create(&claims("n2")).unwrap();
        assert_ne!(a.0, b.0);
    }

    #[test]
    fn verify_rejects_malformed_tokens() {
        let tokens = MockStreamTokens::new();
        assert!(matches!(
            tokens.verify("too-few-fields"),
            Err(StreamTokenError::Invalid)
        ));
        let bad_exp = format!("s1{SEP}2{SEP}u1{SEP}v1{SEP}not-a-number{SEP}n1");
        assert!(matches!(
            tokens.verify(&bad_exp),
            Err(StreamTokenError::Invalid)
        ));
        let out_of_range = format!("s1{SEP}2{SEP}u1{SEP}v1{SEP}{}{SEP}n1", i64::MAX);
        assert!(matches!(
            tokens.verify(&out_of_range),
            Err(StreamTokenError::Invalid)
        ));
        let bad_generation = format!("s1{SEP}not-a-number{SEP}u1{SEP}v1{SEP}1700000000{SEP}n1");
        assert!(matches!(
            tokens.verify(&bad_generation),
            Err(StreamTokenError::Invalid)
        ));
    }

    #[test]
    fn create_failure_maps_to_error() {
        let tokens = MockStreamTokens::new();
        tokens.set_fail();
        assert!(matches!(
            tokens.create(&claims("n1")),
            Err(StreamTokenError::Create(_))
        ));
    }
}
