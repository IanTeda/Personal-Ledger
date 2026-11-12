//! The Document Types Settings page (`docs/ux/desktop-mockups/16-settings/README.md`'s 16p, as
//! amended by #479): a kicker with **+ Add document type** over one table of the Ledger's Document
//! Types in type-filter order, then four one-sentence notes. Other carries a `default` tag and no
//! **remove** action.
//!
//! The selection is a type id, so it follows its row across a rename or a reorder. It may name a
//! type that has since gone, so it is resolved on every read: a removed type leaves the first row
//! selected.

use lib_core::DocumentTypeId;
use lib_list_cursor::{Step, resolve, step_id};
use lib_locale::Segment;
use lib_toast::Notice;
use lib_viper::keymap::{EDIT, Keymap, MOVE_DOWN, MOVE_UP, NEW};
use lib_viper::presenter::{PagePresenter, Presenter};
use lib_viper::view::{Hint, count};

use super::tracks_label;
use crate::interactor::MoveDocumentType;
use crate::keymap::{MOVE_TYPE_DOWN, MOVE_TYPE_UP, REMOVE_TYPE};
use crate::msg;
use crate::protocols::{
    DocumentTypesDialog, DocumentTypesOutcome, DocumentTypesRouter, SharedRepository, TypeChange,
};
use crate::types::{DocumentTypeRow, RemindLead, all_files, get, typed_files};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocumentTypesPageEvent {
    /// `j`/`k`, `g g`, `G`, `Ctrl-d`/`Ctrl-u` and their arrow, Home/End and Page keys.
    Move(Step),
    /// A click on a row.
    Select(DocumentTypeId),
    /// New opens Add; Edit and Remove act on the selected type.
    New,
    Edit,
    Remove,
    /// `J`/`K` (1/-1): the selected type a place down or up the order, staying selected.
    Reorder(isize),
    /// A row's own **edit** or **remove**: selects the row, then opens its Dialog.
    EditRow(DocumentTypeId),
    RemoveRow(DocumentTypeId),
    /// The open Dialog closed, with what its confirm did or `None` when cancelled.
    DialogClosed(Option<DocumentTypesOutcome>),
}

/// The page as drawn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentTypesPageViewModel {
    /// The heading's meta and the status line's note: `9 types · 392 of 412 files typed`.
    pub scope: String,
    pub kicker: String,
    pub add_button: String,
    pub columns: TypeColumns,
    /// In the user's order.
    pub rows: Vec<TypeLine>,
    /// The four notes under the table, each with its `strong` lead.
    pub notes: Vec<Vec<Segment>>,
    /// The tag beside Other's name.
    pub default_flag: String,
    pub row_edit: String,
    pub row_remove: String,
    pub selected: Option<DocumentTypeId>,
    pub hints: Vec<Hint>,
    /// The Dialog the page last opened, until it closes.
    pub open_dialog: Option<DocumentTypesDialog>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeColumns {
    pub type_name: String,
    pub tracks_date: String,
    pub remind: String,
    pub financial_year: String,
    pub files: String,
    pub actions: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeLine {
    pub id: DocumentTypeId,
    pub name: String,
    /// `None` where the type tracks no date, drawn as a dash.
    pub tracks_date: Option<String>,
    /// `None` where the type tracks no date or raises no reminder: Remind is inert without one.
    pub remind: Option<String>,
    /// `Yes` or `No`.
    pub financial_year: String,
    pub files: u32,
    pub is_default: bool,
    /// What a cell with nothing to say shows.
    pub none: String,
}

pub struct DocumentTypesPagePresenter {
    ports: SharedRepository,
    keymap: Keymap<DocumentTypesPageEvent>,
    selected: Option<DocumentTypeId>,
    open_dialog: Option<DocumentTypesDialog>,
}

impl DocumentTypesPagePresenter {
    /// Takes keys through `keymap` from now on, after a `[keybindings]` rebind, so the legend shows
    /// its chords too.
    pub fn set_keymap(&mut self, keymap: Keymap<DocumentTypesPageEvent>) {
        self.keymap = keymap;
    }

    /// `keymap` is the one the Client looks keys up in, so the legend shows the chords it binds.
    pub fn new(ports: SharedRepository, keymap: Keymap<DocumentTypesPageEvent>) -> Self {
        Self {
            ports,
            keymap,
            selected: None,
            open_dialog: None,
        }
    }
}

impl Presenter for DocumentTypesPagePresenter {
    type Event = DocumentTypesPageEvent;
    type ViewModel = DocumentTypesPageViewModel;
    type Router<'r> = dyn DocumentTypesRouter + 'r;

    fn handle(&mut self, event: DocumentTypesPageEvent, router: &mut Self::Router<'_>) {
        match event {
            DocumentTypesPageEvent::Move(step) => {
                self.selected = step_id(&self.ids(), self.selected(), step);
            }
            DocumentTypesPageEvent::Select(id) => self.select(id),
            DocumentTypesPageEvent::New => self.open(DocumentTypesDialog::Add, router),
            DocumentTypesPageEvent::Edit => {
                if let Some(id) = self.selected() {
                    self.open(DocumentTypesDialog::Edit(id), router);
                }
            }
            DocumentTypesPageEvent::Remove => {
                if let Some(id) = self.selected() {
                    self.remove(id, router);
                }
            }
            DocumentTypesPageEvent::Reorder(delta) => {
                if let Some(id) = self.selected() {
                    MoveDocumentType { id, delta }.run(&mut *self.ports.borrow_mut());
                    self.selected = Some(id);
                }
            }
            DocumentTypesPageEvent::EditRow(id) => {
                self.select(id);
                self.open(DocumentTypesDialog::Edit(id), router);
            }
            DocumentTypesPageEvent::RemoveRow(id) => {
                self.select(id);
                self.remove(id, router);
            }
            DocumentTypesPageEvent::DialogClosed(outcome) => {
                self.open_dialog = None;
                // A save or a removal selects the type the Dialog named.
                if let Some(DocumentTypesOutcome {
                    change: TypeChange::Saved | TypeChange::Removed,
                    select: Some(id),
                }) = outcome
                {
                    self.select(id);
                }
            }
        }
    }

    fn view_model(&self) -> DocumentTypesPageViewModel {
        let types = self.ports.borrow().document_types();
        let none = msg::documents_types_none();
        let rows: Vec<TypeLine> = types
            .iter()
            .map(|row| TypeLine {
                id: row.id,
                name: row.name.clone(),
                tracks_date: row.tracks_date.map(tracks_label),
                remind: row.tracks_date.and(row.remind).map(remind_label),
                financial_year: if row.financial_year {
                    msg::documents_types_yes()
                } else {
                    msg::documents_types_no()
                },
                files: row.files,
                is_default: row.is_default,
                none: none.clone(),
            })
            .collect();
        DocumentTypesPageViewModel {
            scope: scope_text(&types),
            kicker: msg::documents_types_kicker(),
            add_button: msg::documents_types_add_button("+"),
            columns: TypeColumns {
                type_name: msg::documents_types_column_type(),
                tracks_date: msg::documents_types_column_tracks_date(),
                remind: msg::documents_types_column_remind(),
                financial_year: msg::documents_types_column_financial_year(),
                files: msg::documents_types_column_files(),
                actions: lib_locale::msg::column_actions(),
            },
            selected: resolve(rows.iter().map(|row| row.id), self.selected),
            rows,
            notes: vec![
                msg::documents_types_note_tracks_date(),
                msg::documents_types_note_financial_year(),
                msg::documents_types_note_removal(),
                msg::documents_types_note_default(),
            ],
            default_flag: msg::documents_types_flag_default(),
            row_edit: msg::documents_types_row_edit(),
            row_remove: msg::documents_types_row_remove(),
            hints: self.hints(),
            open_dialog: self.open_dialog,
        }
    }
}

impl PagePresenter for DocumentTypesPagePresenter {
    type Outcome = DocumentTypesOutcome;

    fn dialog_closed(outcome: Option<DocumentTypesOutcome>) -> Option<DocumentTypesPageEvent> {
        Some(DocumentTypesPageEvent::DialogClosed(outcome))
    }

    fn keymap(&self) -> &Keymap<DocumentTypesPageEvent> {
        &self.keymap
    }
}

impl DocumentTypesPagePresenter {
    /// The type ids in the user's order.
    fn ids(&self) -> Vec<DocumentTypeId> {
        self.ports
            .borrow()
            .document_types()
            .iter()
            .map(|row| row.id)
            .collect()
    }

    /// The selected type: the stored id while it still exists, else the first row.
    fn selected(&self) -> Option<DocumentTypeId> {
        resolve(self.ids(), self.selected)
    }

    /// Selects type `id`, if it still exists.
    fn select(&mut self, id: DocumentTypeId) {
        if self.ids().contains(&id) {
            self.selected = Some(id);
        }
    }

    fn open(&mut self, dialog: DocumentTypesDialog, router: &mut dyn DocumentTypesRouter) {
        if router.open_dialog(dialog) {
            self.open_dialog = Some(dialog);
        }
    }

    /// Other, the Default, has no remove action: it gets a notice instead, and the status line says
    /// so. A type that has gone opens nothing.
    fn remove(&mut self, id: DocumentTypeId, router: &mut dyn DocumentTypesRouter) {
        let types = self.ports.borrow().document_types();
        let Some(row) = get(&types, id) else {
            return;
        };
        if row.is_default {
            router.notify(Notice::Status(msg::documents_types_hint_default_kept()));
            self.open(DocumentTypesDialog::DefaultNotice, router);
        } else {
            self.open(DocumentTypesDialog::Remove(id), router);
        }
    }

    fn hints(&self) -> Vec<Hint> {
        vec![
            Hint::new(
                self.keymap.hint_chords(&[MOVE_DOWN, MOVE_UP]),
                msg::documents_types_hint_row(),
            ),
            Hint::new(
                self.keymap.hint_chords(&[MOVE_TYPE_DOWN, MOVE_TYPE_UP]),
                msg::documents_types_hint_reorder(),
            ),
            Hint::new(
                self.keymap.hint_chord(EDIT),
                msg::documents_types_hint_edit(),
            ),
            Hint::new(
                self.keymap.hint_chord(REMOVE_TYPE),
                msg::documents_types_hint_remove(),
            ),
            Hint::new(self.keymap.hint_chord(NEW), msg::documents_types_hint_new()),
        ]
    }
}

/// The heading's meta as plain text: `9 types · 392 of 412 files typed`.
fn scope_text(types: &[DocumentTypeRow]) -> String {
    msg::documents_types_scope(
        &msg::documents_types_count(count(types.len())),
        &msg::documents_types_files_typed(
            i64::from(all_files(types)),
            &typed_files(types).to_string(),
        ),
    )
}

/// The text of a Remind cell.
fn remind_label(lead: RemindLead) -> String {
    match lead {
        RemindLead::Days(days) => msg::documents_types_remind_days(i64::from(days)),
        RemindLead::Months(months) => msg::documents_types_remind_months(i64::from(months)),
    }
}

#[cfg(test)]
mod tests {
    use lib_config::KeyBindingConfig;
    use lib_toast::Notice;

    use super::super::test_support::sample;
    use super::*;
    use crate::fake::{RecordingRouter, RouterCall};
    use crate::keymap::document_types_page_keymap;

    struct Page {
        presenter: DocumentTypesPagePresenter,
        router: RecordingRouter,
    }

    impl Page {
        fn new(shared: SharedRepository) -> Self {
            Self::with_keys(shared, &KeyBindingConfig::default())
        }

        fn with_keys(shared: SharedRepository, config: &KeyBindingConfig) -> Self {
            Self {
                presenter: DocumentTypesPagePresenter::new(
                    shared,
                    document_types_page_keymap(config),
                ),
                router: RecordingRouter::default(),
            }
        }

        fn send(&mut self, event: DocumentTypesPageEvent) -> Vec<RouterCall> {
            self.presenter.handle(event, &mut self.router);
            self.router.take()
        }

        fn view(&self) -> DocumentTypesPageViewModel {
            self.presenter.view_model()
        }

        fn selected_name(&self) -> Option<String> {
            let view = self.view();
            view.rows
                .into_iter()
                .find(|row| Some(row.id) == view.selected)
                .map(|row| row.name)
        }

        fn names(&self) -> Vec<String> {
            self.view().rows.into_iter().map(|row| row.name).collect()
        }
    }

    fn open(dialog: DocumentTypesDialog) -> Vec<RouterCall> {
        vec![RouterCall::OpenDialog(dialog)]
    }

    #[test]
    fn the_rows_run_in_the_users_order_with_their_cells() {
        let (_, shared) = sample();
        let view = Page::new(shared).view();

        assert_eq!(view.rows.len(), 9);
        assert_eq!(view.rows.last().map(|row| row.name.as_str()), Some("Other"));
        let receipts = &view.rows[0];
        assert_eq!(receipts.tracks_date, None);
        assert_eq!(receipts.remind, None);
        assert_eq!(receipts.financial_year, "Yes");
        assert_eq!(receipts.files, 186);
        assert_eq!(receipts.none, "—");
        let dated = view
            .rows
            .iter()
            .find(|row| row.tracks_date.is_some())
            .expect("a dated sample type");
        assert!(dated.tracks_date.is_some());
        assert_eq!(view.columns.type_name, "Type");
        assert_eq!(view.columns.actions, "Actions");
        assert_eq!(view.notes.len(), 4);
        assert_eq!(view.default_flag, "default");
    }

    #[test]
    fn a_reminder_reads_in_days_or_months_and_only_where_a_date_is_tracked() {
        let (documents, shared) = sample();
        {
            let mut held = documents.borrow_mut();
            held.types[0].tracks_date = Some(crate::types::TracksDate::Ends);
            held.types[0].remind = Some(RemindLead::Months(3));
            held.types[1].tracks_date = None;
            held.types[1].remind = Some(RemindLead::Days(30));
        }
        let view = Page::new(shared).view();

        assert_eq!(view.rows[0].remind.as_deref(), Some("3 months before"));
        assert_eq!(view.rows[1].remind, None, "inert without a date");
    }

    #[test]
    fn the_scope_counts_the_types_and_the_files_typed() {
        let (_, shared) = sample();

        assert_eq!(
            Page::new(shared).view().scope,
            "9 types \u{b7} 392 of 412 files typed"
        );
    }

    #[test]
    fn a_single_type_reads_singular() {
        let (documents, shared) = sample();
        documents.borrow_mut().types.truncate(1);

        assert!(Page::new(shared).view().scope.starts_with("1 type \u{b7} "));
    }

    #[test]
    fn movement_walks_the_rows_and_the_first_is_selected_to_start() {
        let (_, shared) = sample();
        let mut page = Page::new(shared);
        let names = page.names();

        assert_eq!(page.selected_name().as_ref(), names.first());
        page.send(DocumentTypesPageEvent::Move(Step::Next));
        assert_eq!(page.selected_name().as_ref(), names.get(1));
        page.send(DocumentTypesPageEvent::Move(Step::Last));
        assert_eq!(page.selected_name().as_ref(), names.last());
        page.send(DocumentTypesPageEvent::Move(Step::Prev));
        assert_eq!(page.selected_name().as_ref(), names.get(7));
        page.send(DocumentTypesPageEvent::Move(Step::First));
        assert_eq!(page.selected_name().as_ref(), names.first());
    }

    #[test]
    fn reorder_moves_the_selected_type_a_place_and_it_stays_selected() {
        let (documents, shared) = sample();
        let mut page = Page::new(shared);

        page.send(DocumentTypesPageEvent::Reorder(1));
        assert_eq!(page.names()[..2], ["Statements", "Receipts"]);
        assert_eq!(page.selected_name().as_deref(), Some("Receipts"));
        assert_eq!(documents.borrow().types[1].name, "Receipts");
        page.send(DocumentTypesPageEvent::Reorder(-1));
        assert_eq!(page.names()[0], "Receipts");
        // Already first: nothing moves.
        page.send(DocumentTypesPageEvent::Reorder(-1));
        assert_eq!(page.names()[0], "Receipts");
    }

    #[test]
    fn other_reorders_like_any_row() {
        let (_, shared) = sample();
        let mut page = Page::new(shared);
        page.send(DocumentTypesPageEvent::Move(Step::Last));

        page.send(DocumentTypesPageEvent::Reorder(-1));

        assert_eq!(page.names()[7], "Other");
        assert_eq!(page.selected_name().as_deref(), Some("Other"));
    }

    #[test]
    fn new_edit_and_remove_open_their_dialogs_on_the_selected_type() {
        let (_, shared) = sample();
        let mut page = Page::new(shared);
        page.send(DocumentTypesPageEvent::Select(DocumentTypeId::seed(3)));

        assert_eq!(
            page.send(DocumentTypesPageEvent::New),
            open(DocumentTypesDialog::Add)
        );
        assert_eq!(page.view().open_dialog, Some(DocumentTypesDialog::Add));
        page.send(DocumentTypesPageEvent::DialogClosed(None));
        assert_eq!(page.view().open_dialog, None);
        assert_eq!(
            page.send(DocumentTypesPageEvent::Edit),
            open(DocumentTypesDialog::Edit(DocumentTypeId::seed(3)))
        );
        assert_eq!(
            page.send(DocumentTypesPageEvent::Remove),
            open(DocumentTypesDialog::Remove(DocumentTypeId::seed(3)))
        );
        assert_eq!(
            page.send(DocumentTypesPageEvent::RemoveRow(DocumentTypeId::seed(1))),
            open(DocumentTypesDialog::Remove(DocumentTypeId::seed(1)))
        );
        assert_eq!(page.selected_name().as_deref(), Some("Receipts"));
        assert_eq!(
            page.send(DocumentTypesPageEvent::EditRow(DocumentTypeId::seed(4))),
            open(DocumentTypesDialog::Edit(DocumentTypeId::seed(4)))
        );
        assert_eq!(page.selected_name().as_deref(), Some("Insurance"));
    }

    #[test]
    fn remove_on_other_says_why_and_opens_the_notice() {
        let (_, shared) = sample();
        let mut page = Page::new(shared);
        page.send(DocumentTypesPageEvent::Move(Step::Last));

        assert_eq!(
            page.send(DocumentTypesPageEvent::Remove),
            [
                RouterCall::Notice(Notice::Status("Other can't be removed".into())),
                RouterCall::OpenDialog(DocumentTypesDialog::DefaultNotice),
            ]
        );
    }

    #[test]
    fn a_dialog_the_router_could_not_open_is_not_recorded_as_open() {
        struct Refusing(RecordingRouter);
        impl lib_toast::Notify for Refusing {
            fn notify(&mut self, notice: Notice) {
                self.0.notify(notice);
            }
        }
        impl DocumentTypesRouter for Refusing {
            fn open_dialog(&mut self, _dialog: DocumentTypesDialog) -> bool {
                false
            }
        }
        let (_, shared) = sample();
        let mut presenter = DocumentTypesPagePresenter::new(
            shared,
            document_types_page_keymap(&KeyBindingConfig::default()),
        );

        presenter.handle(
            DocumentTypesPageEvent::New,
            &mut Refusing(RecordingRouter::default()),
        );

        assert_eq!(presenter.view_model().open_dialog, None);
    }

    #[test]
    fn a_save_or_a_removal_selects_the_type_the_dialog_named() {
        let (documents, shared) = sample();
        let mut page = Page::new(shared);

        page.send(DocumentTypesPageEvent::DialogClosed(Some(
            DocumentTypesOutcome {
                change: TypeChange::Saved,
                select: Some(DocumentTypeId::seed(5)),
            },
        )));
        assert_eq!(
            page.selected_name().as_deref(),
            Some("Warranties & manuals")
        );
        page.send(DocumentTypesPageEvent::DialogClosed(Some(
            DocumentTypesOutcome {
                change: TypeChange::Removed,
                select: Some(DocumentTypeId::seed(2)),
            },
        )));
        assert_eq!(page.selected_name().as_deref(), Some("Statements"));

        // A selection that has gone falls to the first row.
        documents
            .borrow_mut()
            .types
            .retain(|row| row.id != DocumentTypeId::seed(2));
        assert_eq!(page.selected_name().as_deref(), Some("Receipts"));
    }

    #[test]
    fn selecting_a_type_that_is_gone_changes_nothing() {
        let (_, shared) = sample();
        let mut page = Page::new(shared);
        page.send(DocumentTypesPageEvent::Select(DocumentTypeId::seed(4)));

        page.send(DocumentTypesPageEvent::Select(DocumentTypeId::seed(999)));

        assert_eq!(page.selected_name().as_deref(), Some("Insurance"));
    }

    #[test]
    fn the_hints_read_the_keys_bound_now() {
        let (_, shared) = sample();
        let chords = |page: &Page| -> Vec<String> {
            page.view()
                .hints
                .into_iter()
                .map(|hint| hint.chord)
                .collect()
        };
        assert_eq!(
            chords(&Page::new(shared.clone())),
            ["j/k", "J/K", "e", "x", "n"]
        );

        let mut config = KeyBindingConfig::default();
        config.bindings.insert("new".into(), "a".into());
        config.bindings.insert(REMOVE_TYPE.into(), "d".into());
        assert_eq!(
            chords(&Page::with_keys(shared, &config)),
            ["j/k", "J/K", "e", "d", "a"]
        );
    }
}
