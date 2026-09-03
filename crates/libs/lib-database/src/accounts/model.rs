//! # Account Database Model
//!
//! Defines the `Account` struct, one row of the Sync Server's auth user store
//! (`accounts` table). Password hashing and JWT/refresh-token issuance are not this
//! crate's concern -- it stores and compares opaque hash strings only, the same way it
//! already treats `HexColor`/`UrlSlug` as pre-validated values. See
//! [ADR-0010](https://github.com/IanTeda/Personal-Ledger/blob/feasibility/docs/adr/0010-oauth2-pkce-native-app-auth.md).

/// Database row model representing one persisted account.
///
/// Single-account this cycle (ADR-0010): the Sync Server's user store holds exactly one
/// bootstrap account, not multi-user account management.
#[derive(Debug, sqlx::FromRow, serde::Deserialize, serde::Serialize, PartialEq, Clone)]
pub struct Account {
    /// Unique time-ordered identifier for the account.
    pub id: lib_domain::RowID,

    /// Unique username used to sign in via the `/authorize` login form.
    pub username: String,

    /// Argon2 password hash (PHC string format). Never the plaintext password.
    pub password_hash: String,

    /// Hash of the currently valid refresh token, if any. `None` means no active
    /// session -- the account hasn't completed the OAuth2 flow, or was logged out.
    /// Rotates (replaced) on every successful token refresh.
    pub refresh_token_hash: Option<String>,

    /// UTC timestamp when the account was first created.
    pub created_on: chrono::DateTime<chrono::Utc>,

    /// UTC timestamp when the account was last modified.
    pub updated_on: chrono::DateTime<chrono::Utc>,
}

impl Account {
    /// Generate a mock `Account` instance with randomised test data.
    ///
    /// **Note**: This function is only available in test builds.
    #[cfg(test)]
    pub fn mock() -> Self {
        use crate::accounts::AccountBuilder;
        use fake::Fake;
        use fake::faker::lorem::en::Word;

        let now = chrono::Utc::now();
        let username = format!("{}-{}", Word().fake::<String>(), lib_domain::RowID::new());
        AccountBuilder::new()
            .with_id(lib_domain::RowID::mock())
            .with_username(username)
            .with_password_hash("$argon2id$v=19$m=19456,t=2,p=1$mock$mock".to_string())
            .with_refresh_token_hash_opt(None)
            .with_created_on_opt(Some(now))
            .with_updated_on_opt(Some(now))
            .build()
            .expect("Mock Account should always build successfully")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_generates_valid_account() {
        let account = Account::mock();
        assert!(!account.username.is_empty());
        assert!(!account.password_hash.is_empty());
        assert!(account.refresh_token_hash.is_none());
    }

    #[test]
    fn account_struct_derives_work() {
        let a1 = Account::mock();
        let a2 = a1.clone();
        assert_eq!(a1, a2);

        let debug_str = format!("{:?}", a1);
        assert!(debug_str.contains("Account"));

        let json = serde_json::to_string(&a1).unwrap();
        let deserialized: Account = serde_json::from_str(&json).unwrap();
        assert_eq!(a1, deserialized);
    }
}
