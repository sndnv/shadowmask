use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};

#[derive(Debug, thiserror::Error)]
pub enum PasswordError {
    #[error("password hash error: {0}")]
    Hash(String),
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
    fn verify_errors_on_malformed_hash() {
        assert!(matches!(
            verify("anything", "not-a-phc-string").unwrap_err(),
            PasswordError::Hash(_)
        ));
    }
}
