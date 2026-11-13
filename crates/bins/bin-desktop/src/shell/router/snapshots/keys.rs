//! What the rebind and Display tests read back of every page `Root` hosts (#1167): each page's
//! keys through the one hosting trait, and the date each Display reader draws.

use lib_bills::presenter::PlanFormPresenter;
use lib_budgets::presenter::SwitcherPresenter;

use crate::shell::router::Root;
use crate::shell::view::dialog_entity::ModuleDialog;
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

/// The date each Display reader draws, as drawn. Bills and Budgets draw theirs only in a Dialog,
/// so their fields are `None` while that Dialog is closed.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisplayDatesSnapshot {
    /// The first Needs Attention row's due date, as drawn.
    pub dashboard_due: Option<String>,
    /// The first Transaction's date, as drawn.
    pub transactions_date: Option<String>,
    /// The first Library row's date, as drawn.
    pub documents: Option<String>,
    /// The Plan form's first due date, as drawn, while the Plan form is the open Dialog.
    pub bills: Option<String>,
    /// The first archived Budget's date, as the Switcher draws it in its summary (`archived <date>`),
    /// while the Switcher is the open Dialog.
    pub budgets: Option<String>,
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
        pages.extend(
            self.settings.read(cx).pages().each().map(|(page, handle)| {
                PageKeysSnapshot::of(format!("Settings({page:?})"), handle, cx)
            }),
        );
        pages
    }

    /// How many main Views the Open Ledger hosts, so a test counting the hosted pages follows a
    /// new View.
    #[doc(hidden)]
    pub fn main_view_count(&self) -> usize {
        self.open().views.each().len()
    }

    /// The date each Display reader draws, read back for assertions. The Plan form and the
    /// Switcher are read only while each is the open Dialog.
    #[doc(hidden)]
    pub fn display_dates_snapshot(&self, cx: &gpui::App) -> DisplayDatesSnapshot {
        let views = &self.open().views;
        let dialog = self.hosted_dialog();
        let plan_form =
            dialog.and_then(|dialog| dialog.downcast::<ModuleDialog<PlanFormPresenter>>());
        let switcher =
            dialog.and_then(|dialog| dialog.downcast::<ModuleDialog<SwitcherPresenter>>());
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
            documents: views
                .documents
                .read(cx)
                .view_model()
                .library
                .and_then(|library| library.rows.first().map(|row| row.date.clone())),
            bills: plan_form.map(|form| form.read(cx).view_model().first_due.text),
            budgets: switcher.and_then(|switcher| {
                switcher
                    .read(cx)
                    .view_model()
                    .rows
                    .into_iter()
                    .find(|row| row.archived)
                    .map(|row| {
                        // The summary is `archived <date>`: the date is its last word.
                        row.summary
                            .rsplit_once(' ')
                            .map_or_else(|| row.summary.clone(), |(_, date)| date.to_string())
                    })
            }),
        }
    }
}
