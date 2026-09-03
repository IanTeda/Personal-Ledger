//! # Authorization code store
//!
//! Short-lived, single-use, process-local -- unlike the refresh token, an authorization
//! code only needs to survive the few seconds between the `/authorize` redirect and the
//! Client's `/token` exchange, so it lives in memory rather than the durable store
//! (ADR-0010).

use std::collections::HashMap;
use std::sync::Mutex;

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use rand::RngCore;

/// An issued-but-not-yet-redeemed authorization code and what it was issued for.
#[derive(Debug, Clone)]
pub struct AuthorizationCode {
    pub code_challenge: String,
    pub redirect_uri: String,
    pub account_id: lib_domain::RowID,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

/// Codes are valid for 2 minutes -- long enough for a human to submit the login form,
/// short enough that a leaked-but-unused code is worthless almost immediately.
const CODE_TTL_SECONDS: i64 = 2 * 60;

/// The Sync Server's in-memory table of outstanding authorization codes.
#[derive(Debug, Default)]
pub struct CodeStore {
    codes: Mutex<HashMap<String, AuthorizationCode>>,
}

impl CodeStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Mint a new authorization code for `account_id`, recording the PKCE
    /// `code_challenge` and `redirect_uri` it was issued against.
    pub fn issue(
        &self,
        code_challenge: String,
        redirect_uri: String,
        account_id: lib_domain::RowID,
    ) -> String {
        let mut bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut bytes);
        let code = URL_SAFE_NO_PAD.encode(bytes);

        let entry = AuthorizationCode {
            code_challenge,
            redirect_uri,
            account_id,
            expires_at: chrono::Utc::now() + chrono::Duration::seconds(CODE_TTL_SECONDS),
        };

        self.codes
            .lock()
            .expect("authorization code store mutex should not be poisoned")
            .insert(code.clone(), entry);

        code
    }

    /// Consume a code -- single-use: it's removed whether or not it was found or has
    /// expired, so a second attempt with the same code always fails.
    pub fn redeem(&self, code: &str) -> Option<AuthorizationCode> {
        let entry = self
            .codes
            .lock()
            .expect("authorization code store mutex should not be poisoned")
            .remove(code)?;

        if entry.expires_at < chrono::Utc::now() {
            return None;
        }

        Some(entry)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn issue_then_redeem_returns_the_recorded_entry() {
        let store = CodeStore::new();
        let account_id = lib_domain::RowID::new();

        let code = store.issue(
            "challenge".to_string(),
            "http://127.0.0.1:1234/callback".to_string(),
            account_id,
        );
        let entry = store.redeem(&code).unwrap();

        assert_eq!(entry.code_challenge, "challenge");
        assert_eq!(entry.account_id, account_id);
    }

    #[test]
    fn redeem_is_single_use() {
        let store = CodeStore::new();
        let code = store.issue(
            "challenge".to_string(),
            "http://127.0.0.1:1234/callback".to_string(),
            lib_domain::RowID::new(),
        );

        assert!(store.redeem(&code).is_some());
        assert!(store.redeem(&code).is_none());
    }

    #[test]
    fn redeem_rejects_an_unknown_code() {
        let store = CodeStore::new();
        assert!(store.redeem("never-issued").is_none());
    }

    #[test]
    fn redeem_rejects_an_expired_code() {
        let store = CodeStore::new();
        let account_id = lib_domain::RowID::new();

        // Insert directly with an already-past expiry, bypassing the normal 2-minute TTL.
        let code = "expired-code".to_string();
        store.codes.lock().unwrap().insert(
            code.clone(),
            AuthorizationCode {
                code_challenge: "challenge".to_string(),
                redirect_uri: "http://127.0.0.1:1234/callback".to_string(),
                account_id,
                expires_at: chrono::Utc::now() - chrono::Duration::seconds(1),
            },
        );

        assert!(store.redeem(&code).is_none());
    }
}
