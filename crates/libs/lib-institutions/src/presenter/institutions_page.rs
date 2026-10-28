//! The Institutions Settings page (`docs/ux/desktop-mockups/16-settings/README.md`'s "2a resting
//! state"): every Institution in the order added, with its Products, and `edit · delete` per
//! row. Edit opens the form on the Institution and delete its Delete Dialog.
//!
//! The selection is an Institution id. It may name an Institution that has since gone, so it is
//! resolved on every read: a missing one leaves the first row selected. A delete moves it to the
//! next row, or the previous when the deleted one was last.

use lib_core::InstitutionId;
use lib_list_cursor::{Step, resolve, step_id};
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
    /// A right-click on a row: selects it and opens its menu.
    OpenMenu(InstitutionId),
    CloseMenu,
    /// An entry of the open menu.
    Menu(MenuAction),
    /// The open Dialog closed, with what its confirm did or `None` when cancelled.
    DialogClosed(Option<InstitutionsOutcome>),
}

/// An action a row's menu offers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuAction {
    EditInstitution,
    DeleteInstitution,
}

impl MenuAction {
    /// In menu order, the destructive one last.
    const ALL: [Self; 2] = [Self::EditInstitution, Self::DeleteInstitution];

    /// A stable name (`edit-institution`), for a Client's element ids and tests.
    pub fn slug(self) -> &'static str {
        match self {
            Self::EditInstitution => "edit-institution",
            Self::DeleteInstitution => "delete-institution",
        }
    }

    fn entry(self) -> MenuEntry {
        match self {
            Self::EditInstitution => MenuEntry {
                action: self,
                label: msg::institutions_menu_edit(),
                destructive: false,
            },
            Self::DeleteInstitution => MenuEntry {
                action: self,
                label: msg::institutions_menu_delete(),
                destructive: true,
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuEntry {
    pub action: MenuAction,
    pub label: String,
    pub destructive: bool,
}

/// The menu open on a row, its entries in order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RowMenu {
    pub row: InstitutionId,
    pub entries: Vec<MenuEntry>,
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
    /// The menu a right-click opened, until any other event.
    pub menu: Option<RowMenu>,
    /// The Dialog the page last opened, until it closes.
    pub open_dialog: Option<InstitutionsDialog>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstitutionColumns {
    pub name: String,
    pub products: String,
    pub actions: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstitutionLine {
    pub id: InstitutionId,
    pub name: String,
    pub products: String,
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
    /// The row whose menu is open.
    menu: Option<InstitutionId>,
    open_dialog: Option<InstitutionsDialog>,
}

impl InstitutionsPagePresenter {
    /// `keymap` is the one the Client looks keys up in, so the legend shows the chords it binds.
    pub fn new(ports: SharedPorts, keymap: Keymap<InstitutionsPageEvent>) -> Self {
        Self {
            ports,
            keymap,
            selected: None,
            menu: None,
            open_dialog: None,
        }
    }

    pub fn handle(&mut self, event: InstitutionsPageEvent, router: &mut dyn InstitutionsRouter) {
        // Any event but the menu's own closes it, as a key or click elsewhere does.
        let menu = self.menu.take();
        match event {
            InstitutionsPageEvent::Move(step) => {
                self.selected = step_id(&self.ids(), self.selected(), step);
            }
            InstitutionsPageEvent::Select(id) => self.select(id),
            InstitutionsPageEvent::New => {
                self.open_dialog(InstitutionsDialog::AddInstitution, router)
            }
            InstitutionsPageEvent::Edit => {
                if let Some(id) = self.selected() {
                    self.open_dialog(InstitutionsDialog::EditInstitution(id), router);
                }
            }
            InstitutionsPageEvent::Delete => {
                if let Some(id) = self.selected() {
                    self.open_dialog(InstitutionsDialog::DeleteInstitution(id), router);
                }
            }
            InstitutionsPageEvent::EditRow(id) => {
                self.select(id);
                self.open_dialog(InstitutionsDialog::EditInstitution(id), router);
            }
            InstitutionsPageEvent::DeleteRow(id) => {
                self.select(id);
                self.open_dialog(InstitutionsDialog::DeleteInstitution(id), router);
            }
            InstitutionsPageEvent::OpenMenu(id) => {
                if self.ids().contains(&id) {
                    self.select(id);
                    self.menu = Some(id);
                }
            }
            InstitutionsPageEvent::CloseMenu => {}
            InstitutionsPageEvent::Menu(action) => {
                if let Some(id) = menu {
                    let event = match action {
                        MenuAction::EditInstitution => InstitutionsPageEvent::EditRow(id),
                        MenuAction::DeleteInstitution => InstitutionsPageEvent::DeleteRow(id),
                    };
                    self.handle(event, router);
                }
            }
            InstitutionsPageEvent::DialogClosed(outcome) => {
                self.open_dialog = None;
                // A save selects its Institution; a delete, being gone, selects the neighbour the
                // Dialog read when it confirmed.
                if let Some(InstitutionsOutcome { id, next, .. }) = outcome {
                    if self.ids().contains(&id) {
                        self.select(id);
                    } else if let Some(neighbour) = next {
                        self.select(neighbour);
                    }
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
                products: institution.products,
            })
            .collect();
        let selected = resolve(rows.iter().map(|row| row.id), self.selected);
        let menu = self
            .menu
            .filter(|id| rows.iter().any(|row| row.id == *id))
            .map(|row| RowMenu {
                row,
                entries: MenuAction::ALL.into_iter().map(MenuAction::entry).collect(),
            });
        InstitutionsPageViewModel {
            scope: msg::institutions_count(i64::try_from(rows.len()).unwrap_or(i64::MAX)),
            add_button: msg::institutions_add_button("+"),
            columns: InstitutionColumns {
                name: lib_locale::msg::column_institution(),
                products: msg::institutions_column_products(),
                actions: lib_locale::msg::column_actions(),
            },
            rows,
            row_edit: msg::institutions_row_edit(),
            row_delete: msg::institutions_row_delete(),
            selected,
            hints: self.hints(),
            menu,
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

    fn open_dialog(&mut self, dialog: InstitutionsDialog, router: &mut dyn InstitutionsRouter) {
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

    #[test]
    fn rows_run_in_the_order_added_with_the_first_selected() {
        let (_, shared) = stub();
        let page = Page::new(shared);
        let view = page.view();

        assert_eq!(view.scope, "7 institutions");
        assert_eq!(view.add_button, "+ Add institution");
        assert_eq!(view.rows[0].name, "Personal");
        assert_eq!(view.rows[0].products, "cash");
        assert_eq!(view.rows[1].name, "ANZ Banking Group");
        assert_eq!(view.rows[1].products, "savings \u{b7} offset");
        assert_eq!(page.selected_name().as_deref(), Some("Personal"));
    }

    #[test]
    fn movement_walks_the_rows_and_stops_at_the_ends() {
        let (_, shared) = stub();
        let mut page = Page::new(shared);

        page.send(InstitutionsPageEvent::Move(Step::Next));
        assert_eq!(page.selected_name().as_deref(), Some("ANZ Banking Group"));
        page.send(InstitutionsPageEvent::Move(Step::Last));
        assert_eq!(page.selected_name().as_deref(), Some("Superannuation Fund"));
        page.send(InstitutionsPageEvent::Move(Step::Next));
        assert_eq!(page.selected_name().as_deref(), Some("Superannuation Fund"));
        page.send(InstitutionsPageEvent::Move(Step::First));
        assert_eq!(page.selected_name().as_deref(), Some("Personal"));
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
    fn edit_opens_the_form_on_the_selected_row_or_a_clicked_one() {
        let (_, shared) = stub();
        let mut page = Page::new(shared);
        let personal = page.view().rows[0].id;
        let amex = page.view().rows[2].id;

        assert_eq!(
            page.send(InstitutionsPageEvent::Edit),
            vec![RouterCall::OpenDialog(InstitutionsDialog::EditInstitution(
                personal
            ))]
        );
        page.send(InstitutionsPageEvent::DialogClosed(None));
        assert_eq!(
            page.send(InstitutionsPageEvent::EditRow(amex)),
            vec![RouterCall::OpenDialog(InstitutionsDialog::EditInstitution(
                amex
            ))]
        );
        assert_eq!(page.selected_name().as_deref(), Some("American Express"));
        assert_eq!(
            page.view().open_dialog,
            Some(InstitutionsDialog::EditInstitution(amex))
        );
    }

    #[test]
    fn delete_opens_the_delete_dialog_on_the_selected_row_or_a_clicked_one() {
        let (_, shared) = stub();
        let mut page = Page::new(shared);
        let personal = page.view().rows[0].id;
        let amex = page.view().rows[2].id;

        assert_eq!(
            page.send(InstitutionsPageEvent::Delete),
            vec![RouterCall::OpenDialog(
                InstitutionsDialog::DeleteInstitution(personal)
            )]
        );
        page.send(InstitutionsPageEvent::DialogClosed(None));
        assert_eq!(
            page.send(InstitutionsPageEvent::DeleteRow(amex)),
            vec![RouterCall::OpenDialog(
                InstitutionsDialog::DeleteInstitution(amex)
            )]
        );
        assert_eq!(page.selected_name().as_deref(), Some("American Express"));
        assert_eq!(
            page.view().open_dialog,
            Some(InstitutionsDialog::DeleteInstitution(amex))
        );
    }

    /// Confirms the delete of the row at `at` as the Dialog does: the row goes, then the page is
    /// told.
    fn delete(page: &mut Page, ports: &Rc<RefCell<InMemoryPorts>>, at: usize) {
        let row = page.view().rows[at].clone();
        let ids: Vec<_> = page.view().rows.iter().map(|row| row.id).collect();
        page.send(InstitutionsPageEvent::DeleteRow(row.id));
        ports.borrow_mut().institutions.institutions.remove(at);
        page.send(InstitutionsPageEvent::DialogClosed(Some(
            InstitutionsOutcome {
                id: row.id,
                name: row.name,
                next: ids
                    .get(at + 1)
                    .or(at.checked_sub(1).and_then(|b| ids.get(b)))
                    .copied(),
            },
        )));
    }

    #[test]
    fn a_delete_selects_the_next_row_or_the_previous_when_it_was_last() {
        let (ports, shared) = stub();
        let mut page = Page::new(shared);

        delete(&mut page, &ports, 2);
        assert_eq!(
            page.selected_name().as_deref(),
            Some("Vanguard Investments")
        );

        page.send(InstitutionsPageEvent::Move(Step::Last));
        delete(&mut page, &ports, 5);
        assert_eq!(
            page.selected_name().as_deref(),
            Some("Cryptocurrency Exchange")
        );
        assert_eq!(page.view().rows.len(), 5);
    }

    #[test]
    fn a_cancelled_delete_keeps_the_selection_on_the_row() {
        let (_, shared) = stub();
        let mut page = Page::new(shared);
        let amex = page.view().rows[2].id;

        page.send(InstitutionsPageEvent::DeleteRow(amex));
        page.send(InstitutionsPageEvent::DialogClosed(None));

        assert_eq!(page.selected_name().as_deref(), Some("American Express"));
    }

    #[test]
    fn a_right_click_selects_the_row_and_opens_its_menu_whose_entry_runs_on_it() {
        let (_, shared) = stub();
        let mut page = Page::new(shared);
        let amex = page.view().rows[2].id;

        page.send(InstitutionsPageEvent::OpenMenu(amex));
        let menu = page.view().menu.expect("the menu is open");
        assert_eq!(menu.row, amex);
        let labels: Vec<_> = menu
            .entries
            .iter()
            .map(|entry| (entry.label.as_str(), entry.destructive))
            .collect();
        assert_eq!(
            labels,
            [
                ("Edit institution\u{2026}", false),
                ("Delete institution\u{2026}", true)
            ]
        );
        assert_eq!(page.selected_name().as_deref(), Some("American Express"));

        assert_eq!(
            page.send(InstitutionsPageEvent::Menu(MenuAction::EditInstitution)),
            vec![RouterCall::OpenDialog(InstitutionsDialog::EditInstitution(
                amex
            ))]
        );
        assert_eq!(page.view().menu, None);
    }

    #[test]
    fn the_menus_delete_opens_the_delete_dialog_and_any_other_event_closes_the_menu() {
        let (_, shared) = stub();
        let mut page = Page::new(shared);
        let amex = page.view().rows[2].id;

        page.send(InstitutionsPageEvent::OpenMenu(amex));
        assert_eq!(
            page.send(InstitutionsPageEvent::Menu(MenuAction::DeleteInstitution)),
            vec![RouterCall::OpenDialog(
                InstitutionsDialog::DeleteInstitution(amex)
            )]
        );
        page.send(InstitutionsPageEvent::OpenMenu(amex));
        page.send(InstitutionsPageEvent::Move(Step::Next));
        assert_eq!(page.view().menu, None);
        assert!(
            page.send(InstitutionsPageEvent::Menu(MenuAction::EditInstitution))
                .is_empty(),
            "with no menu open an entry does nothing"
        );
    }

    #[test]
    fn a_save_selects_its_institution_and_a_missing_selection_falls_to_the_first_row() {
        let (ports, shared) = stub();
        let mut page = Page::new(shared);
        let westpac = page.view().rows[4].id;

        page.send(InstitutionsPageEvent::DialogClosed(Some(
            InstitutionsOutcome {
                id: westpac,
                name: "Westpac Banking".into(),
                next: None,
            },
        )));
        assert_eq!(page.selected_name().as_deref(), Some("Westpac Banking"));

        ports.borrow_mut().institutions.institutions.remove(4);
        assert_eq!(page.selected_name().as_deref(), Some("Personal"));
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
