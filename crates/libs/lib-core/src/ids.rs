//! The Ledger's ids (ADR-0040): one [`RowID`] newtype per kind of row, so a `PayeeId` can't be passed
//! where a `CategoryId` is wanted and a row's id is the same value in memory, in the schema and in
//! ADR-0009's Change Sets.

use crate::RowID;

/// Declares a Ledger id: a `Copy` newtype over [`RowID`]. `$tag` keeps the stub seeds of different
/// kinds apart (`PayeeId::seed(1)` is not `TagId::seed(1)`), which makes a crossed-wires bug in a
/// seed show up as a miss rather than as the wrong row.
macro_rules! ledger_id {
    ($(#[$meta:meta])* $name:ident, $tag:expr) => {
        $(#[$meta])*
        #[derive(
            Debug,
            Copy,
            Clone,
            PartialEq,
            Eq,
            PartialOrd,
            Ord,
            Hash,
            serde::Serialize,
            serde::Deserialize,
        )]
        pub struct $name(RowID);

        impl $name {
            /// A fresh time-ordered id, for a row created now.
            #[expect(
                clippy::new_without_default,
                reason = "a default id would be a fresh random one, which `unwrap_or_default` would hide"
            )]
            pub fn new() -> Self {
                Self(RowID::new())
            }

            /// An id from a fixed UUID.
            pub const fn from_uuid(uuid: uuid::Uuid) -> Self {
                Self(RowID::from_uuid(uuid))
            }

            /// The `n`th id of the stub seeds, which are fixed so the headless UI tests stay
            /// deterministic. `n` is the row's place in its seed, not a number to allocate from.
            pub const fn seed(n: u32) -> Self {
                Self::from_uuid(uuid::Uuid::from_u128(
                    0x0190_0000_0000_7000_8000_0000_0000_0000 | (($tag as u128) << 32) | n as u128,
                ))
            }

            /// The `n` a [`seed`](Self::seed) id was made from, for seed code that picks or names
            /// rows by their place in the seed. Meaningless for an id from [`new`](Self::new).
            pub fn seed_number(&self) -> u32 {
                (self.0.as_uuid().as_u128() & 0xFFFF_FFFF) as u32
            }

            pub fn row_id(&self) -> RowID {
                self.0
            }
        }

        impl From<RowID> for $name {
            fn from(row_id: RowID) -> Self {
                Self(row_id)
            }
        }

        impl std::str::FromStr for $name {
            type Err = crate::RowIDError;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                s.parse::<RowID>().map(Self)
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.fmt(f)
            }
        }

        #[cfg(feature = "sqlx")]
        impl sqlx::Type<sqlx::Sqlite> for $name {
            fn type_info() -> sqlx::sqlite::SqliteTypeInfo {
                <RowID as sqlx::Type<sqlx::Sqlite>>::type_info()
            }
        }

        #[cfg(feature = "sqlx")]
        impl<'q> sqlx::Encode<'q, sqlx::Sqlite> for $name {
            fn encode_by_ref(
                &self,
                buf: &mut <sqlx::Sqlite as sqlx::Database>::ArgumentBuffer<'q>,
            ) -> Result<sqlx::encode::IsNull, sqlx::error::BoxDynError> {
                <RowID as sqlx::Encode<'q, sqlx::Sqlite>>::encode_by_ref(&self.0, buf)
            }
        }

        #[cfg(feature = "sqlx")]
        impl<'r> sqlx::Decode<'r, sqlx::Sqlite> for $name {
            fn decode(
                value: sqlx::sqlite::SqliteValueRef<'r>,
            ) -> Result<Self, sqlx::error::BoxDynError> {
                <RowID as sqlx::Decode<sqlx::Sqlite>>::decode(value).map(Self)
            }
        }
    };
}

ledger_id!(
    /// An Account's id.
    AccountId,
    1
);
ledger_id!(
    /// A Bill Plan's id.
    BillPlanId,
    2
);
ledger_id!(
    /// A Budget's id.
    BudgetId,
    3
);
ledger_id!(
    /// A Category's id.
    CategoryId,
    4
);
ledger_id!(
    /// A Document's id.
    DocumentId,
    5
);
ledger_id!(
    /// A Document Type's id.
    DocumentTypeId,
    6
);
ledger_id!(
    /// An Inventory Item's id.
    ItemId,
    7
);
ledger_id!(
    /// A Payee's id.
    PayeeId,
    8
);
ledger_id!(
    /// An Inventory Location's id.
    LocationId,
    9
);
ledger_id!(
    /// An Inventory Area's id.
    AreaId,
    10
);
ledger_id!(
    /// A Tag's id.
    TagId,
    11
);
ledger_id!(
    /// A Transaction's id.
    TransactionId,
    12
);
ledger_id!(
    /// A Price Source's id. A built-in source's id is fixed in `lib_units`' table, so a Ledger
    /// refers to the built-in in code rather than holding a copy of it (ADR-0043).
    PriceSourceId,
    13
);
ledger_id!(
    /// A Unit's id. Addresses the Unit across edits, so a Unit's code (which a person can change)
    /// is never the key an Account or a command holds.
    UnitId,
    0
);

impl UnitId {
    /// The stub seed's fixed ids. They live here, not in `lib_units`, because `lib_accounts`' seed
    /// points at them and the domain crates never depend on each other.
    pub const SEED_AUD: Self = Self::seed(1);
    pub const SEED_BTC: Self = Self::seed(2);
    pub const SEED_VAS: Self = Self::seed(3);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_id_is_not_another_new_id() {
        assert_ne!(PayeeId::new(), PayeeId::new());
    }

    #[test]
    fn a_seed_id_is_the_same_every_time() {
        assert_eq!(PayeeId::seed(3), PayeeId::seed(3));
        assert_ne!(PayeeId::seed(3), PayeeId::seed(4));
    }

    #[test]
    fn a_seed_id_gives_back_its_number() {
        assert_eq!(TagId::seed(42).seed_number(), 42);
    }

    #[test]
    fn seeds_of_different_kinds_hold_different_uuids() {
        assert_ne!(PayeeId::seed(1).row_id(), TagId::seed(1).row_id());
    }

    #[test]
    fn an_id_round_trips_through_its_string() {
        let id = PayeeId::new();
        assert_eq!(id.to_string().parse::<PayeeId>().unwrap(), id);
        assert!("3".parse::<PayeeId>().is_err());
    }

    #[test]
    fn a_seed_id_is_a_valid_row_id() {
        assert!(TransactionId::seed(7).row_id().validate().is_ok());
    }

    #[cfg(feature = "sqlx")]
    #[test]
    fn an_id_is_stored_the_way_a_row_id_is() {
        assert_eq!(
            <PayeeId as sqlx::Type<sqlx::Sqlite>>::type_info(),
            <RowID as sqlx::Type<sqlx::Sqlite>>::type_info()
        );
    }
}
