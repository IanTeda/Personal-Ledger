//! What the rebind and Display tests read back of every page `Root` hosts (#1167): each page's
//! keys through the one hosting trait, and the date style each Display reader draws with.

use lib_core::DateStyle;

use crate::settings::SettingsPage;
use crate::shell::router::Root;
use crate::shell::view::page_handle::{PageBinding, PageHandle};

/// One hosted page's keys, as `PageHandle::keys` reports them.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageKeysSnapshot {
    /// Which page: a main View by its place among the Open Ledger's Views, a Settings page by
    /// name, or the Settings View itself.
    pub page: String,
    /// Each binding as `(command, chord)`.
    pub keys: Vec<(&'static str, String)>,
}

impl PageKeysSnapshot {
    fn of(page: String, keys: Vec<PageBinding>) -> Self {
        Self {
            page,
            keys: keys
                .into_iter()
                .map(|binding| (binding.command, binding.chord))
                .collect(),
        }
    }
}

/// The date style each Display reader draws with: the dates Dashboard and Transactions show, and
/// the style Bills, Documents and Budgets hand the Dialogs and rows that show theirs.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisplayDatesSnapshot {
    /// The first Needs Attention row's due date, as drawn.
    pub dashboard_due: Option<String>,
    /// The first Transaction's date, as drawn.
    pub transactions_date: Option<String>,
    pub bills: Option<DateStyle>,
    pub documents: Option<DateStyle>,
    pub budgets: Option<DateStyle>,
}

impl Root {
    /// Every main View's keys, then the Settings View's (its frame's and every built page's),
    /// then each built Settings page's on its own.
    #[doc(hidden)]
    pub fn page_keys_snapshot(&self, cx: &gpui::App) -> Vec<PageKeysSnapshot> {
        let mut pages: Vec<PageKeysSnapshot> = self
            .open()
            .views
            .all()
            .into_iter()
            .enumerate()
            .map(|(index, view)| PageKeysSnapshot::of(format!("view {index}"), view.keys(cx)))
            .collect();
        pages.push(PageKeysSnapshot::of(
            "settings".to_string(),
            (&self.settings as &dyn PageHandle).keys(cx),
        ));
        let settings = self.settings.read(cx).pages();
        pages.extend(SettingsPage::ALL.into_iter().filter_map(|page| {
            settings
                .get(page)
                .map(|handle| PageKeysSnapshot::of(format!("{page:?}"), handle.keys(cx)))
        }));
        pages
    }

    /// The date style each Display reader draws with, read back for assertions.
    #[doc(hidden)]
    pub fn display_dates_snapshot(&self, cx: &gpui::App) -> DisplayDatesSnapshot {
        let views = &self.open().views;
        DisplayDatesSnapshot {
            dashboard_due: views
                .dashboard
                .read(cx)
                .view_model()
                .bills
                .first()
                .map(|bill| bill.due.clone()),
            transactions_date: views
                .transactions
                .read(cx)
                .view_model()
                .rows
                .first()
                .map(|row| row.date.clone()),
            bills: views.bills.read(cx).presenter().date_style(),
            documents: views.documents.read(cx).presenter().date_style(),
            budgets: views.budgets.read(cx).presenter().date_style(),
        }
    }
}
