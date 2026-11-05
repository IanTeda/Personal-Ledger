//! Settings as a VIPER module (`docs/development/viper-architecture.md`): the frame's Presenter,
//! view model and keymap, and [`SettingsPage`], the pages and their This device / This Ledger
//! grouping. Toolkit-free and I/O-free, it depends on no feature crate, nor on `lib-config`,
//! `lib-tracing` or `lib-units`: the config pages that join it reach Configuration, the log and the
//! Units through its own ports (#1036).
//!
//! The domain pages (Units, Institutions, Accounts, …) stay in their own modules; the frame only
//! chooses which page shows and where focus sits. A page's icon is the Client's (ADR-0037,
//! 2026-10-30).

pub mod fake;
pub mod keymap;
pub mod msg;
pub mod page;
pub mod presenter;
pub mod protocols;

pub use crate::page::{ScopeFacts, ScopeGroup, SettingsPage};
pub use crate::presenter::{
    Breadcrumb, IndexGroup, IndexRow, PickerGroup, PickerRow, PickerView, SettingsFocus,
    SettingsFrameEvent, SettingsFramePresenter, SettingsFrameSetup, SettingsFrameViewModel,
};
pub use crate::protocols::SettingsRouter;
