//! Units: the currencies, cryptocurrencies and other commodities a Ledger counts in (`GLOSSARY.md`'s
//! Unit), with the Price Sources that price them. The first VIPER module
//! (`docs/development/viper-architecture.md`): Entities, one Interactor per user intent, and the
//! ports they reach storage through, with no UI toolkit and no I/O. `fake` holds in-memory ports
//! and the sample seed for tests and UI-first work.
//!
//! The Desktop draws the Presenters' view models (`bin-desktop/src/units/`); the other domains read
//! the Units through their own read ports, answered over the Units store (#1052).

#[cfg(feature = "contract-tests")]
#[expect(
    clippy::expect_used,
    reason = "a contract check is test code a consumer runs, and fails by panicking as a test does"
)]
pub mod contract;
mod entity;
mod error;
pub mod fake;
pub mod fetching;
pub mod interactor;
pub mod keymap;
pub mod msg;
pub mod presenter;
pub mod protocols;

pub use entity::{
    BUILT_IN_SOURCES, BatchRequest, BuiltInOverride, BuiltInSource, COINGECKO, CustomSource,
    Endpoint, FRANKFURTER, HistoryEndpoint, HistoryRef, KeyPlacement, MANUAL_ENTRY, Precision,
    PrecisionStep, PriceLink, PriceSource, PriceSourceAuth, PriceSourceRecord, RequestHeader,
    SeriesShape, SourceOrigin, UnitRow, Updates, WeeklyPrice, YAHOO_FINANCE, add_custom_source,
    add_unit, built_in, edit_custom_source, edit_unit, last_fetched, move_units_to_manual,
    override_built_in, price_source, price_source_exists, price_sources, remove_custom_source,
    remove_unit, sample_units, seed_weekly_prices, seed_written_at, set_unit_active,
    sources_for_kind, units_by_code,
};
pub use error::{Error, Result};
pub use fetching::{
    CallRate, FINE_FRESH, FetchFailure, FetchOutcome, Grain, LastFetch, MIN_BACK_OFF,
    PriceFetching, PriceRequest, PriceResponse, PriceTransport, RETRY_AFTER_DEFAULT, RequestId,
    RequestKey, SharedFetching, TransportError, TransportOutcome,
};
pub use lib_core::{Money, PriceSourceId, UnitId, UnitKind};
pub use protocols::{
    BaseUnitRepository, Holding, KeyState, PortsMut, PriceSourceKeys, PriceSourceRepository,
    SharedPorts, UnitHoldings, UnitOpenings, UnitRepository, UnitUsage, UnitUse, UnitsDialog,
    UnitsPorts, UnitsRouter,
};
