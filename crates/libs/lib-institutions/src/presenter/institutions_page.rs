//! The Institutions Settings page (`docs/ux/desktop-mockups/16-settings/README.md`'s "2a resting
//! state"): every Institution in the order added, with its Account type, and `edit · delete` per
//! row. Edit and delete have no Dialog yet, so they say so on the status line.
//!
//! The selection is an Institution id. It may name an Institution that has since gone, so it is
//! resolved on every read: a missing one leaves the first row selected.

use lib_core::InstitutionId;
use lib_list_cursor::{Step, resolve, step_id};
use lib_toast::Notice;
use lib_viper::keymap::{DELETE, EDIT, Keymap, MOVE_DOWN, MOVE_UP, NEW};

use crate::msg;
use crate::protocols::{InstitutionsDialog, InstitutionsOutcome, InstitutionsRouter, SharedPorts};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstitutionsPageEvent {
    /// `j`/`k`, `g g`, `G`, `Ctrl-d`/`Ctrl-u` and their arrow, Home/End and Page keys.
    Move(Step),
    /// A click on a row.
    Select(InstitutionId),
    /// New opens Add; Edit and Delete act on the selected Institution.
    New,
    Edit,
    Delete,
    /// A row's own **edit** or **delete**: selects the row, then asks for its Dialog.
    EditRow(InstitutionId),
    DeleteRow(InstitutionId),
    /// The open Dialog closed, with what its confirm did or `None` when cancelled.
    DialogClosed(Option<InstitutionsOutcome>),
}

/// The page as drawn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstitutionsPageViewModel {
    /// The heading's meta and the status line's right side: `6 institutions`.
    pub scope: String,
    pub add_button: String,
    pub columns: InstitutionColumns,
    /// In the order added.
    pub rows: Vec<InstitutionLine>,
    pub row_edit: String,
    pub row_delete: String,
    pub selected: Option<InstitutionId>,
    pub hints: Vec<Hint>,
    /// The Dialog the page last opened, until it closes.
    pub open_dialog: Option<InstitutionsDialog>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstitutionColumns {
    pub name: String,
    pub account_types: String,
    pub actions: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstitutionLine {
    pub id: InstitutionId,
    pub name: String,
    pub account_type: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hint {
    pub chord: String,
    pub action: String,
}

pub struct InstitutionsPagePresenter {
    ports: SharedPorts,
    keymap: Keymap<InstitutionsPageEvent>,
    selected: Option<InstitutionId>,
    open_dialog: Option<InstitutionsDialog>,
}

impl InstitutionsPagePresenter {
    /// `keymap` is the one the Client looks keys up in, so the legend shows the chords it binds.
    pub fn new(ports: SharedPorts, keymap: Keymap<InstitutionsPageEvent>) -> Self {
        Self {
            ports,
            keymap,
            selected: None,
            open_dialog: None,
        }
    }

    pub fn handle(&mut self, event: InstitutionsPageEvent, router: &mut dyn InstitutionsRouter) {
        match event {
            InstitutionsPageEvent::Move(step) => {
                self.selected = step_id(&self.ids(), self.selected(), step);
            }
            InstitutionsPageEvent::Select(id) => self.select(id),
            InstitutionsPageEvent::New => self.open(router),
            InstitutionsPageEvent::Edit => {
                if self.selected().is_some() {
                    router.notify(Notice::Status(msg::institutions_edit_not_built()));
                }
            }
            InstitutionsPageEvent::Delete => {
                if self.selected().is_some() {
                    router.notify(Notice::Status(msg::institutions_delete_not_built()));
                }
            }
            InstitutionsPageEvent::EditRow(id) => {
                self.select(id);
                router.notify(Notice::Status(msg::institutions_edit_not_built()));
            }
            InstitutionsPageEvent::DeleteRow(id) => {
                self.select(id);
                router.notify(Notice::Status(msg::institutions_delete_not_built()));
            }
            InstitutionsPageEvent::DialogClosed(outcome) => {
                self.open_dialog = None;
                // A save selects its Institution.
                if let Some(InstitutionsOutcome { id, .. }) = outcome {
                    self.select(id);
                }
            }
        }
    }

    pub fn view_model(&self) -> InstitutionsPageViewModel {
        let rows: Vec<InstitutionLine> = self
            .ports
            .borrow()
            .institutions()
            .institutions()
            .into_iter()
            .map(|institution| InstitutionLine {
                id: institution.id,
                name: institution.name,
                account_type: institution.account_type,
            })
            .collect();
        let selected = resolve(rows.iter().map(|row| row.id), self.selected);
        InstitutionsPageViewModel {
            scope: msg::institutions_count(i64::try_from(rows.len()).unwrap_or(i64::MAX)),
            add_button: msg::institutions_add_button("+"),
            columns: InstitutionColumns {
                name: lib_locale::msg::column_institution(),
                account_types: msg::institutions_column_account_types(),
                actions: lib_locale::msg::column_actions(),
            },
            rows,
            row_edit: msg::institutions_row_edit(),
            row_delete: msg::institutions_row_delete(),
            selected,
            hints: self.hints(),
            open_dialog: self.open_dialog,
        }
    }

    /// The Institution ids in the order added, as the rows run.
    fn ids(&self) -> Vec<InstitutionId> {
        self.ports
            .borrow()
            .institutions()
            .institutions()
            .into_iter()
            .map(|institution| institution.id)
            .collect()
    }

    /// The selected Institution: the stored id while it still exists, else the first row.
    fn selected(&self) -> Option<InstitutionId> {
        resolve(self.ids(), self.selected)
    }

    /// Selects Institution `id`, if it still exists.
    fn select(&mut self, id: InstitutionId) {
        if self.ids().contains(&id) {
            self.selected = Some(id);
        }
    }

    fn open(&mut self, router: &mut dyn InstitutionsRouter) {
        if router.open_dialog(InstitutionsDialog::AddInstitution) {
            self.open_dialog = Some(InstitutionsDialog::AddInstitution);
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
                msg::institutions_hint_row(),
            ),
            hint(self.chord(EDIT).to_string(), msg::institutions_hint_edit()),
            hint(
                self.chord(DELETE).to_string(),
                msg::institutions_hint_delete(),
            ),
            hint(self.chord(NEW).to_string(), msg::institutions_hint_new()),
        ]
    }
}

#[cfg(test)]
mod tests {
    use lib_config::KeyBindingConfig;

    use super::*;
    use crate::fake::{InMemoryPorts, RecordingRouter, RouterCall};
    use crate::keymap::institutions_page_keymap;
    use crate::presenter::test_support::stub;
    use std::cell::RefCell;
    use std::rc::Rc;

    struct Page {
        presenter: InstitutionsPagePresenter,
        router: RecordingRouter,
    }

    impl Page {
        fn new(ports: SharedPorts) -> Self {
            Self::with_keys(ports, &KeyBindingConfig::default())
        }

        fn with_keys(ports: SharedPorts, config: &KeyBindingConfig) -> Self {
            Self {
                presenter: InstitutionsPagePresenter::new(ports, institutions_page_keymap(config)),
                router: RecordingRouter::default(),
            }
        }

        fn send(&mut self, event: InstitutionsPageEvent) -> Vec<RouterCall> {
            self.presenter.handle(event, &mut self.router);
            self.router.take()
        }

        fn view(&self) -> InstitutionsPageViewModel {
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

    fn status(text: &str) -> Vec<RouterCall> {
        vec![RouterCall::Notice(Notice::Status(text.to_string()))]
    }

    #[test]
    fn rows_run_in_the_order_added_with_the_first_selected() {
        let (_, shared) = stub();
        let page = Page::new(shared);
        let view = page.view();

        assert_eq!(view.scope, "6 institutions");
        assert_eq!(view.add_button, "+ Add institution");
        assert_eq!(view.rows[0].name, "ANZ Banking Group");
        assert_eq!(view.rows[0].account_type, "savings \u{b7} offset");
        assert_eq!(page.selected_name().as_deref(), Some("ANZ Banking Group"));
    }

    #[test]
    fn movement_walks_the_rows_and_stops_at_the_ends() {
        let (_, shared) = stub();
        let mut page = Page::new(shared);

        page.send(InstitutionsPageEvent::Move(Step::Next));
        assert_eq!(page.selected_name().as_deref(), Some("American Express"));
        page.send(InstitutionsPageEvent::Move(Step::Last));
        assert_eq!(page.selected_name().as_deref(), Some("Superannuation Fund"));
        page.send(InstitutionsPageEvent::Move(Step::Next));
        assert_eq!(page.selected_name().as_deref(), Some("Superannuation Fund"));
        page.send(InstitutionsPageEvent::Move(Step::First));
        assert_eq!(page.selected_name().as_deref(), Some("ANZ Banking Group"));
    }

    #[test]
    fn new_opens_the_add_dialog_until_it_closes() {
        let (_, shared) = stub();
        let mut page = Page::new(shared);

        assert_eq!(
            page.send(InstitutionsPageEvent::New),
            vec![RouterCall::OpenDialog(InstitutionsDialog::AddInstitution)]
        );
        assert_eq!(
            page.view().open_dialog,
            Some(InstitutionsDialog::AddInstitution)
        );
        page.send(InstitutionsPageEvent::DialogClosed(None));
        assert_eq!(page.view().open_dialog, None);
    }

    #[test]
    fn edit_and_delete_say_they_are_not_built_on_the_selected_row_or_a_clicked_one() {
        let (_, shared) = stub();
        let mut page = Page::new(shared);

        assert_eq!(
            page.send(InstitutionsPageEvent::Edit),
            status("edit institution \u{2014} not yet built")
        );
        assert_eq!(
            page.send(InstitutionsPageEvent::Delete),
            status("delete institution \u{2014} not yet built")
        );
        let amex = page.view().rows[1].id;
        assert_eq!(
            page.send(InstitutionsPageEvent::DeleteRow(amex)),
            status("delete institution \u{2014} not yet built")
        );
        assert_eq!(page.selected_name().as_deref(), Some("American Express"));
    }

    #[test]
    fn a_save_selects_its_institution_and_a_missing_selection_falls_to_the_first_row() {
        let (ports, shared) = stub();
        let mut page = Page::new(shared);
        let westpac = page.view().rows[3].id;

        page.send(InstitutionsPageEvent::DialogClosed(Some(
            InstitutionsOutcome {
                id: westpac,
                name: "Westpac Banking".into(),
            },
        )));
        assert_eq!(page.selected_name().as_deref(), Some("Westpac Banking"));

        ports.borrow_mut().institutions.institutions.remove(3);
        assert_eq!(page.selected_name().as_deref(), Some("ANZ Banking Group"));
    }

    #[test]
    fn the_legend_shows_the_bound_chords() {
        let (_, shared) = stub();
        let mut config = KeyBindingConfig::default();
        config.bindings.insert("new".into(), "a".into());
        let page = Page::with_keys(shared, &config);

        let chords: Vec<String> = page
            .view()
            .hints
            .into_iter()
            .map(|hint| hint.chord)
            .collect();
        assert_eq!(chords, ["j/k", "e", "d", "a"]);
    }

    #[test]
    fn an_empty_ledger_selects_nothing() {
        crate::presenter::test_support::locale();
        let (_, shared): (Rc<RefCell<InMemoryPorts>>, SharedPorts) =
            InMemoryPorts::default().shared();
        let page = Page::new(shared);

        assert_eq!(page.view().selected, None);
        assert_eq!(page.view().scope, "0 institutions");
    }
}
