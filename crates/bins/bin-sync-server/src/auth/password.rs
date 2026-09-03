//! # Password hashing
//!
//! Argon2 hashing/verification for the Sync Server's single bootstrap account
//! (ADR-0010). Plaintext passwords are only ever handled as [`secrecy::SecretString`]
//! so they can't leak into logs or traces (`CLAUDE.md`'s `secrecy::Secret` convention).

use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use secrecy::{ExposeSecret, SecretString};

/// Hash a plaintext password into its PHC string form, for storage in
/// `accounts.password_hash`. A fresh random salt is generated internally per call.
///
/// # Errors
/// Returns an error if Argon2 hashing fails (should not happen for well-formed input).
pub fn hash_password(password: &SecretString) -> Result<String, argon2::password_hash::Error> {
    let hash = Argon2::default().hash_password(password.expose_secret().as_bytes())?;
    Ok(hash.to_string())
}

/// Verify a plaintext password against a stored PHC hash string.
///
/// Returns `true` on match, `false` on mismatch or a malformed stored hash -- callers
/// don't need to distinguish "wrong password" from "corrupt hash" here, both mean
/// authentication fails.
pub fn verify_password(password: &SecretString, stored_hash: &str) -> bool {
    let Ok(parsed_hash) = PasswordHash::new(stored_hash) else {
        return false;
    };
    Argon2::default()
        .verify_password(password.expose_secret().as_bytes(), &parsed_hash)
        .is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_and_verify_round_trip() {
        let password = SecretString::from("correct horse battery staple".to_string());
        let hash = hash_password(&password).unwrap();

        assert!(verify_password(&password, &hash));
    }

    #[test]
    fn verify_rejects_the_wrong_password() {
        let password = SecretString::from("correct horse battery staple".to_string());
        let hash = hash_password(&password).unwrap();

        let wrong = SecretString::from("not the right password".to_string());
        assert!(!verify_password(&wrong, &hash));
    }

    #[test]
    fn verify_rejects_a_malformed_stored_hash() {
        let password = SecretString::from("correct horse battery staple".to_string());
        assert!(!verify_password(&password, "not-a-phc-hash"));
    }

    #[test]
    fn hashing_the_same_password_twice_produces_different_hashes() {
        // Salted -- confirms we're not accidentally using a fixed salt.
        let password = SecretString::from("correct horse battery staple".to_string());
        let hash_a = hash_password(&password).unwrap();
        let hash_b = hash_password(&password).unwrap();

        assert_ne!(hash_a, hash_b);
        assert!(verify_password(&password, &hash_a));
        assert!(verify_password(&password, &hash_b));
    }
}
