use jiff::Timestamp;
use jsonwebtoken::errors::ErrorKind;
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};

use domain::catalog::VersionId;
use domain::error::StreamTokenError;
use domain::session::{SessionId, StreamClaims, StreamToken, StreamTokens};
use domain::user::UserId;

const STREAM_AUDIENCE: &str = "shadowmask-stream";
const STREAM_TOKEN_TYPE: &str = "stream";

pub struct HmacStreamTokens {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    header: Header,
    validation: Validation,
}

impl HmacStreamTokens {
    pub fn new(secret: &[u8]) -> Self {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.set_audience(&[STREAM_AUDIENCE]);
        Self {
            encoding_key: EncodingKey::from_secret(secret),
            decoding_key: DecodingKey::from_secret(secret),
            header: Header::new(Algorithm::HS256),
            validation,
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
        };
        encode(&self.header, &raw, &self.encoding_key)
            .map(StreamToken)
            .map_err(|e| StreamTokenError::Create(e.to_string()))
    }

    fn verify(&self, token: &str) -> Result<StreamClaims, StreamTokenError> {
        let raw = decode::<RawClaims>(token, &self.decoding_key, &self.validation)
            .map_err(|e| match e.kind() {
                ErrorKind::ExpiredSignature => StreamTokenError::Expired,
                _ => StreamTokenError::Invalid,
            })?
            .claims;
        if raw.typ != STREAM_TOKEN_TYPE {
            return Err(StreamTokenError::Invalid);
        }
        let expires_at = Timestamp::from_second(raw.exp).map_err(|_| StreamTokenError::Invalid)?;
        Ok(StreamClaims {
            session: SessionId(raw.sid),
            user: UserId(raw.sub),
            version: VersionId(raw.vid),
            expires_at,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECRET: &[u8] = b"shadowmask-test-secret";

    fn claims_expiring_in(secs: i64) -> StreamClaims {
        StreamClaims {
            session: SessionId("session-1".into()),
            user: UserId("user-1".into()),
            version: VersionId("version-1".into()),
            expires_at: Timestamp::from_second(Timestamp::now().as_second() + secs).unwrap(),
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
        });
        assert!(matches!(
            codec.verify(&token),
            Err(StreamTokenError::Invalid)
        ));
    }
}
