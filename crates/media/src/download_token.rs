use jiff::Timestamp;
use serde::{Deserialize, Serialize};

use domain::catalog::VersionId;
use domain::error::DownloadTokenError;
use domain::session::{DownloadClaims, DownloadToken, DownloadTokens};
use domain::user::UserId;

use crate::hmac_token::{HmacCodec, TokenFailure, TypedClaims};

const DOWNLOAD_AUDIENCE: &str = "shadowmask-download";
const DOWNLOAD_TOKEN_TYPE: &str = "download";

pub struct HmacDownloadTokens {
    codec: HmacCodec,
}

impl HmacDownloadTokens {
    pub fn new(secret: &[u8]) -> Self {
        Self { codec: HmacCodec::new(secret, DOWNLOAD_AUDIENCE) }
    }
}

#[derive(Serialize, Deserialize)]
struct RawClaims {
    sub: String,
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

impl DownloadTokens for HmacDownloadTokens {
    fn create(&self, claims: &DownloadClaims) -> Result<DownloadToken, DownloadTokenError> {
        let raw = RawClaims {
            sub: claims.user.0.clone(),
            vid: claims.version.0.clone(),
            aud: DOWNLOAD_AUDIENCE.to_owned(),
            typ: DOWNLOAD_TOKEN_TYPE.to_owned(),
            exp: claims.expires_at.as_second(),
            nnc: claims.nonce.clone(),
        };
        self.codec.sign(&raw).map(DownloadToken).map_err(DownloadTokenError::Create)
    }

    fn verify(&self, token: &str) -> Result<DownloadClaims, DownloadTokenError> {
        let raw: RawClaims =
            self.codec.verify(token, DOWNLOAD_TOKEN_TYPE).map_err(|failure| match failure {
                TokenFailure::Expired => DownloadTokenError::Expired,
                TokenFailure::Invalid => DownloadTokenError::Invalid,
            })?;
        let expires_at =
            Timestamp::from_second(raw.exp).map_err(|_| DownloadTokenError::Invalid)?;
        Ok(DownloadClaims {
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

    fn claims_expiring_in(secs: i64) -> DownloadClaims {
        DownloadClaims {
            user: UserId("user-1".into()),
            version: VersionId("version-1".into()),
            expires_at: Timestamp::from_second(Timestamp::now().as_second() + secs).unwrap(),
            nonce: "nonce-1".into(),
        }
    }

    fn encode_raw(raw: &RawClaims) -> String {
        encode(&Header::new(Algorithm::HS256), raw, &EncodingKey::from_secret(SECRET)).unwrap()
    }

    #[test]
    fn create_then_verify_roundtrips() {
        let codec = HmacDownloadTokens::new(SECRET);
        let claims = claims_expiring_in(3600);
        let token = codec.create(&claims).unwrap();
        assert_eq!(codec.verify(&token.0).unwrap(), claims);
    }

    #[test]
    fn a_token_is_safe_to_put_in_a_url() {
        let codec = HmacDownloadTokens::new(SECRET);
        let token = codec.create(&claims_expiring_in(3600)).unwrap();
        assert!(
            token.0.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.')),
            "the link is handed to a browser as a path segment: {}",
            token.0
        );
    }

    #[test]
    fn verify_rejects_expired_token() {
        let codec = HmacDownloadTokens::new(SECRET);
        let token = codec.create(&claims_expiring_in(-3600)).unwrap();
        assert!(matches!(codec.verify(&token.0), Err(DownloadTokenError::Expired)));
    }

    #[test]
    fn verify_rejects_wrong_secret() {
        let issuer = HmacDownloadTokens::new(SECRET);
        let other = HmacDownloadTokens::new(b"a-different-secret");
        let token = issuer.create(&claims_expiring_in(3600)).unwrap();
        assert!(matches!(other.verify(&token.0), Err(DownloadTokenError::Invalid)));
    }

    #[test]
    fn verify_rejects_tampered_token() {
        let codec = HmacDownloadTokens::new(SECRET);
        let mut bytes = codec.create(&claims_expiring_in(3600)).unwrap().0.into_bytes();
        let last = bytes.len() - 1;
        bytes[last] = if bytes[last] == b'a' { b'b' } else { b'a' };
        let tampered = String::from_utf8(bytes).unwrap();
        assert!(matches!(codec.verify(&tampered), Err(DownloadTokenError::Invalid)));
    }

    #[test]
    fn verify_rejects_garbage() {
        let codec = HmacDownloadTokens::new(SECRET);
        assert!(matches!(codec.verify("not.a.jwt"), Err(DownloadTokenError::Invalid)));
    }

    #[test]
    fn verify_rejects_out_of_range_expiry() {
        let codec = HmacDownloadTokens::new(SECRET);
        let token = encode_raw(&RawClaims {
            sub: "user-1".into(),
            vid: "version-1".into(),
            aud: DOWNLOAD_AUDIENCE.into(),
            typ: DOWNLOAD_TOKEN_TYPE.into(),
            exp: i64::MAX,
            nnc: String::new(),
        });
        assert!(matches!(codec.verify(&token), Err(DownloadTokenError::Invalid)));
    }

    #[test]
    fn verify_rejects_wrong_audience() {
        let codec = HmacDownloadTokens::new(SECRET);
        let token = encode_raw(&RawClaims {
            sub: "user-1".into(),
            vid: "version-1".into(),
            aud: "shadowmask-stream".into(),
            typ: DOWNLOAD_TOKEN_TYPE.into(),
            exp: Timestamp::now().as_second() + 3600,
            nnc: String::new(),
        });
        assert!(matches!(codec.verify(&token), Err(DownloadTokenError::Invalid)));
    }

    #[test]
    fn verify_rejects_wrong_token_type() {
        let codec = HmacDownloadTokens::new(SECRET);
        let token = encode_raw(&RawClaims {
            sub: "user-1".into(),
            vid: "version-1".into(),
            aud: DOWNLOAD_AUDIENCE.into(),
            typ: "stream".into(),
            exp: Timestamp::now().as_second() + 3600,
            nnc: String::new(),
        });
        assert!(matches!(codec.verify(&token), Err(DownloadTokenError::Invalid)));
    }

    #[test]
    fn a_stream_token_does_not_verify_as_a_download_token() {
        use domain::session::{SessionId, StreamClaims, StreamGeneration, StreamTokens};

        let streams = crate::stream_token::HmacStreamTokens::new(SECRET);
        let downloads = HmacDownloadTokens::new(SECRET);
        let token = streams
            .create(&StreamClaims {
                session: SessionId("session-1".into()),
                generation: StreamGeneration(0),
                user: UserId("user-1".into()),
                version: VersionId("version-1".into()),
                expires_at: Timestamp::from_second(Timestamp::now().as_second() + 3600).unwrap(),
                nonce: "nonce-1".into(),
            })
            .unwrap();
        assert!(matches!(downloads.verify(&token.0), Err(DownloadTokenError::Invalid)));
    }
}
