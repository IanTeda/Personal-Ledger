//! # JWT access tokens
//!
//! Self-issued, HS256-signed access tokens (ADR-0010) -- the bearer credential the
//! gRPC resource-server side checks. Short-lived by design: the durable "stay logged
//! in" property comes from the refresh token, not this signing key (see `main.rs`'s
//! Non-goals note on the ephemeral, in-memory signing secret).

use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use secrecy::{ExposeSecret, SecretString};

/// Access tokens are valid for 5 minutes.
pub const ACCESS_TOKEN_TTL_SECONDS: i64 = 5 * 60;

/// JWT claims for a Sync Server access token.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Claims {
    /// The authenticated account's `RowID`, as a string.
    pub sub: String,
    /// Issued-at time (Unix seconds).
    pub iat: i64,
    /// Expiry time (Unix seconds) -- `jsonwebtoken` validates this automatically.
    pub exp: i64,
}

/// Mint a signed access token for `account_id`.
///
/// # Errors
/// Returns an error if JWT encoding fails (should not happen for well-formed claims).
pub fn issue_access_token(
    account_id: lib_core::RowID,
    signing_key: &SecretString,
) -> Result<String, jsonwebtoken::errors::Error> {
    let now = chrono::Utc::now().timestamp();
    let claims = Claims {
        sub: account_id.to_string(),
        iat: now,
        exp: now + ACCESS_TOKEN_TTL_SECONDS,
    };
    encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(signing_key.expose_secret().as_bytes()),
    )
}

/// Verify a presented access token and return its claims.
///
/// # Errors
/// Returns an error if the token's signature, format, or expiry is invalid.
pub fn verify_access_token(
    token: &str,
    signing_key: &SecretString,
) -> Result<Claims, jsonwebtoken::errors::Error> {
    let data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(signing_key.expose_secret().as_bytes()),
        &Validation::new(Algorithm::HS256),
    )?;
    Ok(data.claims)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_key() -> SecretString {
        SecretString::from("test-signing-key-not-for-production".to_string())
    }

    #[test]
    fn issue_and_verify_round_trip() {
        let account_id = lib_core::RowID::new();
        let key = test_key();

        let token = issue_access_token(account_id, &key).unwrap();
        let claims = verify_access_token(&token, &key).unwrap();

        assert_eq!(claims.sub, account_id.to_string());
    }

    #[test]
    fn verify_rejects_a_token_signed_with_a_different_key() {
        let token = issue_access_token(lib_core::RowID::new(), &test_key()).unwrap();

        let other_key = SecretString::from("a-completely-different-key".to_string());
        assert!(verify_access_token(&token, &other_key).is_err());
    }

    #[test]
    fn verify_rejects_a_garbage_token() {
        assert!(verify_access_token("not-a-jwt", &test_key()).is_err());
    }

    #[test]
    fn verify_rejects_an_expired_token() {
        let key = test_key();
        let account_id = lib_core::RowID::new();
        let claims = Claims {
            sub: account_id.to_string(),
            iat: chrono::Utc::now().timestamp() - 600,
            exp: chrono::Utc::now().timestamp() - 300,
        };
        let expired = encode(
            &Header::new(Algorithm::HS256),
            &claims,
            &EncodingKey::from_secret(key.expose_secret().as_bytes()),
        )
        .unwrap();

        assert!(verify_access_token(&expired, &key).is_err());
    }
}
