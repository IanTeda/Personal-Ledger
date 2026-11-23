use lib_core::PriceSourceId;
use secrecy::SecretString;

use crate::Result;
use crate::protocols::PriceSourceKeys;

/// What a Dialog's Save does to its source's API key (ADR-0043): a blank field keeps it, typed
/// text replaces it, and Remove key or switching the authentication to None removes it.
#[derive(Debug, Clone)]
pub enum KeyChange {
    Keep,
    Replace(SecretString),
    Remove,
}

/// Write a Price Source's API key, in the commit of the Save or delete it belongs to. Never
/// undoable: undoing that Save restores the source and leaves the key.
#[derive(Debug, Clone)]
pub struct SavePriceSourceKey {
    pub source: PriceSourceId,
    pub change: KeyChange,
}

impl SavePriceSourceKey {
    /// Returns whether a key was stored, which clears the source's Key missing and Key rejected
    /// failures. Refused when this device can't write the Ledger's keys or keep a secret for them.
    pub fn run(self, keys: &mut dyn PriceSourceKeys) -> Result<bool> {
        let source = self.source;
        match self.change {
            KeyChange::Keep => Ok(false),
            KeyChange::Remove => {
                if keys.key_state(source) != crate::KeyState::NotSet {
                    keys.remove_key(source);
                    tracing::info!(price_source_id = %source, "Removed a Price Source's API key");
                }
                Ok(false)
            }
            KeyChange::Replace(key) => match keys.store_key(source, key) {
                Ok(()) => {
                    tracing::info!(price_source_id = %source, "Saved a Price Source's API key");
                    Ok(true)
                }
                Err(refusal) => {
                    tracing::info!(
                        price_source_id = %source,
                        %refusal,
                        "Refused to save a Price Source's API key"
                    );
                    Err(refusal)
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake::{InMemoryKey, InMemoryKeys};
    use crate::{COINGECKO, Error, KeyState};

    fn replace(key: &str) -> SavePriceSourceKey {
        SavePriceSourceKey {
            source: COINGECKO,
            change: KeyChange::Replace(key.into()),
        }
    }

    #[test]
    fn a_typed_key_replaces_the_saved_one_and_blank_keeps_it() {
        let mut keys = InMemoryKeys::default();

        assert_eq!(replace("first").run(&mut keys), Ok(true));
        assert_eq!(replace("second").run(&mut keys), Ok(true));
        let kept = SavePriceSourceKey {
            source: COINGECKO,
            change: KeyChange::Keep,
        };
        assert_eq!(kept.run(&mut keys), Ok(false));

        assert_eq!(keys.exposed()[&COINGECKO], "second");
    }

    #[test]
    fn remove_clears_a_saved_or_unreadable_key() {
        let mut keys = InMemoryKeys::default();
        keys.keys.insert(COINGECKO, InMemoryKey::Unreadable);

        let removed = SavePriceSourceKey {
            source: COINGECKO,
            change: KeyChange::Remove,
        }
        .run(&mut keys);

        assert_eq!(removed, Ok(false));
        assert_eq!(keys.key_state(COINGECKO), KeyState::NotSet);
    }

    #[test]
    fn a_ledger_locked_by_another_secret_refuses_the_key_and_keeps_its_own() {
        let mut keys = InMemoryKeys {
            locked: true,
            ..InMemoryKeys::default()
        };
        keys.keys.insert(COINGECKO, InMemoryKey::Unreadable);

        assert_eq!(replace("mine").run(&mut keys), Err(Error::KeysLocked));
        assert_eq!(keys.key_state(COINGECKO), KeyState::Unreadable);
    }
}
