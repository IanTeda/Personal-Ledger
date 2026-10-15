//! The id of a Unit (ADR-0040).

use crate::RowID;

/// A Unit's id. Addresses the Unit across edits, so a Unit's code (which a person can change) is
/// never the key an Account or a command holds.
#[derive(
    Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub struct UnitId(RowID);

impl UnitId {
    /// The stub seed's fixed ids, so the headless UI tests stay deterministic. They live here, not in
    /// `lib_units`, because `lib_accounts`' seed points at them and the domain crates never depend
    /// on each other.
    pub const SEED_AUD: Self = Self::seed(1);
    pub const SEED_BTC: Self = Self::seed(2);
    pub const SEED_VAS: Self = Self::seed(3);

    const fn seed(n: u128) -> Self {
        Self::from_uuid(uuid::Uuid::from_u128(
            0x0190_0000_0000_7000_8000_0000_0000_0000 | n,
        ))
    }

    /// A fresh time-ordered id, for a Unit created now.
    pub fn new() -> Self {
        Self(RowID::new())
    }

    /// An id from a fixed UUID, for the stub seeds, which keep the headless UI tests deterministic.
    pub const fn from_uuid(uuid: uuid::Uuid) -> Self {
        Self(RowID::from_uuid(uuid))
    }

    pub fn row_id(&self) -> RowID {
        self.0
    }
}

impl Default for UnitId {
    fn default() -> Self {
        Self::new()
    }
}

impl From<RowID> for UnitId {
    fn from(row_id: RowID) -> Self {
        Self(row_id)
    }
}

impl std::fmt::Display for UnitId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_id_is_not_another_new_id() {
        assert_ne!(UnitId::new(), UnitId::new());
    }

    #[test]
    fn a_fixed_uuid_gives_the_same_id_every_time() {
        let uuid = uuid::Uuid::from_u128(0x0190_0000_0000_7000_8000_0000_0000_0001);
        assert_eq!(UnitId::from_uuid(uuid), UnitId::from_uuid(uuid));
    }
}
