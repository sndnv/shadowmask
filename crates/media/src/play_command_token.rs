use jiff::Timestamp;
use serde::{Deserialize, Serialize};

use domain::catalog::VersionId;
use domain::error::PlayCommandTokenError;
use domain::session::{PlayCommandClaims, PlayCommandToken, PlayCommandTokens};
use domain::user::{DeviceId, UserId};

use crate::hmac_token::{HmacCodec, TokenFailure, TypedClaims};

const PLAY_COMMAND_AUDIENCE: &str = "shadowmask-play-command";
const PLAY_COMMAND_TOKEN_TYPE: &str = "play_command";

pub struct HmacPlayCommandTokens {
    codec: HmacCodec,
}

impl HmacPlayCommandTokens {
    pub fn new(secret: &[u8]) -> Self {
        Self { codec: HmacCodec::new(secret, PLAY_COMMAND_AUDIENCE).without_leeway() }
    }
}

#[derive(Serialize, Deserialize)]
struct RawClaims {
    sub: String,
    did: String,
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

impl PlayCommandTokens for HmacPlayCommandTokens {
    fn create(
        &self,
        claims: &PlayCommandClaims,
    ) -> Result<PlayCommandToken, PlayCommandTokenError> {
        let raw = RawClaims {
            sub: claims.user.0.clone(),
            did: claims.device.0.clone(),
            vid: claims.version.0.clone(),
            aud: PLAY_COMMAND_AUDIENCE.to_owned(),
            typ: PLAY_COMMAND_TOKEN_TYPE.to_owned(),
            exp: claims.expires_at.as_second(),
            nnc: claims.nonce.clone(),
        };
        self.codec.sign(&raw).map(PlayCommandToken).map_err(PlayCommandTokenError::Create)
    }

    fn verify(&self, token: &str) -> Result<PlayCommandClaims, PlayCommandTokenError> {
        let raw: RawClaims =
            self.codec.verify(token, PLAY_COMMAND_TOKEN_TYPE).map_err(|failure| match failure {
                TokenFailure::Expired => PlayCommandTokenError::Expired,
                TokenFailure::Invalid => PlayCommandTokenError::Invalid,
            })?;
        let expires_at =
            Timestamp::from_second(raw.exp).map_err(|_| PlayCommandTokenError::Invalid)?;
        Ok(PlayCommandClaims {
            user: UserId(raw.sub),
            device: DeviceId(raw.did),
            version: VersionId(raw.vid),
            expires_at,
            nonce: raw.nnc,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::session::{DownloadClaims, DownloadTokens};
    use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};

    use crate::download_token::HmacDownloadTokens;

    const SECRET: &[u8] = b"shadowmask-test-secret";

    fn claims_expiring_in(secs: i64) -> PlayCommandClaims {
        PlayCommandClaims {
            user: UserId("user-1".into()),
            device: DeviceId("device-1".into()),
            version: VersionId("version-1".into()),
            expires_at: Timestamp::from_second(Timestamp::now().as_second() + secs).unwrap(),
            nonce: "nonce-1".into(),
        }
    }

    fn raw(aud: &str, typ: &str, exp: i64) -> String {
        let claims = RawClaims {
            sub: "user-1".into(),
            did: "device-1".into(),
            vid: "version-1".into(),
            aud: aud.into(),
            typ: typ.into(),
            exp,
            nnc: String::new(),
        };
        encode(&Header::new(Algorithm::HS256), &claims, &EncodingKey::from_secret(SECRET)).unwrap()
    }

    #[test]
    fn create_then_verify_roundtrips() {
        let codec = HmacPlayCommandTokens::new(SECRET);
        let claims = claims_expiring_in(60);
        let token = codec.create(&claims).unwrap();
        assert_eq!(codec.verify(&token.0).unwrap(), claims);
    }

    #[test]
    fn a_token_is_safe_to_put_in_a_url() {
        let codec = HmacPlayCommandTokens::new(SECRET);
        let token = codec.create(&claims_expiring_in(60)).unwrap();
        assert!(
            token.0.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.')),
            "the Roku receives it as an ECP query parameter: {}",
            token.0
        );
    }

    #[test]
    fn verify_rejects_expired_token() {
        let codec = HmacPlayCommandTokens::new(SECRET);
        let token = codec.create(&claims_expiring_in(-3600)).unwrap();
        assert!(matches!(codec.verify(&token.0), Err(PlayCommandTokenError::Expired)));
    }

    #[test]
    fn verify_rejects_a_token_just_past_its_expiry() {
        let codec = HmacPlayCommandTokens::new(SECRET);
        let token = codec.create(&claims_expiring_in(-5)).unwrap();
        assert!(matches!(codec.verify(&token.0), Err(PlayCommandTokenError::Expired)));
    }

    #[test]
    fn verify_rejects_wrong_secret() {
        let issuer = HmacPlayCommandTokens::new(SECRET);
        let other = HmacPlayCommandTokens::new(b"a-different-secret");
        let token = issuer.create(&claims_expiring_in(60)).unwrap();
        assert!(matches!(other.verify(&token.0), Err(PlayCommandTokenError::Invalid)));
    }

    #[test]
    fn verify_rejects_tampered_token() {
        let codec = HmacPlayCommandTokens::new(SECRET);
        let mut bytes = codec.create(&claims_expiring_in(60)).unwrap().0.into_bytes();
        let last = bytes.len() - 1;
        bytes[last] = if bytes[last] == b'a' { b'b' } else { b'a' };
        let tampered = String::from_utf8(bytes).unwrap();
        assert!(matches!(codec.verify(&tampered), Err(PlayCommandTokenError::Invalid)));
    }

    #[test]
    fn verify_rejects_garbage() {
        let codec = HmacPlayCommandTokens::new(SECRET);
        assert!(matches!(codec.verify("not.a.jwt"), Err(PlayCommandTokenError::Invalid)));
    }

    #[test]
    fn verify_rejects_out_of_range_expiry() {
        let codec = HmacPlayCommandTokens::new(SECRET);
        let token = raw(PLAY_COMMAND_AUDIENCE, PLAY_COMMAND_TOKEN_TYPE, i64::MAX);
        assert!(matches!(codec.verify(&token), Err(PlayCommandTokenError::Invalid)));
    }

    #[test]
    fn verify_rejects_wrong_audience_or_type() {
        let codec = HmacPlayCommandTokens::new(SECRET);
        let exp = Timestamp::now().as_second() + 60;
        for token in [
            raw("shadowmask-download", PLAY_COMMAND_TOKEN_TYPE, exp),
            raw(PLAY_COMMAND_AUDIENCE, "download", exp),
        ] {
            assert!(matches!(codec.verify(&token), Err(PlayCommandTokenError::Invalid)));
        }
    }

    #[test]
    fn download_and_play_command_tokens_do_not_verify_as_each_other() {
        let commands = HmacPlayCommandTokens::new(SECRET);
        let downloads = HmacDownloadTokens::new(SECRET);
        let download = downloads
            .create(&DownloadClaims {
                user: UserId("user-1".into()),
                version: VersionId("version-1".into()),
                expires_at: Timestamp::from_second(Timestamp::now().as_second() + 60).unwrap(),
                nonce: "nonce-1".into(),
            })
            .unwrap();
        assert!(matches!(commands.verify(&download.0), Err(PlayCommandTokenError::Invalid)));
        let command = commands.create(&claims_expiring_in(60)).unwrap();
        assert!(downloads.verify(&command.0).is_err());
    }
}
