//! The active View as `Root` hosts it (ADR-0016's and ADR-0056's #1079 update): [`page`] is the one
//! match on the [`ActiveView`], handing back the View's Entity as one `dyn` [`PageHandle`], so a new
//! View fails to compile here and nowhere else. `Root` calls the trait, not a forwarding method per
//! View, and keeps each View's typed Entity in [`ViewEntities`] for the hand-offs between Views.
//!
//! A View `Root` still draws itself has no handle: a placeholder Noun, or with no Ledger open the
//! no Ledger open content ([`ShellView`]).

use gpui::Entity;

use crate::bills::BillsPage;
use crate::budgets::BudgetsPage;
use crate::dashboard::DashboardPage;
use crate::documents::view::page::DocumentsPage;
use crate::navigation::{active_view::ActiveView, nav::Noun};
use crate::settings::state::SettingsView;
use crate::shell::view::page_handle::PageHandle;
use crate::transaction_import::ImportPage;
use crate::transactions::TransactionsPage;

/// The Views `Root` still draws itself, with no Entity of their own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellView {
    /// A Noun whose surface is a later map's work (#420).
    Placeholder(Noun),
    /// No Ledger open (#977): the Dashboard's no Ledger open content, offering `:open` and `:new`,
    /// which stands without the Open Ledger's Views.
    NoLedger,
}

impl ShellView {
    /// What `Root` draws for `view`, which has no handle: with no Ledger open, every View but
    /// Settings is the no Ledger open content.
    pub fn of(view: ActiveView, ledger_open: bool) -> Self {
        match view {
            ActiveView::Placeholder(noun) if ledger_open => Self::Placeholder(noun),
            _ => Self::NoLedger,
        }
    }
}

/// The View Entities `Root` owns, one per View that renders and handles its own input.
pub struct ViewEntities {
    pub dashboard: Entity<DashboardPage>,
    pub bills: Entity<BillsPage>,
    pub budgets: Entity<BudgetsPage>,
    pub transactions: Entity<TransactionsPage>,
    pub import: Entity<ImportPage>,
    pub documents: Entity<DocumentsPage>,
}

impl ViewEntities {
    /// Every View, for what `Root` does to each alike (following its events).
    pub fn all(&self) -> [&dyn PageHandle; 6] {
        [
            &self.dashboard,
            &self.bills,
            &self.budgets,
            &self.transactions,
            &self.import,
            &self.documents,
        ]
    }
}

/// The Entity behind `view`, from the Open Ledger's View Entities and `Root`'s Settings: `None` for
/// a View `Root` still draws ([`ShellView`]). With no Ledger open (`views` is `None`) only Settings
/// has a handle.
pub fn page<'a>(
    view: ActiveView,
    views: Option<&'a ViewEntities>,
    settings: &'a Entity<SettingsView>,
) -> Option<&'a dyn PageHandle> {
    if let ActiveView::Settings(_) = view {
        return Some(settings);
    }
    let views = views?;
    match view {
        ActiveView::Dashboard => Some(&views.dashboard),
        ActiveView::Bills => Some(&views.bills),
        ActiveView::Budgets => Some(&views.budgets),
        ActiveView::Transactions => Some(&views.transactions),
        ActiveView::Import => Some(&views.import),
        ActiveView::Documents => Some(&views.documents),
        ActiveView::Settings(_) => Some(settings),
        ActiveView::Placeholder(_) => None,
    }
}
