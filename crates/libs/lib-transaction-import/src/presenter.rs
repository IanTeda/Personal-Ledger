//! The Import "match payees" step's Presenter (6e, #284): one Presenter holds the rows, the
//! selected row, the one open select, "remember" and the continue gate, and builds the snapshot
//! the View draws.
//!
//! - Moving, opening a select, choosing, `n` and toggling "remember" are state edits, using
//!   `lib-list-cursor`; only **continue** runs an Interactor ([`CommitImport`]).
//! - **Continue** is refused while any row needs review, with a status-line message; a refused
//!   commit (a missing or inactive Account, a name since taken) raises an Error Toast and writes
//!   nothing.
//! - The Payees and Categories the selects offer are read when the step opens: it is the only
//!   thing on screen, so nothing changes them under it.

use lib_core::CategoryId;
use lib_list_cursor::Step;
use lib_locale::format::{AmountStyle, format_amount};
use lib_payees::{Payee, PayeeCategory};
use lib_toast::{Notice, ToastKind};
use lib_viper::keymap::{ARROW_DOWN, ARROW_UP, BACK, Keymap, MOVE_DOWN, MOVE_UP, SELECT};
use lib_viper::view::{Choice, DialogHint, SelectView};

use crate::entity::{ImportRow, PayeeChoices, RowPayee, RowStatus, cleaned_name, summary};
use crate::interactor::{CommitImport, CommitRefused, MatchStatement};
use crate::keymap::{CONTINUE, NEW_PAYEE, PICK_CATEGORY, PICK_PAYEE, REMEMBER};
use crate::msg;
use crate::plan::ImportRefused;
use crate::protocols::{ImportPorts, ImportRouter};

/// Which of a row's two selects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowSelect {
    Payee,
    Category,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportEvent {
    /// `j`/`k` and the rest: the selected row, clamped at either end.
    Move(Step),
    /// A click on a row; another row's open select closes.
    SelectRow(usize),
    /// `p`/`c`: the selected row's select.
    Open(RowSelect),
    /// A click on a row's select cell: selects the row and opens it.
    OpenRow(usize, RowSelect),
    /// The open select's highlight, clamped at either end.
    Highlight(Step),
    /// `enter`/`space` in an open select: chooses the highlighted option.
    Commit,
    /// A click on option `index` of the open select.
    Choose(usize),
    /// `n`: the selected row creates a Payee from its cleaned name, if that name is free.
    NewPayee,
    ToggleRemember,
    Continue,
    /// `esc`: closes an open select, else leaves the step.
    Cancel,
}

/// A row as drawn. `payee` and `category` read "choose …" until chosen, which `payee_chosen` and
/// `category_chosen` say, so a Client can mark the empty select.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RowLine {
    pub raw: String,
    pub payee: String,
    pub payee_chosen: bool,
    pub category: String,
    pub category_chosen: bool,
    pub status: RowStatus,
    pub status_label: String,
    pub amount: String,
    pub negative: bool,
}

/// The one open select: its row, which select, and its options as `lib-viper`'s [`SelectView`].
/// An option's value is its index in the list; `chosen` is the row's current value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenSelectView {
    pub row: usize,
    pub which: RowSelect,
    pub select: SelectView<usize>,
    pub highlight: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnHeadings {
    pub raw: String,
    pub payee: String,
    pub category: String,
    pub status: String,
    pub amount: String,
}

/// The footer's figures, worded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SummaryLines {
    pub rows: String,
    pub matched: String,
    pub new_payees: String,
    pub needs_review: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportViewModel {
    pub steps: Vec<String>,
    pub title: String,
    pub subline: String,
    pub columns: ColumnHeadings,
    pub rows: Vec<RowLine>,
    pub selected: usize,
    pub open_select: Option<OpenSelectView>,
    pub summary: SummaryLines,
    pub remember_label: String,
    pub remember: bool,
    pub back: String,
    pub continue_label: String,
    pub can_continue: bool,
    /// The status line's right-hand text.
    pub status: String,
    /// The keys the step takes now, in drawn order; read their chords with [`legend`].
    pub hints: Vec<DialogHint>,
}

/// `hints` as `(chord, action)` pairs, each chord read from `keymap` so a `[keybindings]` override
/// shows. A hint of two commands reads `up/down`.
pub fn legend<E: Clone>(hints: &[DialogHint], keymap: &Keymap<E>) -> Vec<(String, String)> {
    hints
        .iter()
        .map(|hint| {
            let chords: Vec<&str> = hint
                .commands
                .iter()
                .map(|command| keymap.chord(command).unwrap_or_default())
                .collect();
            (chords.join("/"), hint.action.clone())
        })
        .collect()
}

struct OpenSelect {
    row: usize,
    which: RowSelect,
    highlight: usize,
}

pub struct ImportPresenter {
    ports: ImportPorts,
    account_id: lib_core::AccountId,
    file_name: String,
    payees: Vec<Payee>,
    categories: Vec<PayeeCategory>,
    rows: Vec<ImportRow>,
    selected: usize,
    open: Option<OpenSelect>,
    /// "remember new payees' rules for next time", ticked by default as in the handoff.
    remember: bool,
}

impl ImportPresenter {
    /// The step on the source's statement, matched against the stored Payees.
    pub fn new(ports: ImportPorts) -> Self {
        let matched = MatchStatement::run(&*ports.statement, &*ports.payees.borrow());
        let payees = ports.payees.borrow().import_payees();
        let categories = ports.categories.import_categories();
        Self {
            account_id: matched.account_id,
            file_name: matched.file_name,
            rows: matched.rows,
            payees,
            categories,
            ports,
            selected: 0,
            open: None,
            remember: true,
        }
    }

    pub fn handle(&mut self, event: ImportEvent, router: &mut dyn ImportRouter) {
        match event {
            ImportEvent::Move(step) => {
                self.selected = lib_list_cursor::step(self.selected, self.rows.len(), step);
            }
            ImportEvent::SelectRow(index) => self.select_row(index),
            ImportEvent::Open(which) => self.open(self.selected, which),
            ImportEvent::OpenRow(index, which) => self.open(index, which),
            ImportEvent::Highlight(step) => {
                let len = self
                    .options(self.open.as_ref().map(|open| (open.row, open.which)))
                    .len();
                if let Some(open) = self.open.as_mut() {
                    open.highlight = lib_list_cursor::step(open.highlight, len, step);
                }
            }
            ImportEvent::Commit => {
                if let Some(open) = &self.open {
                    self.apply(open.highlight);
                }
            }
            ImportEvent::Choose(index) => self.apply(index),
            ImportEvent::NewPayee => self.create_new_payee(),
            ImportEvent::ToggleRemember => self.remember = !self.remember,
            ImportEvent::Continue => self.commit(router),
            ImportEvent::Cancel => {
                if self.open.take().is_none() {
                    router.close_import(None);
                }
            }
        }
    }

    fn select_row(&mut self, index: usize) {
        if index >= self.rows.len() {
            return;
        }
        self.selected = index;
        if self.open.as_ref().is_some_and(|open| open.row != index) {
            self.open = None;
        }
    }

    fn can_continue(&self) -> bool {
        !self.rows.iter().any(ImportRow::needs_review)
    }

    /// The Payee options for `raw`'s row: create first, then every active Payee.
    fn payee_choices(&self, raw: &str) -> PayeeChoices {
        PayeeChoices::new(&self.payees, raw, create_label)
    }

    /// Labels of the select `target` names (a row and which select); empty when closed.
    fn options(&self, target: Option<(usize, RowSelect)>) -> Vec<String> {
        let Some((row, which)) = target else {
            return Vec::new();
        };
        match which {
            RowSelect::Payee => self
                .rows
                .get(row)
                .map(|row| self.payee_choices(&row.raw).labels)
                .unwrap_or_default(),
            RowSelect::Category => std::iter::once(msg::transaction_import_choose_category())
                .chain(self.categories.iter().map(|leaf| leaf.label.clone()))
                .collect(),
        }
    }

    /// The option index holding `row`'s current value in `which`, if it has one.
    fn current_option(&self, row: &ImportRow, which: RowSelect) -> Option<usize> {
        match which {
            RowSelect::Payee => {
                let choices = self.payee_choices(&row.raw);
                let wanted = match row.payee.as_ref()? {
                    RowPayee::Rule(id) => RowPayee::Existing(*id),
                    other => other.clone(),
                };
                choices.choices.iter().position(|choice| *choice == wanted)
            }
            RowSelect::Category => {
                let id = row.category?;
                self.categories
                    .iter()
                    .position(|leaf| leaf.id == id)
                    .map(|index| index + 1)
            }
        }
    }

    /// Opens row `index`'s `which` on its current value, closing any other.
    fn open(&mut self, index: usize, which: RowSelect) {
        let Some(row) = self.rows.get(index) else {
            return;
        };
        let highlight = self.current_option(row, which).unwrap_or(0);
        self.selected = index;
        self.open = Some(OpenSelect {
            row: index,
            which,
            highlight,
        });
    }

    /// Writes option `index` of the open select into its row and closes it. Picking an existing
    /// Payee pre-fills an empty Category with its default; picking "choose category…" clears it.
    fn apply(&mut self, index: usize) {
        let Some(open) = self.open.take() else {
            return;
        };
        let Some(raw) = self.rows.get(open.row).map(|row| row.raw.clone()) else {
            return;
        };
        match open.which {
            RowSelect::Payee => {
                let Some(choice) = self.payee_choices(&raw).choices.get(index).cloned() else {
                    return;
                };
                let default = choice
                    .id()
                    .and_then(|id| lib_payees::get(&self.payees, id))
                    .and_then(|payee| payee.default_category);
                if let Some(row) = self.rows.get_mut(open.row) {
                    row.payee = Some(choice);
                    if row.category.is_none() {
                        row.category = default;
                    }
                }
            }
            RowSelect::Category => {
                let category = match index.checked_sub(1) {
                    None => None,
                    Some(leaf) => match self.categories.get(leaf) {
                        Some(leaf) => Some(leaf.id),
                        None => return,
                    },
                };
                if let Some(row) = self.rows.get_mut(open.row) {
                    row.category = category;
                }
            }
        }
    }

    fn create_new_payee(&mut self) {
        let Some(row) = self.rows.get_mut(self.selected) else {
            return;
        };
        if let Some(name) =
            cleaned_name(&row.raw).filter(|name| lib_payees::name_available(&self.payees, name))
        {
            row.payee = Some(RowPayee::Create(name));
        }
    }

    fn commit(&mut self, router: &mut dyn ImportRouter) {
        // Read now, not at the call: the store behind the port may be the one that holds them.
        let stored = self.ports.payees.borrow().import_payees();
        let outcome = CommitImport {
            account_id: self.account_id,
            rows: self.rows.clone(),
            remember: self.remember,
        }
        .run(
            &*self.ports.accounts,
            &stored,
            &mut *self.ports.import.borrow_mut(),
        );
        match outcome {
            Ok(committed) => {
                self.open = None;
                router.notify(Notice::Toast(
                    ToastKind::Success,
                    msg::transaction_import_done(count(committed.transactions)),
                ));
                router.close_import(Some(committed));
            }
            Err(CommitRefused::NeedsReview(pending)) => {
                router.notify(Notice::Status(msg::transaction_import_status_pending(
                    count(pending),
                )));
            }
            Err(refusal) => {
                router.notify(Notice::Toast(ToastKind::Error, refusal_text(&refusal)));
            }
        }
    }

    fn payee_label(&self, row: &ImportRow) -> String {
        match &row.payee {
            None => msg::transaction_import_choose_payee(),
            Some(RowPayee::Create(name)) => create_label(name),
            Some(RowPayee::Rule(id) | RowPayee::Existing(id)) => lib_payees::get(&self.payees, *id)
                .map(|payee| payee.name.clone())
                .unwrap_or_default(),
        }
    }

    fn category_label(&self, category: Option<CategoryId>) -> String {
        category
            .and_then(|id| self.categories.iter().find(|leaf| leaf.id == id))
            .map_or_else(msg::transaction_import_choose_category, |leaf| {
                leaf.label.clone()
            })
    }

    pub fn view_model(&self) -> ImportViewModel {
        let figures = summary(&self.rows);
        let can_continue = self.can_continue();
        let selected = self.selected.min(self.rows.len().saturating_sub(1));
        let rows = self
            .rows
            .iter()
            .map(|row| {
                let amount = format_amount(&row.amount, AmountStyle::own());
                RowLine {
                    raw: row.raw.clone(),
                    payee: self.payee_label(row),
                    payee_chosen: row.payee.is_some(),
                    category: self.category_label(row.category),
                    category_chosen: row.category.is_some(),
                    status: row.status(),
                    status_label: status_label(row.status()),
                    amount: amount.text,
                    negative: amount.is_negative,
                }
            })
            .collect();
        ImportViewModel {
            steps: vec![
                msg::transaction_import_step_upload(),
                msg::transaction_import_step_match(),
                msg::transaction_import_step_confirm(),
            ],
            title: msg::transaction_import_title(),
            subline: msg::transaction_import_subline(count(self.rows.len()), &self.file_name),
            columns: ColumnHeadings {
                raw: msg::transaction_import_column_raw(),
                payee: msg::transaction_import_column_payee(),
                category: msg::transaction_import_column_category(),
                status: msg::transaction_import_column_status(),
                amount: msg::transaction_import_column_amount(),
            },
            rows,
            selected,
            open_select: self.open_select_view(),
            summary: SummaryLines {
                rows: msg::transaction_import_summary_rows(count(figures.rows)),
                matched: msg::transaction_import_summary_matched(&figures.rule_matched.to_string()),
                new_payees: msg::transaction_import_summary_new(count(figures.new_payees)),
                needs_review: msg::transaction_import_summary_review(count(figures.needs_review)),
            },
            remember_label: msg::transaction_import_remember(),
            remember: self.remember,
            back: msg::transaction_import_back(),
            continue_label: msg::transaction_import_continue(),
            can_continue,
            status: if can_continue {
                msg::transaction_import_status_ready()
            } else {
                msg::transaction_import_status_pending(count(figures.needs_review))
            },
            hints: self.hints(),
        }
    }

    fn open_select_view(&self) -> Option<OpenSelectView> {
        let open = self.open.as_ref()?;
        let row = self.rows.get(open.row)?;
        let label = match open.which {
            RowSelect::Payee => msg::transaction_import_column_payee(),
            RowSelect::Category => msg::transaction_import_column_category(),
        };
        Some(OpenSelectView {
            row: open.row,
            which: open.which,
            select: SelectView {
                label,
                options: self
                    .options(Some((open.row, open.which)))
                    .into_iter()
                    .enumerate()
                    .map(|(value, label)| Choice { value, label })
                    .collect(),
                chosen: self.current_option(row, open.which),
                focused: true,
            },
            highlight: open.highlight,
        })
    }

    fn hints(&self) -> Vec<DialogHint> {
        let hint = |commands, action| DialogHint { commands, action };
        if self.open.is_some() {
            return vec![
                hint(
                    &[ARROW_UP, ARROW_DOWN],
                    msg::transaction_import_hint_choose(),
                ),
                hint(&[SELECT], msg::transaction_import_hint_select()),
                hint(&[BACK], msg::transaction_import_hint_close()),
            ];
        }
        vec![
            hint(&[MOVE_DOWN, MOVE_UP], msg::transaction_import_hint_row()),
            hint(&[PICK_PAYEE], msg::transaction_import_hint_payee()),
            hint(&[PICK_CATEGORY], msg::transaction_import_hint_category()),
            hint(
                &[NEW_PAYEE],
                msg::transaction_import_hint_create_new_payee(),
            ),
            hint(&[REMEMBER], msg::transaction_import_hint_remember_rules()),
            hint(&[CONTINUE], msg::transaction_import_hint_continue()),
            hint(&[BACK], msg::transaction_import_hint_back()),
        ]
    }
}

fn create_label(name: &str) -> String {
    msg::transaction_import_create_payee("+", name)
}

fn count(count: usize) -> i64 {
    i64::try_from(count).unwrap_or(i64::MAX)
}

fn status_label(status: RowStatus) -> String {
    match status {
        RowStatus::RuleMatch => msg::transaction_import_status_rule_match(),
        RowStatus::NewPayee => msg::transaction_import_status_new_payee(),
        RowStatus::Matched => msg::transaction_import_status_matched(),
        RowStatus::NeedsReview => msg::transaction_import_status_needs_review(),
    }
}

/// A refused commit, worded. `NeedsReview` never gets here: it is a status-line message.
fn refusal_text(refusal: &CommitRefused) -> String {
    match refusal {
        CommitRefused::AccountGone => msg::transaction_import_error_account_gone(),
        CommitRefused::AccountInactive => msg::transaction_import_error_account_inactive(),
        CommitRefused::Refused(ImportRefused::PayeeNameTaken(name)) => {
            msg::transaction_import_error_name_taken(name)
        }
        CommitRefused::Refused(ImportRefused::AliasTaken(alias)) => {
            msg::transaction_import_error_alias_taken(alias)
        }
        CommitRefused::NeedsReview(pending) => {
            msg::transaction_import_status_pending(count(*pending))
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use chrono::NaiveDate;
    use lib_config::KeyBindingConfig;
    use lib_core::PayeeId;

    use super::*;
    use crate::fake::{InMemoryImport, InMemoryImportReads, RecordingRouter, stub_ports};
    use crate::keymap::{import_keymap, import_select_keymap};
    use crate::plan::ImportCommitted;

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 9, 28).expect("a September day")
    }

    struct Harness {
        presenter: ImportPresenter,
        store: Rc<RefCell<InMemoryImport>>,
        router: RecordingRouter,
    }

    impl Harness {
        fn new() -> Self {
            let (ports, store) = stub_ports(today());
            Self::on(ports, store)
        }

        fn on(ports: ImportPorts, store: Rc<RefCell<InMemoryImport>>) -> Self {
            lib_locale::init_with_layers("en-US", &[msg::LAYER]);
            Self {
                presenter: ImportPresenter::new(ports),
                store,
                router: RecordingRouter::default(),
            }
        }

        fn send(&mut self, event: ImportEvent) {
            self.presenter.handle(event, &mut self.router);
        }

        /// Gives each row still needing review the first Category, by the keys that do it.
        fn review_every_row(&mut self) {
            let pending: Vec<usize> = self
                .presenter
                .rows
                .iter()
                .enumerate()
                .filter(|(_, row)| row.needs_review())
                .map(|(index, _)| index)
                .collect();
            for index in pending {
                self.send(ImportEvent::SelectRow(index));
                self.send(ImportEvent::Open(RowSelect::Category));
                self.send(ImportEvent::Highlight(Step::Next));
                self.send(ImportEvent::Commit);
            }
        }

        fn row_of(&self, raw: &str) -> usize {
            self.presenter
                .rows
                .iter()
                .position(|row| row.raw == raw)
                .expect("a seeded row")
        }
    }

    #[test]
    fn the_step_opens_on_the_seeded_statement_with_four_rows_to_review() {
        let step = Harness::new();
        let view = step.presenter.view_model();

        assert_eq!(view.rows.len(), 18);
        assert_eq!(view.selected, 0);
        assert!(view.remember, "remember is ticked by default");
        assert!(!view.can_continue);
        assert_eq!(view.summary.needs_review, "4 to review");
        assert_eq!(view.summary.matched, "14 matched by rule");
        assert_eq!(view.status, "Review 4 rows before continuing");
        assert_eq!(view.rows[0].payee, "Woolworths");
        assert_eq!(view.rows[0].status, RowStatus::RuleMatch);
        assert_eq!(view.rows[4].payee, "+ create \"Aussie Candle Co\"");
        assert_eq!(view.rows[4].category, "choose category…");
        assert_eq!(view.rows[4].status_label, "needs review");
        assert!(view.open_select.is_none());
    }

    #[test]
    fn moving_walks_the_rows_and_stops_at_either_end() {
        let mut step = Harness::new();
        step.send(ImportEvent::Move(Step::Prev));
        assert_eq!(step.presenter.view_model().selected, 0);
        step.send(ImportEvent::Move(Step::Last));
        step.send(ImportEvent::Move(Step::Next));
        assert_eq!(step.presenter.view_model().selected, 17);
        step.send(ImportEvent::SelectRow(3));
        assert_eq!(step.presenter.view_model().selected, 3);
    }

    #[test]
    fn continue_is_refused_while_a_row_needs_review_and_writes_nothing() {
        let mut step = Harness::new();
        let before = step.store.borrow().clone();

        step.send(ImportEvent::Continue);

        assert_eq!(
            step.router.notices,
            [Notice::Status(
                "Review 4 rows before continuing".to_string()
            )]
        );
        assert!(step.router.closed.is_empty());
        assert_eq!(*step.store.borrow(), before);
    }

    #[test]
    fn a_select_opens_on_the_current_value_and_a_click_chooses_an_option() {
        let mut step = Harness::new();
        let aussie = step.row_of("SP AUSSIE CANDLE CO");
        step.send(ImportEvent::OpenRow(aussie, RowSelect::Payee));

        let open = step.presenter.view_model().open_select.expect("open");
        assert_eq!((open.row, open.which), (aussie, RowSelect::Payee));
        assert_eq!(open.highlight, 0, "the create option is the current value");
        assert_eq!(open.select.chosen, Some(0));
        assert_eq!(
            open.select.options[0].label,
            "+ create \"Aussie Candle Co\""
        );

        let coles = open
            .select
            .options
            .iter()
            .find(|choice| choice.label == "Coles")
            .map(|choice| choice.value)
            .expect("an active Payee is offered");
        step.send(ImportEvent::Choose(coles));

        let view = step.presenter.view_model();
        assert!(view.open_select.is_none(), "choosing closes the select");
        assert_eq!(view.rows[aussie].payee, "Coles");
        assert_ne!(
            view.rows[aussie].category, "choose category…",
            "an existing Payee pre-fills its default Category"
        );
        assert_eq!(view.rows[aussie].status, RowStatus::Matched);
    }

    #[test]
    fn the_keys_of_an_open_select_move_the_highlight_choose_and_close() {
        let mut step = Harness::new();
        step.send(ImportEvent::Open(RowSelect::Category));
        let highlight = |step: &Harness| {
            step.presenter
                .view_model()
                .open_select
                .expect("open")
                .highlight
        };
        // Row 0 is Woolworths, so the select opens on its default Category.
        let opened = highlight(&step);
        assert!(opened > 0);
        step.send(ImportEvent::Highlight(Step::Next));
        step.send(ImportEvent::Highlight(Step::Next));
        assert_eq!(highlight(&step), opened + 2);

        step.send(ImportEvent::Cancel);
        let view = step.presenter.view_model();
        assert!(view.open_select.is_none());
        assert!(step.router.closed.is_empty(), "esc closed the select only");

        step.send(ImportEvent::Open(RowSelect::Category));
        step.send(ImportEvent::Highlight(Step::Next));
        step.send(ImportEvent::Commit);
        let chosen = step.presenter.view_model().rows[0].category.clone();
        // Option `opened + 1` is Category `opened`, the lead option being option 0.
        assert_eq!(chosen, step.presenter.categories[opened].label);

        step.send(ImportEvent::Open(RowSelect::Category));
        step.send(ImportEvent::Commit);
        assert_eq!(
            step.presenter.view_model().rows[0].category,
            chosen,
            "committing the highlighted current value keeps it"
        );
        step.send(ImportEvent::Open(RowSelect::Category));
        step.send(ImportEvent::Highlight(Step::First));
        step.send(ImportEvent::Commit);
        assert_eq!(
            step.presenter.view_model().rows[0].category,
            "choose category…",
            "the lead option clears the Category"
        );
    }

    #[test]
    fn selecting_another_row_closes_an_open_select() {
        let mut step = Harness::new();
        step.send(ImportEvent::Open(RowSelect::Payee));
        step.send(ImportEvent::SelectRow(0));
        assert!(step.presenter.view_model().open_select.is_some());
        step.send(ImportEvent::SelectRow(1));
        assert!(step.presenter.view_model().open_select.is_none());
    }

    #[test]
    fn n_creates_a_payee_only_when_the_cleaned_name_is_free() {
        let mut step = Harness::new();
        let aussie = step.row_of("SP AUSSIE CANDLE CO");
        let smith = step.row_of("TFR TO J SMITH REF PAYMENT");

        // "J Smith" is an active Payee, so its name is taken and `n` changes nothing.
        step.send(ImportEvent::SelectRow(smith));
        let before = step.presenter.rows[smith].clone();
        step.send(ImportEvent::NewPayee);
        assert_eq!(step.presenter.rows[smith], before);

        // "Aussie Candle Co" is free: pick an existing Payee, then `n` puts the create back.
        step.send(ImportEvent::SelectRow(aussie));
        step.presenter.rows[aussie].payee = Some(RowPayee::Existing(PayeeId::seed(2)));
        step.send(ImportEvent::NewPayee);
        assert_eq!(
            step.presenter.rows[aussie].payee,
            Some(RowPayee::Create("Aussie Candle Co".to_string()))
        );
    }

    #[test]
    fn continue_writes_the_import_toasts_and_leaves_the_step() {
        let mut step = Harness::new();
        step.review_every_row();
        assert!(step.presenter.view_model().can_continue);
        assert_eq!(step.presenter.view_model().status, "ready to import");

        step.send(ImportEvent::Continue);

        assert_eq!(
            step.router.notices,
            [Notice::Toast(
                ToastKind::Success,
                "Imported 18 transactions".to_string()
            )]
        );
        assert_eq!(
            step.router.closed,
            [Some(ImportCommitted {
                transactions: 18,
                new_payees: 3
            })]
        );
        let store = step.store.borrow();
        assert_eq!(store.transactions.transactions.len(), 18);
        let candle = lib_payees::find_by_name(&store.payees.payees, "Aussie Candle Co")
            .and_then(|id| lib_payees::get(&store.payees.payees, id))
            .expect("the new Payee is stored");
        assert_eq!(candle.aliases, ["AUSSIE CANDLE CO"]);
    }

    #[test]
    fn remember_unticked_adds_no_aliases() {
        let mut step = Harness::new();
        step.review_every_row();
        step.send(ImportEvent::ToggleRemember);
        assert!(!step.presenter.view_model().remember);

        step.send(ImportEvent::Continue);

        let store = step.store.borrow();
        let candle = lib_payees::find_by_name(&store.payees.payees, "Aussie Candle Co")
            .and_then(|id| lib_payees::get(&store.payees.payees, id))
            .expect("the new Payee is stored");
        assert!(candle.aliases.is_empty());
    }

    fn refused_with_accounts(accounts: Vec<crate::ImportAccount>) -> Harness {
        let (mut ports, store) = stub_ports(today());
        ports.accounts = Rc::new(InMemoryImportReads {
            accounts,
            categories: lib_payees::fake::stub_categories(),
        });
        let mut step = Harness::on(ports, store);
        step.review_every_row();
        step.send(ImportEvent::Continue);
        step
    }

    #[test]
    fn a_missing_account_is_refused_with_a_toast_and_writes_nothing() {
        let step = refused_with_accounts(Vec::new());

        assert_eq!(
            step.router.notices,
            [Notice::Toast(
                ToastKind::Error,
                "This statement's account no longer exists, so nothing was imported.".to_string()
            )]
        );
        assert!(step.router.closed.is_empty());
        assert!(step.store.borrow().transactions.transactions.is_empty());
        assert_eq!(
            step.store.borrow().payees.payees,
            lib_payees::default_payees()
        );
    }

    #[test]
    fn an_inactive_account_is_refused_with_a_toast_and_writes_nothing() {
        let step = refused_with_accounts(vec![crate::ImportAccount {
            id: crate::SeededStatement::STUB_ACCOUNT,
            name: "ANZ Everyday".to_string(),
            is_active: false,
        }]);

        assert_eq!(
            step.router.notices,
            [Notice::Toast(
                ToastKind::Error,
                "This statement's account is inactive, so nothing was imported.".to_string()
            )]
        );
        assert!(step.router.closed.is_empty());
        assert!(step.store.borrow().transactions.transactions.is_empty());
    }

    #[test]
    fn a_name_taken_since_the_step_opened_is_refused_and_writes_nothing() {
        let mut step = Harness::new();
        step.review_every_row();
        // Another writer takes the new Payee's name after the rows were matched.
        step.store.borrow_mut().payees.payees.push(Payee {
            id: PayeeId::new(),
            name: "Aussie Candle Co".to_string(),
            aliases: Vec::new(),
            default_category: None,
            is_active: true,
        });
        let before = step.store.borrow().clone();

        step.send(ImportEvent::Continue);

        assert_eq!(
            step.router.notices,
            [Notice::Toast(
                ToastKind::Error,
                "A payee named \"Aussie Candle Co\" already exists, so nothing was imported."
                    .to_string()
            )]
        );
        assert_eq!(*step.store.borrow(), before);
    }

    #[test]
    fn cancel_leaves_the_step_without_writing() {
        let mut step = Harness::new();
        step.send(ImportEvent::Cancel);

        assert_eq!(step.router.closed, [None]);
        assert!(step.router.notices.is_empty());
        assert!(step.store.borrow().transactions.transactions.is_empty());
    }

    /// Every key is a keymap entry and every hint's commands are bound in the keymap it is read
    /// through, so the legend shows a chord for each.
    #[test]
    fn every_hint_names_bound_commands_and_follows_an_override() {
        let mut step = Harness::new();
        let rows = import_keymap(&KeyBindingConfig::default());
        let legend_rows = legend(&step.presenter.view_model().hints, &rows);
        assert_eq!(legend_rows.len(), 7);
        assert!(legend_rows.iter().all(|(chord, action)| {
            !chord.is_empty() && !chord.split('/').any(str::is_empty) && !action.is_empty()
        }));
        assert_eq!(legend_rows[0], ("j/k".to_string(), "row".to_string()));
        assert_eq!(legend_rows[1], ("p".to_string(), "payee".to_string()));

        step.send(ImportEvent::Open(RowSelect::Payee));
        let select = import_select_keymap(&KeyBindingConfig::default());
        let legend_select = legend(&step.presenter.view_model().hints, &select);
        assert_eq!(
            legend_select,
            [
                ("up/down".to_string(), "choose".to_string()),
                ("enter".to_string(), "select".to_string()),
                ("esc".to_string(), "close".to_string()),
            ]
        );

        let mut config = KeyBindingConfig::default();
        config
            .bindings
            .insert(NEW_PAYEE.to_string(), "N".to_string());
        let moved = import_keymap(&config);
        step.send(ImportEvent::Cancel);
        let legend_moved = legend(&step.presenter.view_model().hints, &moved);
        assert_eq!(legend_moved[3].0, "N");
    }
}
