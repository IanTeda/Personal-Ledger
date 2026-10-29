//! # Units Database Model
//!
//! Defines the `Units` struct, one row of the `units` table — a currency, cryptocurrency,
//! stock, precious metal, or other tradeable instrument a Client's Accounts and
//! Transactions are denominated in (FR.1, `docs/product-requirements.md`). No cross-Unit
//! conversion exists in V1 (see `GLOSSARY.md`'s Unit glossary entry) — `unit_kind` is a
//! descriptive grouping only. Each Unit links to a Price Source (`price_source_id`,
//! `price_symbol`, `quote_unit_id`) and keeps its newest price with no history (ADR-0043).

/// Database row model representing one persisted Unit.
#[derive(Debug, sqlx::FromRow, serde::Deserialize, serde::Serialize, PartialEq, Clone)]
pub struct Units {
    /// Unique time-ordered identifier for the Unit.
    pub id: lib_core::RowID,

    /// A unique, free-form code identifying the Unit (e.g. "AUD", "BTC", "AAPL", "XAU").
    /// Unlike Category codes, no fixed format is enforced — real-world Unit codes vary too
    /// widely (ISO 4217 currency codes, variable-length tickers) for one pattern to fit.
    pub code: String,

    /// Human-readable display name (e.g. "Australian Dollar", "Bitcoin").
    pub name: String,

    /// Descriptive grouping — fiat, crypto, stock, precious metal, or other.
    pub unit_kind: lib_core::UnitKind,

    /// Id of the Price Source that prices this Unit. Not a foreign key yet: Price Sources
    /// persist in a later ticket.
    pub price_source_id: String,

    /// What the Price Source calls this Unit, filling a request's `{symbol}`. Empty for
    /// Manual entry.
    pub price_symbol: String,

    /// The Unit the price is quoted in. The base Unit quotes itself.
    pub quote_unit_id: lib_core::RowID,

    /// Soft-delete flag — `false` means the Unit should not be used for new Accounts or
    /// Transactions, but existing ones remain valid.
    pub is_active: bool,

    /// UTC timestamp when the Unit was first created.
    pub created_on: chrono::DateTime<chrono::Utc>,

    /// UTC timestamp when the Unit was last modified.
    pub updated_on: chrono::DateTime<chrono::Utc>,
}

impl Units {
    /// Generates a mock `Units` instance with randomised test data.
    #[cfg(test)]
    pub fn mock() -> Self {
        use crate::units::UnitsBuilder;
        use fake::Fake;
        use fake::faker::currency::en::CurrencyCode;

        let now = chrono::Utc::now();
        let code = format!(
            "{}-{}",
            CurrencyCode().fake::<String>(),
            lib_core::RowID::new()
        );
        UnitsBuilder::new()
            .with_id(lib_core::RowID::mock())
            .with_code(code)
            .with_name(Self::generate_mock_name())
            .with_unit_kind(lib_core::UnitKind::mock())
            .with_is_active_opt(Some(true))
            .with_created_on_opt(Some(now))
            .with_updated_on_opt(Some(now))
            .build()
            .expect("Mock Units should always build successfully")
    }

    #[cfg(test)]
    fn generate_mock_name() -> String {
        use fake::Fake;
        use fake::faker::lorem::en::Words;

        let words: Vec<String> = Words(1..3).fake();
        words.join(" ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_generates_valid_unit() {
        let unit = Units::mock();
        assert!(!unit.code.is_empty());
        assert!(!unit.name.is_empty());
        assert!(unit.is_active);
    }

    #[test]
    fn units_struct_derives_work() {
        let a = Units::mock();
        let b = a.clone();
        assert_eq!(a, b);

        let debug_str = format!("{:?}", a);
        assert!(debug_str.contains("Units"));

        let json = serde_json::to_string(&a).unwrap();
        let deserialized: Units = serde_json::from_str(&json).unwrap();
        assert_eq!(a, deserialized);
    }
}
