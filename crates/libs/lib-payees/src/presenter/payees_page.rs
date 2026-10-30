//! The Payees Settings page (`docs/ux/desktop-mockups/16-settings/README.md`, to the 2j pattern):
//! every Payee A–Z, ignoring case, with its Default category and match rules, and `edit · delete`
//! per row. Usage lives in Transactions and Reports, so there are no transaction or total columns.
//!
//! The selection is a Payee id, so it follows its row across a rename. It may name a Payee that has
//! since gone, so it is resolved on every read: a deleted Payee leaves the first row selected.

use lib_core::PayeeId;
use lib_list_cursor::{Step, resolve, step_id};
use lib_locale::Segment;
use lib_viper::keymap::{DELETE, EDIT, Keymap, MOVE_DOWN, MOVE_UP, NEW, SELECT};

use super::count;
use crate::protocols::{PayeeChange, PayeesDialog, PayeesOutcome, PayeesRouter, SharedPorts};
use crate::{Payee, active_count, msg, sorted_by_name, without_default_category_count};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PayeesPageEvent {
    /// `j`/`k`, `g g`, `G`, `Ctrl-d`/`Ctrl-u` and their arrow, Home/End and Page keys.
    Move(Step),
    /// A click on a row.
    Select(PayeeId),
    /// Shows the selected row's Transactions.
    OpenTransactions,
    /// New opens Add; Edit and Delete act on the selected Payee.
    New,
    Edit,
    Delete,
    /// A row's own **edit** or **delete**: selects the row, then opens its Dialog.
    EditRow(PayeeId),
    DeleteRow(PayeeId),
    /// Another View handed off Payee `id` (a Document's Payee link): selects it, if it still
    /// exists.
    Reveal(PayeeId),
    /// The open Dialog closed, with what its confirm did or `None` when cancelled.
    DialogClosed(Option<PayeesOutcome>),
}

/// The page as drawn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PayeesPageViewModel {
    /// The heading's meta: `24 payees · 3 without a default category`, the second count tagged
    /// `strong`.
    pub scope: Vec<Segment>,
    /// The same, plain, for the status line.
    pub status: String,
    pub kicker: String,
    pub add_button: String,
    pub columns: PayeeColumns,
    /// A–Z, ignoring case.
    pub rows: Vec<PayeeLine>,
    /// What shows in place of the table while there are no Payees.
    pub empty: Option<String>,
    pub footnote: String,
    /// The Default category cell of a Payee without one.
    pub no_default_category: String,
    /// The tag beside an inactive Payee's name.
    pub inactive: String,
    pub row_edit: String,
    pub row_delete: String,
    pub selected: Option<PayeeId>,
    pub hints: Vec<Hint>,
    /// The Dialog the page last opened, until it closes.
    pub open_dialog: Option<PayeesDialog>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PayeeColumns {
    pub name: String,
    pub default_category: String,
    pub match_rules: String,
    pub actions: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PayeeLine {
    pub id: PayeeId,
    pub name: String,
    /// The Default category's label, `None` when there is none to show.
    pub default_category: Option<String>,
    /// `3 rules`.
    pub rules: String,
    /// Drawn dimmed and tagged, rather than hidden.
    pub is_active: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hint {
    pub chord: String,
    pub action: String,
}

pub struct PayeesPagePresenter {
    ports: SharedPorts,
    keymap: Keymap<PayeesPageEvent>,
    selected: Option<PayeeId>,
    open_dialog: Option<PayeesDialog>,
}

impl PayeesPagePresenter {
    /// `keymap` is the one the Client looks keys up in, so the legend shows the chords it binds.
    pub fn new(ports: SharedPorts, keymap: Keymap<PayeesPageEvent>) -> Self {
        Self {
            ports,
            keymap,
            selected: None,
            open_dialog: None,
        }
    }

    pub fn handle(&mut self, event: PayeesPageEvent, router: &mut dyn PayeesRouter) {
        match event {
            PayeesPageEvent::Move(step) => {
                self.selected = step_id(&self.ids(), self.selected(), step);
            }
            PayeesPageEvent::Select(id) | PayeesPageEvent::Reveal(id) => self.select(id),
            PayeesPageEvent::OpenTransactions => {
                if let Some(id) = self.selected() {
                    router.open_transactions(id);
                }
            }
            PayeesPageEvent::New => self.open(PayeesDialog::AddPayee, router),
            PayeesPageEvent::Edit => {
                if let Some(id) = self.selected() {
                    self.open(PayeesDialog::EditPayee(id), router);
                }
            }
            PayeesPageEvent::Delete => {
                if let Some(id) = self.selected() {
                    self.open(PayeesDialog::DeletePayee(id), router);
                }
            }
            PayeesPageEvent::EditRow(id) => {
                self.select(id);
                self.open(PayeesDialog::EditPayee(id), router);
            }
            PayeesPageEvent::DeleteRow(id) => {
                self.select(id);
                self.open(PayeesDialog::DeletePayee(id), router);
            }
            PayeesPageEvent::DialogClosed(outcome) => {
                self.open_dialog = None;
                // A save selects its Payee; a removal's Toast is its Dialog's.
                if let Some(PayeesOutcome {
                    change: PayeeChange::Saved,
                    id,
                    ..
                }) = outcome
                {
                    self.select(id);
                }
            }
        }
    }

    pub fn view_model(&self) -> PayeesPageViewModel {
        let ports = self.ports.borrow();
        let payees = ports.payees().payees();
        let categories = ports.categories().payee_categories();
        let rows: Vec<PayeeLine> = sorted_by_name(&payees)
            .into_iter()
            .map(|payee| PayeeLine {
                id: payee.id,
                name: payee.name.clone(),
                default_category: payee.default_category.and_then(|id| {
                    categories
                        .iter()
                        .find(|category| category.id == id)
                        .map(|category| category.label.clone())
                }),
                rules: msg::payees_rule_count(count(payee.aliases.len())),
                is_active: payee.is_active,
            })
            .collect();
        let selected = resolve(rows.iter().map(|row| row.id), self.selected);
        let active = count(active_count(&payees));
        let without = without_default_category_count(&payees).to_string();
        let count_text = msg::payees_count(active);
        let mut scope = vec![
            Segment {
                text: count_text.clone(),
                tag: None,
                token: None,
            },
            Segment {
                text: " \u{b7} ".to_string(),
                tag: None,
                token: None,
            },
        ];
        scope.extend(msg::payees_without_default(&without));
        PayeesPageViewModel {
            scope,
            status: format!(
                "{count_text} \u{b7} {}",
                msg::payees_status_without_default(&without)
            ),
            kicker: msg::payees_kicker(),
            add_button: msg::payees_add_button("+"),
            columns: PayeeColumns {
                name: lib_locale::msg::column_name(),
                default_category: msg::payees_column_default_category(),
                match_rules: msg::payees_column_match_rules(),
                actions: lib_locale::msg::column_actions(),
            },
            empty: rows.is_empty().then(|| msg::payees_empty(self.chord(NEW))),
            rows,
            footnote: msg::payees_footnote(),
            no_default_category: msg::payees_no_default_category(),
            inactive: msg::payees_inactive(),
            row_edit: msg::payees_row_edit(),
            row_delete: msg::payees_row_delete(),
            selected,
            hints: self.hints(),
            open_dialog: self.open_dialog,
        }
    }

    /// The Payee ids A–Z, as the rows run.
    fn ids(&self) -> Vec<PayeeId> {
        let payees = self.ports.borrow().payees().payees();
        sorted_by_name(&payees)
            .into_iter()
            .map(|payee: &Payee| payee.id)
            .collect()
    }

    /// The selected Payee: the stored id while it still exists, else the first row.
    fn selected(&self) -> Option<PayeeId> {
        resolve(self.ids(), self.selected)
    }

    /// Selects Payee `id`, if it still exists.
    fn select(&mut self, id: PayeeId) {
        if self.ids().contains(&id) {
            self.selected = Some(id);
        }
    }

    fn open(&mut self, dialog: PayeesDialog, router: &mut dyn PayeesRouter) {
        if router.open_dialog(dialog) {
            self.open_dialog = Some(dialog);
        }
    }

    fn chord(&self, command: &str) -> &str {
        self.keymap.chord(command).unwrap_or_default()
    }

    fn hints(&self) -> Vec<Hint> {
        let hint = |chord: String, action: String| Hint { chord, action };
        vec![
            hint(
                format!("{}/{}", self.chord(MOVE_DOWN), self.chord(MOVE_UP)),
                msg::payees_hint_row(),
            ),
            hint(
                self.chord(SELECT).to_string(),
                msg::payees_hint_open_transactions(),
            ),
            hint(self.chord(EDIT).to_string(), msg::payees_hint_edit()),
            hint(self.chord(DELETE).to_string(), msg::payees_hint_delete()),
            hint(self.chord(NEW).to_string(), msg::payees_hint_new()),
        ]
    }
}

#[cfg(test)]
mod tests {
    use lib_config::KeyBindingConfig;

    use super::super::test_support::stub;
    use super::*;
    use crate::PayeeRepository as _;
    use crate::fake::{RecordingRouter, RouterCall};
    use crate::keymap::payees_page_keymap;

    struct Page {
        presenter: PayeesPagePresenter,
        router: RecordingRouter,
    }

    impl Page {
        fn new(shared: SharedPorts) -> Self {
            Self::with_keys(shared, &KeyBindingConfig::default())
        }

        fn with_keys(shared: SharedPorts, config: &KeyBindingConfig) -> Self {
            Self {
                presenter: PayeesPagePresenter::new(shared, payees_page_keymap(config)),
                router: RecordingRouter::default(),
            }
        }

        fn send(&mut self, event: PayeesPageEvent) -> Vec<RouterCall> {
            self.presenter.handle(event, &mut self.router);
            self.router.take()
        }

        fn view(&self) -> PayeesPageViewModel {
            self.presenter.view_model()
        }

        fn selected_name(&self) -> Option<String> {
            let view = self.view();
            view.rows
                .into_iter()
                .find(|row| Some(row.id) == view.selected)
                .map(|row| row.name)
        }
    }

    fn open(dialog: PayeesDialog) -> Vec<RouterCall> {
        vec![RouterCall::OpenDialog(dialog)]
    }

    #[test]
    fn open_transactions_asks_the_router_for_the_selected_rows_transactions() {
        let (_, shared) = stub();
        let mut page = Page::new(shared);

        page.send(PayeesPageEvent::Move(Step::Last));
        let selected = page.presenter.selected().expect("a stub row is selected");
        assert_eq!(
            page.send(PayeesPageEvent::OpenTransactions),
            [RouterCall::OpenTransactions(selected)]
        );
    }

    #[test]
    fn the_rows_run_a_to_z_ignoring_case_with_their_category_and_rules() {
        let (_, shared) = stub();
        let page = Page::new(shared);
        let view = page.view();

        let names: Vec<String> = view
            .rows
            .iter()
            .map(|row| row.name.to_lowercase())
            .collect();
        let mut sorted = names.clone();
        sorted.sort();
        assert_eq!(names, sorted);
        let woolworths = view
            .rows
            .iter()
            .find(|row| row.name == "Woolworths")
            .expect("a stub Payee");
        assert_eq!(woolworths.default_category.as_deref(), Some("Groceries"));
        assert_eq!(woolworths.rules, "3 rules");
        let j_smith = view.rows.iter().find(|row| row.name == "J Smith");
        assert_eq!(j_smith.and_then(|row| row.default_category.clone()), None);
        assert!(
            !view
                .rows
                .iter()
                .find(|row| row.name == "Don Quijote")
                .is_some_and(|row| row.is_active)
        );
        assert_eq!(view.empty, None);
    }

    #[test]
    fn the_scope_counts_active_payees_and_those_without_a_default() {
        let (_, shared) = stub();
        let view = Page::new(shared).view();

        assert_eq!(view.status, "22 payees \u{b7} 2 without default category");
        let strong: Vec<&str> = view
            .scope
            .iter()
            .filter(|segment| segment.tag.as_deref() == Some("strong"))
            .map(|segment| segment.text.as_str())
            .collect();
        assert_eq!(strong, ["2"]);
    }

    #[test]
    fn movement_walks_the_rows_and_the_first_is_selected_to_start() {
        let (_, shared) = stub();
        let mut page = Page::new(shared);
        let names: Vec<String> = page.view().rows.into_iter().map(|row| row.name).collect();

        assert_eq!(page.selected_name().as_ref(), names.first());
        page.send(PayeesPageEvent::Move(Step::Next));
        assert_eq!(page.selected_name().as_ref(), names.get(1));
        page.send(PayeesPageEvent::Move(Step::Last));
        assert_eq!(page.selected_name().as_ref(), names.last());
        page.send(PayeesPageEvent::Move(Step::First));
        assert_eq!(page.selected_name().as_ref(), names.first());
    }

    #[test]
    fn reveal_selects_the_payee_and_ignores_one_that_is_gone() {
        let (_, shared) = stub();
        let mut page = Page::new(shared);

        assert_eq!(page.send(PayeesPageEvent::Reveal(PayeeId::seed(22))), []);
        assert_eq!(page.selected_name().as_deref(), Some("AAMI"));
        page.send(PayeesPageEvent::Reveal(PayeeId::new()));
        assert_eq!(page.selected_name().as_deref(), Some("AAMI"));
    }

    #[test]
    fn new_edit_and_delete_open_their_dialogs_on_the_selected_payee() {
        let (_, shared) = stub();
        let mut page = Page::new(shared);
        page.send(PayeesPageEvent::Select(PayeeId::seed(8)));

        assert_eq!(
            page.send(PayeesPageEvent::New),
            open(PayeesDialog::AddPayee)
        );
        assert_eq!(page.view().open_dialog, Some(PayeesDialog::AddPayee));
        page.send(PayeesPageEvent::DialogClosed(None));
        assert_eq!(page.view().open_dialog, None);
        assert_eq!(
            page.send(PayeesPageEvent::Edit),
            open(PayeesDialog::EditPayee(PayeeId::seed(8)))
        );
        assert_eq!(
            page.send(PayeesPageEvent::Delete),
            open(PayeesDialog::DeletePayee(PayeeId::seed(8)))
        );
        assert_eq!(
            page.send(PayeesPageEvent::DeleteRow(PayeeId::seed(1))),
            open(PayeesDialog::DeletePayee(PayeeId::seed(1)))
        );
        assert_eq!(page.selected_name().as_deref(), Some("Woolworths"));
    }

    #[test]
    fn a_save_selects_its_payee_and_a_deleted_selection_falls_to_the_first_row() {
        let (ports, shared) = stub();
        let mut page = Page::new(shared);
        let first = page.selected_name();
        page.send(PayeesPageEvent::Select(PayeeId::seed(3)));

        page.send(PayeesPageEvent::DialogClosed(Some(PayeesOutcome {
            change: PayeeChange::Saved,
            id: PayeeId::seed(20),
            name: "Telstra".into(),
        })));
        assert_eq!(page.selected_name().as_deref(), Some("Telstra"));

        ports.borrow_mut().payees.delete_payee(PayeeId::seed(20));
        assert_eq!(page.selected_name(), first);
    }

    #[test]
    fn an_empty_ledger_says_how_to_add_one_with_the_bound_chord() {
        let (ports, shared) = stub();
        ports.borrow_mut().payees.payees.clear();
        let mut config = KeyBindingConfig::default();
        config.bindings.insert("new".into(), "a".into());
        let page = Page::with_keys(shared, &config);
        let view = page.view();

        assert_eq!(
            view.empty.as_deref(),
            Some("No payees yet. Press a to add one.")
        );
        assert_eq!(view.selected, None);
        let chords: Vec<&str> = view.hints.iter().map(|hint| hint.chord.as_str()).collect();
        assert_eq!(chords, ["j/k", "enter", "e", "d", "a"]);
    }
}
