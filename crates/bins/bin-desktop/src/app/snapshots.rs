//! What the integration tests read back of each page and overlay: plain-value snapshot shapes and
//! the `Shell` accessors that build them, one file per destination. Kept apart from the
//! `shell/<domain>_ui.rs` files so those mean only "this domain's behaviour wiring".

mod bills;
mod budgets;
mod documents;
mod overlays;
mod palette;
mod settings;
mod transactions;

pub use bills::{BillRowSnapshot, BillsSnapshot};
pub use budgets::{BudgetRowSnapshot, BudgetsSnapshot};
pub use documents::DocumentsSnapshot;
pub use overlays::{
    DashboardBillSnapshot, ImportRowSnapshot, ImportSnapshot, ToastSnapshot, ToastsSnapshot,
};
pub use palette::PaletteSnapshot;
pub use settings::{AccountFormSnapshot, SettingsSnapshot};
pub use transactions::{ChipSnapshot, TransactionsSnapshot};
