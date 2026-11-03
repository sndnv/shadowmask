use argon2::{Algorithm, Argon2, Params, PasswordHash, PasswordHasher, PasswordVerifier, Version};

const CLIENT_PARAMS: Params = match Params::new(19_456, 2, 1, Some(32)) {
    Ok(params) => params,
    Err(_) => panic!("client digest parameters are out of range"),
};

#[derive(Debug, thiserror::Error)]
pub enum PasswordError {
    #[error("password hash error: {0}")]
    Hash(String),
}

pub fn client_digest(username: &str, password: &str) -> String {
    let salt = format!("shadowmask-{username}");
    let mut digest = [0u8; 32];
    Argon2::new(Algorithm::Argon2id, Version::V0x13, CLIENT_PARAMS)
        .hash_password_into(password.as_bytes(), salt.as_bytes(), &mut digest)
        .expect("a prefixed salt and fixed parameters are always in range");
    hex::encode(digest)
}

pub fn hash(password: &str) -> Result<String, PasswordError> {
    Argon2::default()
        .hash_password(password.as_bytes())
        .map(|hash| hash.to_string())
        .map_err(|e| PasswordError::Hash(e.to_string()))
}

pub fn verify(password: &str, hash: &str) -> Result<bool, PasswordError> {
    let parsed = PasswordHash::new(hash).map_err(|e| PasswordError::Hash(e.to_string()))?;
    Ok(Argon2::default().verify_password(password.as_bytes(), &parsed).is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    const ARGON2_0_5_HASH: &str = "$argon2id$v=19$m=19456,t=2,p=1$vwm92w7zuWFPBBhk6uh4qg$7HRnlIQXxiyD3uW8eltne1nwxdNt70G3L3+8vvY8ND0";

    #[test]
    fn verify_accepts_hash_stored_by_argon2_0_5() {
        assert!(verify("correct horse", ARGON2_0_5_HASH).unwrap());
        assert!(!verify("battery staple", ARGON2_0_5_HASH).unwrap());
    }

    #[test]
    fn hash_then_verify_roundtrip() {
        let stored = hash("correct horse").unwrap();
        assert!(stored != "correct horse");
        assert!(verify("correct horse", &stored).unwrap());
    }

    #[test]
    fn verify_rejects_wrong_password() {
        let stored = hash("correct horse").unwrap();
        assert!(!verify("battery staple", &stored).unwrap());
    }

    #[test]
    fn client_digest_matches_what_the_clients_send() {
        assert_eq!(
            client_digest("testUser", "correct horse"),
            "760e3fe1125f79fc2dc3634c38f5b816953a2d76ebe52b8c2b4588f5b803d0e1"
        );
    }

    #[test]
    fn client_digest_differs_per_username() {
        assert_ne!(
            client_digest("testUser", "correct horse"),
            client_digest("otherUser", "correct horse")
        );
    }

    #[test]
    fn verify_errors_on_malformed_hash() {
        assert!(matches!(
            verify("anything", "not-a-phc-string").unwrap_err(),
            PasswordError::Hash(_)
        ));
    }
}
