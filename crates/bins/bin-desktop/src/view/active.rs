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
use crate::settings::SettingsPage;
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
    /// Every View, for what `Root` does to each alike (following its events, a rebind, a Display
    /// change). Built from [`Self::get`]'s exhaustive match over every View a Noun derives, so a
    /// new View can't be left out.
    pub fn all(&self) -> Vec<&dyn PageHandle> {
        derived_views()
            .into_iter()
            .filter_map(|view| self.get(view))
            .collect()
    }

    /// The Entity behind `view`, `None` for Settings, which `Root` holds apart, and a placeholder.
    fn get(&self, view: ActiveView) -> Option<&dyn PageHandle> {
        match view {
            ActiveView::Dashboard => Some(&self.dashboard),
            ActiveView::Bills => Some(&self.bills),
            ActiveView::Budgets => Some(&self.budgets),
            ActiveView::Transactions => Some(&self.transactions),
            ActiveView::Import => Some(&self.import),
            ActiveView::Documents => Some(&self.documents),
            ActiveView::Settings(_) | ActiveView::Placeholder(_) => None,
        }
    }
}

/// Every View a Noun derives, once each, in the rail's order: the Import step beside Transactions,
/// and Settings on one page, standing for them all.
fn derived_views() -> Vec<ActiveView> {
    let mut views: Vec<ActiveView> = Vec::new();
    for noun in Noun::ALL {
        for import_open in [false, true] {
            let view = ActiveView::derive(noun, SettingsPage::General, import_open);
            if !views.contains(&view) {
                views.push(view);
            }
        }
    }
    views
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
    views?.get(view)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_view_a_noun_derives_is_listed_once() {
        let views = derived_views();

        for view in [
            ActiveView::Dashboard,
            ActiveView::Transactions,
            ActiveView::Import,
            ActiveView::Documents,
            ActiveView::Bills,
            ActiveView::Budgets,
        ] {
            assert_eq!(
                views.iter().filter(|derived| **derived == view).count(),
                1,
                "{view:?}"
            );
        }
    }
}
