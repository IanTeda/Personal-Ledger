//! # Units Builder
//!
//! Provides a fluent API for constructing [`Units`] records, mirroring
//! [`crate::categories::CategoriesBuilder`]'s shape.

use super::Units;
use crate::Error;

/// Fluent builder for [`Units`] rows.
#[derive(Debug, Default, Clone)]
pub struct UnitsBuilder {
    id: Option<lib_core::RowID>,
    code: Option<String>,
    name: Option<String>,
    unit_kind: Option<lib_core::UnitKind>,
    price_source_id: Option<String>,
    price_symbol: Option<String>,
    quote_unit_id: Option<lib_core::RowID>,
    last_price: Option<String>,
    last_price_fetched_on: Option<String>,
    is_active: Option<bool>,
    created_on: Option<chrono::DateTime<chrono::Utc>>,
    updated_on: Option<chrono::DateTime<chrono::Utc>>,
}

impl UnitsBuilder {
    /// Starts building a new Unit with no preset values.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Use an existing [`RowID`](lib_core::RowID) for the Unit.
    #[must_use]
    pub fn with_id(mut self, id: lib_core::RowID) -> Self {
        self.id = Some(id);
        self
    }

    /// Set the Unit's code (e.g. "AUD", "BTC").
    #[must_use]
    pub fn with_code(mut self, code: impl Into<String>) -> Self {
        self.code = Some(code.into());
        self
    }

    /// Set the Unit's display name.
    #[must_use]
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the Unit's descriptive kind.
    #[must_use]
    pub fn with_unit_kind(mut self, unit_kind: lib_core::UnitKind) -> Self {
        self.unit_kind = Some(unit_kind);
        self
    }

    /// Set the id of the Price Source that prices the Unit, defaulting to `manual` when unset.
    #[must_use]
    pub fn with_price_source_id(mut self, price_source_id: impl Into<String>) -> Self {
        self.price_source_id = Some(price_source_id.into());
        self
    }

    /// Set what the Price Source calls the Unit, defaulting to empty when unset.
    #[must_use]
    pub fn with_price_symbol(mut self, price_symbol: impl Into<String>) -> Self {
        self.price_symbol = Some(price_symbol.into());
        self
    }

    /// Set the Unit the price is quoted in, defaulting to the Unit itself (the base Unit)
    /// when unset.
    #[must_use]
    pub fn with_quote_unit_id(mut self, quote_unit_id: lib_core::RowID) -> Self {
        self.quote_unit_id = Some(quote_unit_id);
        self
    }

    /// Set the newest price (a decimal string) and when it was fetched or entered.
    #[must_use]
    pub fn with_last_price(
        mut self,
        last_price: impl Into<String>,
        fetched_on: impl Into<String>,
    ) -> Self {
        self.last_price = Some(last_price.into());
        self.last_price_fetched_on = Some(fetched_on.into());
        self
    }

    /// Provide an optional active flag, defaulting to `true` when unset.
    #[must_use]
    pub fn with_is_active_opt(mut self, is_active: Option<bool>) -> Self {
        self.is_active = is_active;
        self
    }

    /// Provide an optional creation timestamp, defaulting to now when unset.
    #[must_use]
    pub fn with_created_on_opt(
        mut self,
        created_on: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Self {
        self.created_on = created_on;
        self
    }

    /// Provide an optional update timestamp, defaulting to now when unset.
    #[must_use]
    pub fn with_updated_on_opt(
        mut self,
        updated_on: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Self {
        self.updated_on = updated_on;
        self
    }

    /// Build the [`Units`], returning an error when required fields are missing.
    pub fn build(self) -> crate::Result<Units> {
        let code = self.code.ok_or(Error::UnitsBuilder(
            "code is required but was not set".to_string(),
        ))?;
        let name = self.name.ok_or(Error::UnitsBuilder(
            "name is required but was not set".to_string(),
        ))?;

        // clippy's unwrap_or_default suggestion is WRONG here: RowID::default() is a nil
        // (version 0) UUID, not a usable row id -- RowID's Decode requires version 7.
        #[expect(
            clippy::unwrap_or_default,
            reason = "RowID::default() is a nil UUID, not a usable row id"
        )]
        let id = self.id.unwrap_or_else(lib_core::RowID::new);

        Ok(Units {
            id,
            code,
            name,
            unit_kind: self.unit_kind.unwrap_or_default(),
            price_source_id: self.price_source_id.unwrap_or_else(|| "manual".to_string()),
            price_symbol: self.price_symbol.unwrap_or_default(),
            quote_unit_id: self.quote_unit_id.unwrap_or(id),
            last_price: self.last_price,
            last_price_fetched_on: self.last_price_fetched_on,
            is_active: self.is_active.unwrap_or(true),
            created_on: self.created_on.unwrap_or_else(chrono::Utc::now),
            updated_on: self.updated_on.unwrap_or_else(chrono::Utc::now),
        })
    }
}
