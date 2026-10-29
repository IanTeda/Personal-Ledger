//! Units: the currencies, cryptocurrencies and other commodities a Ledger counts in (`GLOSSARY.md`'s
//! Unit), with the Price Sources that price them. The first VIPER module
//! (`docs/development/viper-architecture.md`): Entities, one Interactor per user intent, and the
//! ports they reach storage through, with no UI toolkit and no I/O. `fake` holds in-memory ports
//! and the stub seed for tests and UI-first work.
//!
//! The Desktop draws the Presenters' view models (`bin-desktop/src/units/`); `lib-ledger` keeps
//! only a read copy of the Units for the domains not yet migrated (#746).

#[cfg(feature = "contract-tests")]
#[expect(
    clippy::expect_used,
    reason = "a contract check is test code a consumer runs, and fails by panicking as a test does"
)]
pub mod contract;
mod entity;
mod error;
pub mod fake;
pub mod interactor;
pub mod keymap;
pub mod msg;
pub mod presenter;
pub mod protocols;

pub use entity::{
    BUILT_IN_SOURCES, BuiltInOverride, BuiltInSource, COINGECKO, CustomSource, Endpoint,
    FRANKFURTER, KeyPlacement, MANUAL_ENTRY, PriceLink, PriceSource, PriceSourceAuth,
    PriceSourceRecord, SourceOrigin, UnitRow, Updates, WeeklyPrice, YAHOO_FINANCE,
    add_custom_source, add_unit, built_in, default_units, edit_custom_source, edit_unit,
    last_fetched, move_units_to_manual, override_built_in, price_source, price_source_exists,
    price_sources, remove_custom_source, remove_unit, seed_weekly_prices, set_unit_active,
    sources_for_kind, units_by_code,
};
pub use error::{Error, Result};
pub use lib_core::{Money, PriceSourceId, UnitId, UnitKind};
pub use protocols::{
    BaseUnitRepository, Holding, PortsMut, PriceSourceRepository, SharedPorts, UnitHoldings,
    UnitRepository, UnitUsage, UnitUse, UnitsDialog, UnitsPorts, UnitsRouter,
};
