use jsonwebtoken::errors::ErrorKind;
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::Serialize;
use serde::de::DeserializeOwned;

#[derive(Debug)]
pub(crate) enum TokenFailure {
    Expired,
    Invalid,
}

pub(crate) trait TypedClaims {
    fn typ(&self) -> &str;
}

pub(crate) struct HmacCodec {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    header: Header,
    validation: Validation,
}

impl HmacCodec {
    pub(crate) fn new(secret: &[u8], audience: &str) -> Self {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.set_audience(&[audience]);
        Self {
            encoding_key: EncodingKey::from_secret(secret),
            decoding_key: DecodingKey::from_secret(secret),
            header: Header::new(Algorithm::HS256),
            validation,
        }
    }

    pub(crate) fn sign<T: Serialize>(&self, claims: &T) -> Result<String, String> {
        encode(&self.header, claims, &self.encoding_key).map_err(|e| e.to_string())
    }

    pub(crate) fn verify<T>(&self, token: &str, typ: &str) -> Result<T, TokenFailure>
    where
        T: DeserializeOwned + TypedClaims,
    {
        let claims = decode::<T>(token, &self.decoding_key, &self.validation)
            .map_err(|e| match e.kind() {
                ErrorKind::ExpiredSignature => TokenFailure::Expired,
                _ => TokenFailure::Invalid,
            })?
            .claims;
        if claims.typ() != typ {
            return Err(TokenFailure::Invalid);
        }
        Ok(claims)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    const SECRET: &[u8] = b"shadowmask-test-secret";
    const AUDIENCE: &str = "shadowmask-test";

    #[derive(Serialize, Deserialize)]
    struct Claims {
        sub: String,
        aud: String,
        typ: String,
        exp: i64,
    }

    impl TypedClaims for Claims {
        fn typ(&self) -> &str {
            &self.typ
        }
    }

    fn claims(typ: &str, aud: &str, exp_offset: i64) -> Claims {
        Claims {
            sub: "u1".into(),
            aud: aud.to_owned(),
            typ: typ.to_owned(),
            exp: jiff::Timestamp::now().as_second() + exp_offset,
        }
    }

    #[test]
    fn a_signed_token_verifies_back() {
        let codec = HmacCodec::new(SECRET, AUDIENCE);

        let token = codec.sign(&claims("thing", AUDIENCE, 60)).unwrap();
        let back: Claims = codec.verify(&token, "thing").unwrap();

        assert_eq!(back.sub, "u1");
    }

    #[test]
    fn a_token_of_the_wrong_type_is_invalid() {
        let codec = HmacCodec::new(SECRET, AUDIENCE);

        let token = codec.sign(&claims("other", AUDIENCE, 60)).unwrap();

        assert!(matches!(
            codec.verify::<Claims>(&token, "thing"),
            Err(TokenFailure::Invalid)
        ));
    }

    #[test]
    fn a_token_for_another_audience_is_invalid() {
        let codec = HmacCodec::new(SECRET, AUDIENCE);
        let other = HmacCodec::new(SECRET, "shadowmask-elsewhere");

        let token = other
            .sign(&claims("thing", "shadowmask-elsewhere", 60))
            .unwrap();

        assert!(matches!(
            codec.verify::<Claims>(&token, "thing"),
            Err(TokenFailure::Invalid)
        ));
    }

    #[test]
    fn an_expired_token_is_told_apart_from_an_invalid_one() {
        let codec = HmacCodec::new(SECRET, AUDIENCE);

        let token = codec.sign(&claims("thing", AUDIENCE, -3600)).unwrap();

        assert!(matches!(
            codec.verify::<Claims>(&token, "thing"),
            Err(TokenFailure::Expired)
        ));
    }

    #[test]
    fn a_token_signed_with_another_secret_is_invalid() {
        let codec = HmacCodec::new(SECRET, AUDIENCE);
        let forger = HmacCodec::new(b"not-the-secret", AUDIENCE);

        let token = forger.sign(&claims("thing", AUDIENCE, 60)).unwrap();

        assert!(matches!(
            codec.verify::<Claims>(&token, "thing"),
            Err(TokenFailure::Invalid)
        ));
    }
}
