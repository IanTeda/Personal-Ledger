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
    /// Which page, by its `ActiveView`'s `Debug` name (`Dashboard`, `Settings(General)`), or
    /// `settings` for the Settings View itself.
    pub page: String,
    /// Each binding as `(command, chord)`.
    pub keys: Vec<(&'static str, String)>,
    /// The chords of the page's status line legend, which a page binding no key of its own (an
    /// info page) reads from the Settings frame's keymap.
    pub legend: Vec<String>,
}

impl PageKeysSnapshot {
    fn of(page: String, handle: &dyn PageHandle, cx: &gpui::App) -> Self {
        Self {
            page,
            keys: handle
                .keys(cx)
                .into_iter()
                .map(|binding: PageBinding| (binding.command, binding.chord))
                .collect(),
            legend: handle
                .page_status(cx)
                .map(|status| status.hints.into_iter().map(|hint| hint.chord).collect())
                .unwrap_or_default(),
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
    /// The date style the Bills page hands its rows and Dialogs.
    pub bills: Option<DateStyle>,
    /// The date style the Documents page hands its rows and Dialogs.
    pub documents: Option<DateStyle>,
    /// The date style the Budgets page hands its rows.
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
            .each()
            .into_iter()
            .map(|(view, page)| PageKeysSnapshot::of(format!("{view:?}"), page, cx))
            .collect();
        pages.push(PageKeysSnapshot::of(
            "settings".to_string(),
            &self.settings,
            cx,
        ));
        let settings = self.settings.read(cx).pages();
        pages.extend(SettingsPage::ALL.into_iter().filter_map(|page| {
            settings
                .get(page)
                .map(|handle| PageKeysSnapshot::of(format!("Settings({page:?})"), handle, cx))
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
