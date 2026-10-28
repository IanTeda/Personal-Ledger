//! The Inventory Settings page (`docs/ux/desktop-mockups/16-settings/README.md`'s 16q): a LOCATIONS
//! kicker over one NAME table, each Location a row with its Area and Item counts and, while it is
//! open, its Areas beneath it in area-tab order. Address and cover are kept on the register in
//! Inventory, so no column shows them.
//!
//! Selection is one flat list over the visible Location and Area rows, by id, so it follows its row
//! across a rename or a reorder. It shows the first row until the first move, and the first row
//! again once its row has gone or is hidden. The folds are the Locations shown open, and none starts
//! open. A row carries no buttons (the Units table principles): a double-click edits it and a
//! right-click opens its menu, whose entries do what the keys do.

use std::collections::HashSet;

use lib_core::{AreaId, LocationId};
use lib_list_cursor::{Step, resolve, step_id};
use lib_locale::format::upper;
use lib_toast::{Notice, ToastKind};
use lib_viper::keymap::{EDIT, Keymap, MOVE_DOWN, MOVE_UP, NEW};

use super::count;
use crate::interactor::MoveArea;
use crate::keymap::{ADD_AREA, COLLAPSE, EXPAND, MOVE_AREA_DOWN, MOVE_AREA_UP, REMOVE};
use crate::protocols::{
    InventoryChange, InventoryDialog, InventoryOutcome, InventoryRouter, InventoryRow,
    SharedInventory,
};
use crate::{Inventory, Location, msg};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InventoryPageEvent {
    /// `j`/`k`, `g g`, `G`, `Ctrl-d`/`Ctrl-u` and their arrow, Home/End and Page keys.
    Move(Step),
    /// A click on a row.
    Select(InventoryRow),
    /// A click on a Location's ▾/▸.
    Toggle(LocationId),
    /// A double-click on a row: selects it and opens its Edit Dialog.
    EditRow(InventoryRow),
    /// New opens Add location; Edit and Remove act on the selected row.
    New,
    Edit,
    Remove,
    /// `a`: Add area to the selected Location (or the selected Area's).
    AddArea,
    /// `J`/`K`: move the selected Area a place down (`1`) or up (`-1`) its Location.
    MoveArea(isize),
    /// `right` opens a closed Location, or steps from an open one into its first Area.
    Expand,
    /// `left` closes an open Location, or climbs from an Area to its Location.
    Collapse,
    /// A right-click on a row: selects it and opens its menu.
    OpenMenu(InventoryRow),
    CloseMenu,
    /// An entry of the open menu.
    Menu(MenuAction),
    /// The open Dialog closed, with what its confirm did or `None` when cancelled.
    DialogClosed(Option<InventoryOutcome>),
}

/// An entry of a row's context menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuAction {
    AddArea,
    EditLocation,
    RemoveLocation,
    EditArea,
    MoveUp,
    MoveDown,
    RemoveArea,
}

impl MenuAction {
    /// A row's entries in menu order, the destructive one last.
    pub fn for_row(row: InventoryRow) -> &'static [Self] {
        match row {
            InventoryRow::Location(_) => &[Self::AddArea, Self::EditLocation, Self::RemoveLocation],
            InventoryRow::Area(_) => &[
                Self::EditArea,
                Self::MoveUp,
                Self::MoveDown,
                Self::RemoveArea,
            ],
        }
    }

    fn label(self) -> String {
        match self {
            Self::AddArea => msg::inventory_menu_add_area(),
            Self::EditLocation => msg::inventory_menu_edit_location(),
            Self::RemoveLocation => msg::inventory_menu_remove_location(),
            Self::EditArea => msg::inventory_menu_edit_area(),
            Self::MoveUp => msg::inventory_menu_move_up(),
            Self::MoveDown => msg::inventory_menu_move_down(),
            Self::RemoveArea => msg::inventory_menu_remove_area(),
        }
    }

    /// Drawn in red below a rule.
    fn is_destructive(self) -> bool {
        matches!(self, Self::RemoveLocation | Self::RemoveArea)
    }
}

/// The page as drawn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InventoryPageViewModel {
    /// The heading's meta and the status line's right-hand text: `3 locations · 14 areas · 241 items`.
    pub scope: String,
    /// `LOCATIONS · 3`.
    pub kicker: String,
    /// The square button beside the kicker that adds a Location.
    pub add_button: String,
    /// The table's NAME header.
    pub column_name: String,
    /// The rows the folds show, in screen order.
    pub rows: Vec<InventoryLine>,
    /// What shows in place of the table while there are no Locations.
    pub empty: String,
    pub note: String,
    pub selected: Option<InventoryRow>,
    pub menu: Option<MenuView>,
    pub hints: Vec<Hint>,
    /// The Dialog the page last opened, until it closes.
    pub open_dialog: Option<InventoryDialog>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InventoryLine {
    pub row: InventoryRow,
    pub name: String,
    /// A Location's `10 areas · 214 items`.
    pub note: Option<String>,
    /// A Location's disclosure glyph, ▾ while it is open.
    pub disclosure: Option<String>,
}

/// The menu open on a row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuView {
    pub row: InventoryRow,
    pub entries: Vec<MenuEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuEntry {
    pub action: MenuAction,
    pub label: String,
    pub destructive: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hint {
    pub chord: String,
    pub action: String,
}

pub struct InventoryPagePresenter {
    inventory: SharedInventory,
    keymap: Keymap<InventoryPageEvent>,
    selected: Option<InventoryRow>,
    expanded: HashSet<LocationId>,
    menu: Option<InventoryRow>,
    open_dialog: Option<InventoryDialog>,
}

impl InventoryPagePresenter {
    /// `keymap` is the one the Client looks keys up in, so the legend shows the chords it binds.
    pub fn new(inventory: SharedInventory, keymap: Keymap<InventoryPageEvent>) -> Self {
        Self {
            inventory,
            keymap,
            selected: None,
            expanded: HashSet::new(),
            menu: None,
            open_dialog: None,
        }
    }

    /// Whether the page takes `event` from its keymap, rather than the Client's frame having it:
    /// `left` with nothing to close or climb to is the Settings index's again.
    pub fn takes(&self, event: &InventoryPageEvent) -> bool {
        *event != InventoryPageEvent::Collapse || self.left_is_local()
    }

    /// Whether a row menu is open, which `Esc` closes before it leaves the page.
    pub fn menu_open(&self) -> bool {
        self.menu.is_some()
    }

    pub fn handle(&mut self, event: InventoryPageEvent, router: &mut dyn InventoryRouter) {
        // Any key or click other than the one that opens a menu puts the open one away.
        if !matches!(
            event,
            InventoryPageEvent::OpenMenu(_) | InventoryPageEvent::Menu(_)
        ) {
            self.menu = None;
        }
        match event {
            InventoryPageEvent::Move(step) => self.step(step),
            InventoryPageEvent::Select(row) => self.select(Some(row)),
            InventoryPageEvent::Toggle(id) => self.toggle(id),
            InventoryPageEvent::EditRow(row) => {
                self.select(Some(row));
                self.open_edit(row, router);
            }
            InventoryPageEvent::New => self.open_add_location(router),
            InventoryPageEvent::Edit => {
                if let Some(row) = self.selected() {
                    self.open_edit(row, router);
                }
            }
            InventoryPageEvent::Remove => {
                if let Some(row) = self.selected() {
                    self.open_remove(row, router);
                }
            }
            InventoryPageEvent::AddArea => self.add_area_for_selection(router),
            InventoryPageEvent::MoveArea(delta) => {
                if let Some(InventoryRow::Area(area)) = self.selected() {
                    self.move_area(area, delta);
                }
            }
            InventoryPageEvent::Expand => self.step_in(),
            InventoryPageEvent::Collapse => self.step_out(),
            InventoryPageEvent::OpenMenu(row) => {
                self.select(Some(row));
                self.menu = Some(row);
            }
            InventoryPageEvent::CloseMenu => self.menu = None,
            InventoryPageEvent::Menu(action) => {
                if let Some(row) = self.menu.take() {
                    self.menu_action(action, row, router);
                }
            }
            InventoryPageEvent::DialogClosed(outcome) => {
                self.open_dialog = None;
                if let Some(outcome) = outcome {
                    self.land(outcome);
                }
            }
        }
    }

    pub fn view_model(&self) -> InventoryPageViewModel {
        let inventory = self.inventory.borrow().inventory();
        let rows = visible_rows(&inventory.locations, &self.expanded);
        let selected = resolve(rows.iter().copied(), self.selected);
        InventoryPageViewModel {
            scope: scope_text(&inventory),
            kicker: upper(&msg::inventory_kicker(
                &inventory.location_count().to_string(),
            )),
            add_button: msg::inventory_add_location(),
            column_name: upper(&lib_locale::msg::column_name()),
            rows: rows
                .iter()
                .filter_map(|row| self.line(&inventory, *row))
                .collect(),
            empty: msg::inventory_empty(self.chord(NEW)),
            note: msg::inventory_note(),
            selected,
            menu: self.menu.map(|row| MenuView {
                row,
                entries: MenuAction::for_row(row)
                    .iter()
                    .map(|&action| MenuEntry {
                        action,
                        label: action.label(),
                        destructive: action.is_destructive(),
                    })
                    .collect(),
            }),
            hints: self.hints(selected),
            open_dialog: self.open_dialog,
        }
    }

    fn line(&self, inventory: &Inventory, row: InventoryRow) -> Option<InventoryLine> {
        match row {
            InventoryRow::Location(id) => {
                let location = inventory.location(id)?;
                Some(InventoryLine {
                    row,
                    name: location.name.clone(),
                    note: Some(location_note(inventory, location)),
                    disclosure: Some(if self.expanded.contains(&id) {
                        msg::inventory_disclosure_open()
                    } else {
                        msg::inventory_disclosure_closed()
                    }),
                })
            }
            InventoryRow::Area(id) => {
                let (_, area) = inventory.area(id)?;
                Some(InventoryLine {
                    row,
                    name: area.name.clone(),
                    note: None,
                    disclosure: None,
                })
            }
        }
    }

    fn locations(&self) -> Vec<Location> {
        self.inventory.borrow().locations()
    }

    fn rows(&self) -> Vec<InventoryRow> {
        visible_rows(&self.locations(), &self.expanded)
    }

    /// The selected row: the stored one while it is still on screen, else the first row, so the
    /// cursor is never lost. `None` only with no Locations.
    fn selected(&self) -> Option<InventoryRow> {
        resolve(self.rows(), self.selected)
    }

    fn select(&mut self, row: Option<InventoryRow>) {
        self.selected = row;
    }

    /// Walks the rows the folds show, Locations and Areas as one list.
    fn step(&mut self, step: Step) {
        let rows = self.rows();
        if !rows.is_empty() {
            self.selected = step_id(&rows, self.selected(), step);
        }
    }

    /// The Location the selected row belongs to.
    fn selected_location(&self) -> Option<LocationId> {
        match self.selected()? {
            InventoryRow::Location(id) => Some(id),
            InventoryRow::Area(id) => self
                .inventory
                .borrow()
                .inventory()
                .area(id)
                .map(|(location, _)| location.id),
        }
    }

    /// Whether `left` has something to do: close an open Location, or climb from an Area to its
    /// Location.
    fn left_is_local(&self) -> bool {
        match self.selected() {
            Some(InventoryRow::Area(_)) => true,
            Some(InventoryRow::Location(id)) => self.expanded.contains(&id),
            None => false,
        }
    }

    /// `right`: open a closed Location, or step from an open one to its first Area.
    fn step_in(&mut self) {
        let Some(InventoryRow::Location(id)) = self.selected() else {
            return;
        };
        if self.expanded.insert(id) {
            return;
        }
        let first_area = self
            .inventory
            .borrow()
            .location(id)
            .and_then(|location| location.areas.first().map(|area| area.id));
        if let Some(area) = first_area {
            self.select(Some(InventoryRow::Area(area)));
        }
    }

    /// `left`: climb from an Area to its Location, or close an open Location.
    fn step_out(&mut self) {
        match self.selected() {
            Some(InventoryRow::Area(_)) => {
                if let Some(location) = self.selected_location() {
                    self.select(Some(InventoryRow::Location(location)));
                }
            }
            Some(InventoryRow::Location(id)) => {
                self.expanded.remove(&id);
            }
            None => {}
        }
    }

    /// The disclosure control of a Location row. Closing keeps the selection on screen by moving a
    /// hidden Area's selection up to its Location.
    fn toggle(&mut self, id: LocationId) {
        if !self.expanded.remove(&id) {
            self.expanded.insert(id);
            return;
        }
        if let Some(InventoryRow::Area(area)) = self.selected {
            let holder = self
                .inventory
                .borrow()
                .inventory()
                .area(area)
                .map(|(location, _)| location.id);
            if holder == Some(id) {
                self.select(Some(InventoryRow::Location(id)));
            }
        }
    }

    /// Moves an Area a place within its Location; one already at that end stays put.
    fn move_area(&mut self, area: AreaId, delta: isize) {
        MoveArea { area, delta }.run(&mut *self.inventory.borrow_mut());
    }

    /// Selects and opens what a landed Dialog left the cursor on.
    fn land(&mut self, outcome: InventoryOutcome) {
        match outcome.change {
            InventoryChange::LocationAdded | InventoryChange::AreaAdded => {
                self.expanded.insert(outcome.location);
            }
            InventoryChange::LocationRemoved => {
                self.expanded.remove(&outcome.location);
            }
            InventoryChange::LocationSaved
            | InventoryChange::AreaSaved
            | InventoryChange::AreaRemoved => {}
        }
        self.select(outcome.selected);
    }

    /// `a`: Add area for the selected Location (or the selected Area's). The Location opens when
    /// the Area lands (`land`), not before, so a cancelled Dialog leaves it as it was. With no
    /// Location it is a Toast.
    fn add_area_for_selection(&mut self, router: &mut dyn InventoryRouter) {
        let Some(location) = self.selected_location() else {
            router.notify(Notice::Toast(
                ToastKind::Info,
                msg::inventory_toast_no_location(),
            ));
            return;
        };
        self.open(InventoryDialog::AddArea(location), router);
    }

    fn menu_action(
        &mut self,
        action: MenuAction,
        row: InventoryRow,
        router: &mut dyn InventoryRouter,
    ) {
        match (action, row) {
            (MenuAction::AddArea, InventoryRow::Location(id)) => {
                self.open(InventoryDialog::AddArea(id), router);
            }
            (MenuAction::EditLocation | MenuAction::EditArea, row) => self.open_edit(row, router),
            (MenuAction::RemoveLocation | MenuAction::RemoveArea, row) => {
                self.open_remove(row, router);
            }
            (MenuAction::MoveUp, InventoryRow::Area(area)) => self.move_area(area, -1),
            (MenuAction::MoveDown, InventoryRow::Area(area)) => self.move_area(area, 1),
            // An entry is only offered on the kind of row it acts on.
            (MenuAction::AddArea, InventoryRow::Area(_))
            | (MenuAction::MoveUp | MenuAction::MoveDown, InventoryRow::Location(_)) => {}
        }
    }

    fn open(&mut self, dialog: InventoryDialog, router: &mut dyn InventoryRouter) {
        if router.open_dialog(dialog) {
            self.open_dialog = Some(dialog);
        }
    }

    /// Add location pre-selects the selected row's Location's Unit; the form falls back to the
    /// first fiat Unit when that is not one it offers.
    fn open_add_location(&mut self, router: &mut dyn InventoryRouter) {
        let unit = self
            .selected_location()
            .and_then(|id| self.inventory.borrow().location(id))
            .map(|location| location.unit);
        self.open(InventoryDialog::AddLocation { unit }, router);
    }

    fn open_edit(&mut self, row: InventoryRow, router: &mut dyn InventoryRouter) {
        let dialog = match row {
            InventoryRow::Location(id) => InventoryDialog::EditLocation(id),
            InventoryRow::Area(id) => InventoryDialog::EditArea(id),
        };
        self.open(dialog, router);
    }

    /// An empty Area gets a plain confirm; one with Items a destination picker; the last Area with
    /// Items a notice, as nowhere is left to move them.
    fn open_remove(&mut self, row: InventoryRow, router: &mut dyn InventoryRouter) {
        let dialog = match row {
            InventoryRow::Location(id) => InventoryDialog::RemoveLocation(id),
            InventoryRow::Area(id) => {
                let inventory = self.inventory.borrow().inventory();
                let Some((location, _)) = inventory.area(id) else {
                    return;
                };
                if inventory.area_items(id) > 0 && location.areas.len() == 1 {
                    InventoryDialog::AreaBlocked(id)
                } else {
                    InventoryDialog::RemoveArea(id)
                }
            }
        };
        self.open(dialog, router);
    }

    fn chord(&self, command: &str) -> &str {
        self.keymap.chord(command).unwrap_or_default()
    }

    /// The legend for the selected row.
    fn hints(&self, selected: Option<InventoryRow>) -> Vec<Hint> {
        let hint = |chord: &str, action: String| Hint {
            chord: chord.to_string(),
            action,
        };
        let pair = |a: &str, b: &str| format!("{}/{}", self.chord(a), self.chord(b));
        let motion = hint(&pair(MOVE_DOWN, MOVE_UP), msg::inventory_hint_move());
        let edit = hint(self.chord(EDIT), msg::inventory_hint_edit());
        let remove = hint(self.chord(REMOVE), msg::inventory_hint_remove());
        let new_location = hint(self.chord(NEW), msg::inventory_hint_new_location());
        let new_area = hint(self.chord(ADD_AREA), msg::inventory_hint_new_area());
        match selected {
            Some(InventoryRow::Location(_)) => vec![
                motion,
                hint(&pair(EXPAND, COLLAPSE), msg::inventory_hint_expand()),
                edit,
                remove,
                new_location,
                new_area,
            ],
            Some(InventoryRow::Area(_)) => vec![
                motion,
                hint(self.chord(COLLAPSE), msg::inventory_hint_location()),
                hint(
                    &pair(MOVE_AREA_DOWN, MOVE_AREA_UP),
                    msg::inventory_hint_reorder(),
                ),
                edit,
                remove,
                new_area,
                new_location,
            ],
            None => vec![new_location],
        }
    }
}

/// The rows `j`/`k` walk, in screen order: each Location, then its Areas when it is open.
fn visible_rows(locations: &[Location], expanded: &HashSet<LocationId>) -> Vec<InventoryRow> {
    let mut rows = Vec::new();
    for location in locations {
        rows.push(InventoryRow::Location(location.id));
        if expanded.contains(&location.id) {
            rows.extend(location.areas.iter().map(|a| InventoryRow::Area(a.id)));
        }
    }
    rows
}

/// The heading's meta: how many Locations, Areas and Items.
fn scope_text(inventory: &Inventory) -> String {
    msg::inventory_scope(
        &msg::inventory_count_locations(count(inventory.location_count())),
        &msg::inventory_count_areas(count(inventory.area_count())),
        &msg::inventory_count_items(count(inventory.item_count())),
    )
}

/// A Location row's note: `10 areas · 214 items`.
fn location_note(inventory: &Inventory, location: &Location) -> String {
    msg::inventory_location_note(
        &msg::inventory_count_areas(count(location.areas.len())),
        &msg::inventory_count_items(count(inventory.location_items(location.id))),
    )
}

#[cfg(test)]
mod tests {
    use lib_config::KeyBindingConfig;

    use super::super::test_support::stub;
    use super::*;
    use crate::fake::{InMemoryInventory, RecordingRouter, RouterCall};
    use crate::keymap::inventory_page_keymap;
    use crate::protocols::InventoryRepository as _;

    struct Page {
        presenter: InventoryPagePresenter,
        router: RecordingRouter,
        inventory: std::rc::Rc<std::cell::RefCell<InMemoryInventory>>,
    }

    impl Page {
        fn new() -> Self {
            Self::with_keys(&KeyBindingConfig::default())
        }

        fn with_keys(config: &KeyBindingConfig) -> Self {
            let (inventory, shared) = stub();
            Self {
                presenter: InventoryPagePresenter::new(shared, inventory_page_keymap(config)),
                router: RecordingRouter::default(),
                inventory,
            }
        }

        fn empty() -> Self {
            let page = Self::new();
            *page.inventory.borrow_mut() = InMemoryInventory::default();
            page
        }

        fn send(&mut self, event: InventoryPageEvent) -> Vec<RouterCall> {
            self.presenter.handle(event, &mut self.router);
            self.router.take()
        }

        fn view(&self) -> InventoryPageViewModel {
            self.presenter.view_model()
        }

        /// The selected row as `location:<name>` or `area:<name>`.
        fn selected_name(&self) -> Option<String> {
            let view = self.view();
            view.rows
                .into_iter()
                .find(|line| Some(line.row) == view.selected)
                .map(|line| match line.row {
                    InventoryRow::Location(_) => format!("location:{}", line.name),
                    InventoryRow::Area(_) => format!("area:{}", line.name),
                })
        }

        fn locations(&self) -> Vec<Location> {
            self.inventory.borrow().locations()
        }

        fn elm(&self) -> Location {
            self.locations()[0].clone()
        }
    }

    #[test]
    fn the_page_lists_the_locations_collapsed_with_the_first_selected_and_the_scope() {
        let view = Page::new().view();

        assert_eq!(view.scope, "3 locations \u{b7} 14 areas \u{b7} 241 items");
        assert_eq!(view.kicker, "LOCATIONS \u{b7} 3");
        assert_eq!(view.column_name, "NAME");
        let names: Vec<&str> = view.rows.iter().map(|line| line.name.as_str()).collect();
        assert_eq!(names, ["12 Elm St contents", "Storage unit", "Hilux"]);
        let notes: Vec<_> = view.rows.iter().map(|line| line.note.as_deref()).collect();
        assert_eq!(
            notes,
            [
                Some("10 areas \u{b7} 214 items"),
                Some("1 area \u{b7} 18 items"),
                Some("3 areas \u{b7} 9 items")
            ]
        );
        assert_eq!(view.rows[0].disclosure.as_deref(), Some("\u{25b8}"));
        assert_eq!(
            view.selected,
            Some(view.rows[0].row),
            "the cursor starts on the first row"
        );
        assert_eq!(view.empty, "No locations yet. Press n to add one.");
    }

    #[test]
    fn zeros_are_shown_and_one_reads_singular() {
        let page = Page::new();
        {
            let mut inventory = page.inventory.borrow_mut();
            inventory.locations.truncate(1);
            inventory.items.clear();
            inventory.locations[0].areas.clear();
        }

        assert_eq!(
            page.view().scope,
            "1 location \u{b7} 0 areas \u{b7} 0 items"
        );
    }

    #[test]
    fn right_opens_a_location_then_steps_in_and_left_climbs_then_closes() {
        let mut page = Page::new();
        let elm = page.elm();

        page.send(InventoryPageEvent::Expand);
        let names: Vec<_> = page.view().rows.iter().map(|l| l.name.clone()).collect();
        assert_eq!(names.len(), 3 + elm.areas.len());
        assert_eq!(
            page.selected_name().as_deref(),
            Some("location:12 Elm St contents")
        );
        assert_eq!(page.view().rows[0].disclosure.as_deref(), Some("\u{25be}"));

        page.send(InventoryPageEvent::Expand);
        assert_eq!(
            page.selected_name(),
            Some(format!("area:{}", elm.areas[0].name))
        );
        assert!(page.presenter.takes(&InventoryPageEvent::Collapse));
        page.send(InventoryPageEvent::Collapse);
        assert_eq!(
            page.selected_name().as_deref(),
            Some("location:12 Elm St contents")
        );
        page.send(InventoryPageEvent::Collapse);
        assert_eq!(page.view().rows.len(), 3, "an open Location closes");
        assert!(
            !page.presenter.takes(&InventoryPageEvent::Collapse),
            "a closed Location leaves left to Settings"
        );
    }

    #[test]
    fn movement_walks_locations_and_areas_as_one_list() {
        let mut page = Page::new();
        page.send(InventoryPageEvent::Expand);
        let rows = page.view().rows;

        for line in rows.iter().skip(1) {
            page.send(InventoryPageEvent::Move(Step::Next));
            assert_eq!(page.view().selected, Some(line.row));
        }
        page.send(InventoryPageEvent::Move(Step::Next));
        assert_eq!(page.view().selected, Some(rows.last().expect("rows").row));
        page.send(InventoryPageEvent::Move(Step::First));
        assert_eq!(page.view().selected, Some(rows[0].row));
        page.send(InventoryPageEvent::Move(Step::Last));
        assert_eq!(page.selected_name().as_deref(), Some("location:Hilux"));
    }

    #[test]
    fn a_click_selects_and_the_disclosure_toggles_keeping_the_cursor_on_screen() {
        let mut page = Page::new();
        let elm = page.elm();
        let area = InventoryRow::Area(elm.areas[0].id);

        page.send(InventoryPageEvent::Toggle(elm.id));
        page.send(InventoryPageEvent::Select(area));
        assert_eq!(page.view().selected, Some(area));
        page.send(InventoryPageEvent::Toggle(elm.id));

        assert_eq!(page.view().rows.len(), 3);
        assert_eq!(
            page.view().selected,
            Some(InventoryRow::Location(elm.id)),
            "a hidden Area's cursor moves up to its Location"
        );
    }

    #[test]
    fn new_edit_and_remove_open_their_dialogs_on_the_selected_row() {
        let mut page = Page::new();
        let elm = page.elm();
        let aud = elm.unit;

        assert_eq!(
            page.send(InventoryPageEvent::New),
            [RouterCall::OpenDialog(InventoryDialog::AddLocation {
                unit: Some(aud)
            })]
        );
        assert_eq!(
            page.view().open_dialog,
            Some(InventoryDialog::AddLocation { unit: Some(aud) })
        );
        page.send(InventoryPageEvent::DialogClosed(None));
        assert_eq!(page.view().open_dialog, None);

        assert_eq!(
            page.send(InventoryPageEvent::Edit),
            [RouterCall::OpenDialog(InventoryDialog::EditLocation(
                elm.id
            ))]
        );
        assert_eq!(
            page.send(InventoryPageEvent::Remove),
            [RouterCall::OpenDialog(InventoryDialog::RemoveLocation(
                elm.id
            ))]
        );
        let area = elm.areas[0].id;
        page.send(InventoryPageEvent::Toggle(elm.id));
        assert_eq!(
            page.send(InventoryPageEvent::EditRow(InventoryRow::Area(area))),
            [RouterCall::OpenDialog(InventoryDialog::EditArea(area))]
        );
        assert_eq!(page.view().selected, Some(InventoryRow::Area(area)));
        assert_eq!(
            page.send(InventoryPageEvent::Remove),
            [RouterCall::OpenDialog(InventoryDialog::RemoveArea(area))]
        );
    }

    #[test]
    fn removing_the_last_area_while_it_holds_items_opens_the_notice() {
        let mut page = Page::new();
        let storage = page.locations()[1].clone();
        let only = storage.areas[0].id;
        page.send(InventoryPageEvent::Select(InventoryRow::Location(
            storage.id,
        )));
        page.send(InventoryPageEvent::Expand);
        page.send(InventoryPageEvent::Expand);

        assert_eq!(
            page.send(InventoryPageEvent::Remove),
            [RouterCall::OpenDialog(InventoryDialog::AreaBlocked(only))]
        );
    }

    #[test]
    fn add_area_leaves_the_location_as_it_was_and_works_from_one_of_its_areas() {
        let mut page = Page::new();
        let elm = page.elm();

        assert_eq!(
            page.send(InventoryPageEvent::AddArea),
            [RouterCall::OpenDialog(InventoryDialog::AddArea(elm.id))]
        );
        assert_eq!(
            page.view().rows.len(),
            3,
            "a cancelled Add area opens nothing"
        );
        page.send(InventoryPageEvent::DialogClosed(None));
        assert_eq!(page.view().rows.len(), 3, "the Location stays closed");
        page.send(InventoryPageEvent::Expand);
        assert_eq!(
            page.send(InventoryPageEvent::AddArea),
            [RouterCall::OpenDialog(InventoryDialog::AddArea(elm.id))],
            "an Area adds to its own Location"
        );
    }

    #[test]
    fn the_empty_page_ignores_edit_and_remove_toasts_on_add_area_and_opens_add_location() {
        let mut page = Page::empty();

        assert_eq!(page.view().selected, None);
        assert_eq!(page.send(InventoryPageEvent::Edit), []);
        assert_eq!(page.send(InventoryPageEvent::Remove), []);
        page.send(InventoryPageEvent::Move(Step::Next));
        page.send(InventoryPageEvent::MoveArea(1));
        page.send(InventoryPageEvent::Expand);
        assert_eq!(page.view().rows, []);
        assert_eq!(
            page.send(InventoryPageEvent::AddArea),
            [RouterCall::Notice(Notice::Toast(
                ToastKind::Info,
                "Add a location first".into()
            ))]
        );
        assert_eq!(
            page.send(InventoryPageEvent::New),
            [RouterCall::OpenDialog(InventoryDialog::AddLocation {
                unit: None
            })]
        );
    }

    #[test]
    fn reordering_moves_an_area_a_place_and_is_inert_on_a_location_and_at_the_ends() {
        let mut page = Page::new();
        let elm = page.elm();
        let names =
            |page: &Page| -> Vec<String> { page.elm().areas.into_iter().map(|a| a.name).collect() };
        let before = names(&page);

        page.send(InventoryPageEvent::MoveArea(1));
        assert_eq!(names(&page), before, "a Location row has nothing to move");

        page.send(InventoryPageEvent::Expand);
        page.send(InventoryPageEvent::Expand);
        page.send(InventoryPageEvent::MoveArea(1));
        let after = names(&page);
        assert_eq!((&after[0], &after[1]), (&before[1], &before[0]));
        assert_eq!(
            page.view().selected,
            Some(InventoryRow::Area(elm.areas[0].id)),
            "the cursor follows the Area"
        );
        page.send(InventoryPageEvent::MoveArea(-1));
        assert_eq!(names(&page), before);
        page.send(InventoryPageEvent::MoveArea(-1));
        assert_eq!(names(&page), before, "the top edge is a no-op");
    }

    #[test]
    fn a_right_click_opens_the_rows_menu_and_its_entries_do_what_the_keys_do() {
        let mut page = Page::new();
        let elm = page.elm();
        let area = InventoryRow::Area(elm.areas[0].id);

        page.send(InventoryPageEvent::OpenMenu(InventoryRow::Location(elm.id)));
        let menu = page.view().menu.expect("a menu is open");
        let entries: Vec<_> = menu
            .entries
            .iter()
            .map(|e| (e.label.as_str(), e.destructive))
            .collect();
        assert_eq!(
            entries,
            [
                ("Add area\u{2026}", false),
                ("Edit location\u{2026}", false),
                ("Remove location\u{2026}", true)
            ]
        );
        assert_eq!(
            page.send(InventoryPageEvent::Menu(MenuAction::AddArea)),
            [RouterCall::OpenDialog(InventoryDialog::AddArea(elm.id))]
        );
        assert_eq!(page.view().menu, None, "an entry closes the menu");
        page.send(InventoryPageEvent::DialogClosed(None));
        page.send(InventoryPageEvent::Expand);

        page.send(InventoryPageEvent::OpenMenu(area));
        assert_eq!(page.view().selected, Some(area), "the right-click selects");
        page.send(InventoryPageEvent::Menu(MenuAction::MoveDown));
        assert_eq!(page.elm().areas[1].id, elm.areas[0].id);
        page.send(InventoryPageEvent::OpenMenu(area));
        page.send(InventoryPageEvent::Move(Step::Next));
        assert_eq!(page.view().menu, None, "a key puts the menu away");
        assert!(!page.presenter.menu_open());
    }

    #[test]
    fn a_landed_dialog_selects_its_row_and_opens_or_closes_the_location() {
        let mut page = Page::new();
        let elm = page.elm();
        let area = InventoryRow::Area(elm.areas[1].id);

        page.send(InventoryPageEvent::DialogClosed(Some(InventoryOutcome {
            change: InventoryChange::AreaAdded,
            location: elm.id,
            selected: Some(area),
        })));
        assert_eq!(page.view().selected, Some(area));
        assert!(page.view().rows.len() > 3, "the Location opens to show it");

        page.send(InventoryPageEvent::DialogClosed(Some(InventoryOutcome {
            change: InventoryChange::LocationRemoved,
            location: elm.id,
            selected: Some(InventoryRow::Location(page.locations()[1].id)),
        })));
        assert_eq!(page.view().rows.len(), 3, "a removed Location's fold goes");
    }

    #[test]
    fn the_legend_follows_the_selected_row_and_shows_the_chords_the_keymap_binds() {
        let mut page = Page::new();
        let chords = |page: &Page| -> Vec<String> {
            page.view().hints.into_iter().map(|h| h.chord).collect()
        };

        assert_eq!(chords(&page), ["j/k", "right/left", "e", "x", "n", "a"]);
        page.send(InventoryPageEvent::Expand);
        page.send(InventoryPageEvent::Expand);
        assert_eq!(chords(&page), ["j/k", "left", "J/K", "e", "x", "a", "n"]);

        let mut config = KeyBindingConfig::default();
        config
            .bindings
            .insert("inventory.add_area".into(), "s".into());
        let rebound = Page::with_keys(&config);
        assert_eq!(chords(&rebound), ["j/k", "right/left", "e", "x", "n", "s"]);
        assert_eq!(Page::empty().view().hints.len(), 1);
    }
}
