//! What the integration tests read back of each page and overlay: plain-value snapshot shapes and
//! the `Root` accessors that build them, one file per destination. Kept apart from the
//! `shell/<domain>_ui.rs` files so those mean only "this domain's behaviour wiring".

mod bills;
mod budgets;
mod documents;
mod overlays;
mod palette;
mod settings;
mod status;
mod transactions;

pub use crate::view::dashboard::state::DashboardBillSnapshot;
pub use crate::view::help::HelpGroup;
pub use bills::{BillRowSnapshot, BillsSnapshot};
pub use budgets::{BudgetRowSnapshot, BudgetsSnapshot};
pub use documents::DocumentsSnapshot;
pub use overlays::{ToastSnapshot, ToastsSnapshot};
pub use palette::PaletteSnapshot;
pub use settings::{
    SettingsAccountsSnapshot, SettingsCategoriesSnapshot, SettingsDisplaySnapshot,
    SettingsDocumentTypesSnapshot, SettingsGeneralSnapshot, SettingsInstitutionsSnapshot,
    SettingsInventorySnapshot, SettingsPayeesSnapshot, SettingsSnapshot, SettingsTagsSnapshot,
    SettingsTracingSnapshot, SettingsUnitsSnapshot, UnitPricesSnapshot,
};
pub use status::StatusSnapshot;
pub use transactions::{ChipSnapshot, ImportRowSnapshot, ImportSnapshot, TransactionsSnapshot};
