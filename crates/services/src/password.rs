use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::{PasswordHash, SaltString};
use argon2::{Argon2, PasswordHasher, PasswordVerifier};

#[derive(Debug, thiserror::Error)]
pub enum PasswordError {
    #[error("password hash error: {0}")]
    Hash(String),
}

pub fn hash(password: &str) -> Result<String, PasswordError> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
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
