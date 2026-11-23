//! # The `[Secrets]` section
//!
//! The device's API key secret (ADR-0043): the string the Ledger's Price Source API keys are
//! encrypted with, `[Secrets] api_key_secret` or `PERSONAL_LEDGER_SECRETS__API_KEY_SECRET`. A
//! person may set their own; otherwise a Client generates one the first time a key is saved on a
//! Ledger with none ([`crate::ConfigWriter::set_api_key_secret`]). It never syncs, so every device
//! on a Ledger is given the same one by hand. No Settings page shows or writes it, and neither
//! `Debug`, serialising nor a trace ever prints it.

use secrecy::{ExposeSecret as _, SecretString};

/// Configuration for the `[Secrets]` section.
#[derive(Debug, Clone, Default, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct SecretsConfig {
    /// `None` until set by hand or generated. A blank value counts as none.
    #[serde(serialize_with = "redacted")]
    pub api_key_secret: Option<SecretString>,
}

impl SecretsConfig {
    /// The API key secret, if one is set and not blank.
    pub fn api_key_secret(&self) -> Option<&SecretString> {
        self.api_key_secret
            .as_ref()
            .filter(|secret| !secret.expose_secret().trim().is_empty())
    }
}

/// Whether a secret is set, never the secret.
fn redacted<S: serde::Serializer>(
    secret: &Option<SecretString>,
    serializer: S,
) -> crate::Result<S::Ok, S::Error> {
    match secret {
        Some(_) => serializer.serialize_some("[REDACTED]"),
        None => serializer.serialize_none(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_blank_secret_is_none() {
        let blank = SecretsConfig {
            api_key_secret: Some(" ".into()),
        };
        assert!(blank.api_key_secret().is_none());
        assert!(SecretsConfig::default().api_key_secret().is_none());
    }

    #[test]
    fn the_environment_sets_the_secret_over_the_file() {
        let config = crate::ledger::parsed_for_test(
            "[Secrets]\napi_key_secret = from-file\n",
            &[("PERSONAL_LEDGER_SECRETS__API_KEY_SECRET", "from-env")],
        );

        assert_eq!(
            config
                .secrets
                .api_key_secret()
                .map(|secret| secret.expose_secret().to_string()),
            Some("from-env".into())
        );
        assert!(
            !format!("{config:?}").contains("from-env"),
            "Config's Debug redacts it"
        );
    }

    #[test]
    fn debug_and_serialising_never_print_the_secret() {
        let config = SecretsConfig {
            api_key_secret: Some("hunter2-hunter2".into()),
        };

        assert!(!format!("{config:?}").contains("hunter2"));
        let json = serde_json::to_string(&config).unwrap();
        assert!(!json.contains("hunter2"), "{json}");
    }
}
