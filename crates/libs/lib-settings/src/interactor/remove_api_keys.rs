use crate::protocols::ApiKeys;

/// Delete every Price Source API key, the way out when the secret is lost (ADR-0043). Nothing can
/// refuse it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RemoveApiKeys;

impl RemoveApiKeys {
    pub fn run(self, keys: &dyn ApiKeys) {
        keys.remove_all();
        tracing::info!("Removed every Price Source API key");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake::InMemoryApiKeys;

    #[test]
    fn every_key_is_gone_after_the_removal() {
        let keys = InMemoryApiKeys::new(Some("3F9A-21C0"), 2);

        RemoveApiKeys.run(&keys);

        assert_eq!(keys.saved(), 0);
    }
}
