use jiff::Timestamp;
use serde::{Deserialize, Serialize};

use domain::catalog::VersionId;
use domain::error::StreamTokenError;
use domain::session::{SessionId, StreamClaims, StreamToken, StreamTokens};
use domain::user::UserId;

use crate::hmac_token::{HmacCodec, TokenFailure, TypedClaims};

const STREAM_AUDIENCE: &str = "shadowmask-stream";
const STREAM_TOKEN_TYPE: &str = "stream";

pub struct HmacStreamTokens {
    codec: HmacCodec,
}

impl HmacStreamTokens {
    pub fn new(secret: &[u8]) -> Self {
        Self {
            codec: HmacCodec::new(secret, STREAM_AUDIENCE),
        }
    }
}

#[derive(Serialize, Deserialize)]
struct RawClaims {
    sub: String,
    sid: String,
    vid: String,
    aud: String,
    typ: String,
    exp: i64,
    nnc: String,
}

impl TypedClaims for RawClaims {
    fn typ(&self) -> &str {
        &self.typ
    }
}

impl StreamTokens for HmacStreamTokens {
    fn create(&self, claims: &StreamClaims) -> Result<StreamToken, StreamTokenError> {
        let raw = RawClaims {
            sub: claims.user.0.clone(),
            sid: claims.session.0.clone(),
            vid: claims.version.0.clone(),
            aud: STREAM_AUDIENCE.to_owned(),
            typ: STREAM_TOKEN_TYPE.to_owned(),
            exp: claims.expires_at.as_second(),
            nnc: claims.nonce.clone(),
        };
        self.codec
            .sign(&raw)
            .map(StreamToken)
            .map_err(StreamTokenError::Create)
    }

    fn verify(&self, token: &str) -> Result<StreamClaims, StreamTokenError> {
        let raw: RawClaims = self
            .codec
            .verify(token, STREAM_TOKEN_TYPE)
            .map_err(|failure| match failure {
                TokenFailure::Expired => StreamTokenError::Expired,
                TokenFailure::Invalid => StreamTokenError::Invalid,
            })?;
        let expires_at = Timestamp::from_second(raw.exp).map_err(|_| StreamTokenError::Invalid)?;
        Ok(StreamClaims {
            session: SessionId(raw.sid),
            user: UserId(raw.sub),
            version: VersionId(raw.vid),
            expires_at,
            nonce: raw.nnc,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};

    const SECRET: &[u8] = b"shadowmask-test-secret";

    fn claims_expiring_in(secs: i64) -> StreamClaims {
        StreamClaims {
            session: SessionId("session-1".into()),
            user: UserId("user-1".into()),
            version: VersionId("version-1".into()),
            expires_at: Timestamp::from_second(Timestamp::now().as_second() + secs).unwrap(),
            nonce: "nonce-1".into(),
        }
    }

    #[test]
    fn create_then_verify_roundtrips() {
        let codec = HmacStreamTokens::new(SECRET);
        let claims = claims_expiring_in(3600);
        let token = codec.create(&claims).unwrap();
        let verified = codec.verify(&token.0).unwrap();
        assert_eq!(verified, claims);
    }

    #[test]
    fn verify_rejects_expired_token() {
        let codec = HmacStreamTokens::new(SECRET);
        let token = codec.create(&claims_expiring_in(-3600)).unwrap();
        assert!(matches!(
            codec.verify(&token.0),
            Err(StreamTokenError::Expired)
        ));
    }

    #[test]
    fn verify_rejects_wrong_secret() {
        let issuer = HmacStreamTokens::new(SECRET);
        let other = HmacStreamTokens::new(b"a-different-secret");
        let token = issuer.create(&claims_expiring_in(3600)).unwrap();
        assert!(matches!(
            other.verify(&token.0),
            Err(StreamTokenError::Invalid)
        ));
    }

    #[test]
    fn verify_rejects_tampered_token() {
        let codec = HmacStreamTokens::new(SECRET);
        let mut bytes = codec
            .create(&claims_expiring_in(3600))
            .unwrap()
            .0
            .into_bytes();
        let last = bytes.len() - 1;
        bytes[last] = if bytes[last] == b'a' { b'b' } else { b'a' };
        let tampered = String::from_utf8(bytes).unwrap();
        assert!(matches!(
            codec.verify(&tampered),
            Err(StreamTokenError::Invalid)
        ));
    }

    #[test]
    fn verify_rejects_garbage() {
        let codec = HmacStreamTokens::new(SECRET);
        assert!(matches!(
            codec.verify("not.a.jwt"),
            Err(StreamTokenError::Invalid)
        ));
    }

    #[test]
    fn verify_rejects_out_of_range_expiry() {
        let codec = HmacStreamTokens::new(SECRET);
        let raw = RawClaims {
            sub: "user-1".into(),
            sid: "session-1".into(),
            vid: "version-1".into(),
            aud: STREAM_AUDIENCE.into(),
            typ: STREAM_TOKEN_TYPE.into(),
            exp: i64::MAX,
            nnc: String::new(),
        };
        let token = encode(
            &Header::new(Algorithm::HS256),
            &raw,
            &EncodingKey::from_secret(SECRET),
        )
        .unwrap();
        assert!(matches!(
            codec.verify(&token),
            Err(StreamTokenError::Invalid)
        ));
    }

    fn encode_raw(raw: &RawClaims) -> String {
        encode(
            &Header::new(Algorithm::HS256),
            raw,
            &EncodingKey::from_secret(SECRET),
        )
        .unwrap()
    }

    #[test]
    fn verify_rejects_wrong_audience() {
        let codec = HmacStreamTokens::new(SECRET);
        let token = encode_raw(&RawClaims {
            sub: "user-1".into(),
            sid: "session-1".into(),
            vid: "version-1".into(),
            aud: "shadowmask-rest".into(),
            typ: STREAM_TOKEN_TYPE.into(),
            exp: Timestamp::now().as_second() + 3600,
            nnc: String::new(),
        });
        assert!(matches!(
            codec.verify(&token),
            Err(StreamTokenError::Invalid)
        ));
    }

    #[test]
    fn verify_rejects_wrong_token_type() {
        let codec = HmacStreamTokens::new(SECRET);
        let token = encode_raw(&RawClaims {
            sub: "user-1".into(),
            sid: "session-1".into(),
            vid: "version-1".into(),
            aud: STREAM_AUDIENCE.into(),
            typ: "access".into(),
            exp: Timestamp::now().as_second() + 3600,
            nnc: String::new(),
        });
        assert!(matches!(
            codec.verify(&token),
            Err(StreamTokenError::Invalid)
        ));
    }
}

#[cfg(test)]
mod prop_tests {
    use super::*;
    use proptest::prelude::*;

    prop_compose! {
        fn future_claims()(
            sub in "\\PC{0,20}",
            sid in "\\PC{0,20}",
            vid in "\\PC{0,20}",
            nnc in "\\PC{0,20}",
            offset in 3600i64..=1_000_000,
        ) -> StreamClaims {
            StreamClaims {
                session: SessionId(sid),
                user: UserId(sub),
                version: VersionId(vid),
                expires_at: Timestamp::from_second(Timestamp::now().as_second() + offset).unwrap(),
                nonce: nnc,
            }
        }
    }

    proptest! {
        #[test]
        fn roundtrips_future_claims(claims in future_claims()) {
            let codec = HmacStreamTokens::new(b"prop-secret");
            let token = codec.create(&claims).unwrap();
            prop_assert_eq!(codec.verify(&token.0).unwrap(), claims);
        }

        #[test]
        fn wrong_secret_is_invalid(claims in future_claims()) {
            let issuer = HmacStreamTokens::new(b"prop-secret");
            let other = HmacStreamTokens::new(b"a-different-secret");
            let token = issuer.create(&claims).unwrap();
            prop_assert!(matches!(
                other.verify(&token.0),
                Err(StreamTokenError::Invalid)
            ));
        }

        #[test]
        fn past_expiry_is_expired(
            sub in "\\PC{0,20}",
            sid in "\\PC{0,20}",
            vid in "\\PC{0,20}",
            ago in 3600i64..=1_000_000,
        ) {
            let codec = HmacStreamTokens::new(b"prop-secret");
            let claims = StreamClaims {
                session: SessionId(sid),
                user: UserId(sub),
                version: VersionId(vid),
                expires_at: Timestamp::from_second(Timestamp::now().as_second() - ago).unwrap(),
                nonce: String::new(),
            };
            let token = codec.create(&claims).unwrap();
            prop_assert!(matches!(
                codec.verify(&token.0),
                Err(StreamTokenError::Expired)
            ));
        }
    }
}
