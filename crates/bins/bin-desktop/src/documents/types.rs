//! The Ledger's user-managed Document Types (ADR-0031), as the Settings › Documents page shows
//! them: in the user's order, each with its Tracks date, Remind lead time and Financial year flag.
//! `gpui`-free and side-effect free, so every rule is unit-tested without a window.
//!
//! The Documents surface reads this list through `documents::DocumentType`, which is just a row's
//! stable `id`, so a rename or a reorder never touches a Document.

pub use lib_documents::types::*;

use crate::{
    chrome::dialog_host::{Dialog, DialogKey, DialogOutcome},
    form::field::TextField,
    form::select::SelectState,
};

/// Longest Document Type name, in characters (#477).
pub const NAME_MAX: usize = 40;

/// The Tracks date control's options, in order; `None` tracks no date.
pub const TRACKS_OPTIONS: [Option<TracksDate>; 5] = [
    None,
    Some(TracksDate::Renews),
    Some(TracksDate::Ends),
    Some(TracksDate::Expires),
    Some(TracksDate::Revalue),
];

/// The Remind control's options, in order; `None` raises no reminder.
pub const REMIND_OPTIONS: [Option<RemindLead>; 7] = [
    None,
    Some(RemindLead::Days(7)),
    Some(RemindLead::Days(14)),
    Some(RemindLead::Days(30)),
    Some(RemindLead::Days(60)),
    Some(RemindLead::Months(3)),
    Some(RemindLead::Months(6)),
];

/// A field of the Add/Edit dialog, in `Tab` order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormField {
    Name,
    TracksDate,
    Remind,
    FinancialYear,
}

impl FormField {
    const ORDER: [FormField; 4] = [
        FormField::Name,
        FormField::TracksDate,
        FormField::Remind,
        FormField::FinancialYear,
    ];
}

/// Why a name can't be saved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NameError {
    Empty,
    TooLong,
    /// Another type already has the name, ignoring case.
    Taken,
}

/// The Add/Edit dialog's draft. The other types' names are copied in at open, so the form
/// validates without `Shell`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentTypeForm {
    pub name: TextField,
    pub tracks_date: Option<TracksDate>,
    pub remind: Option<RemindLead>,
    pub financial_year: bool,
    pub focused: FormField,
    /// Whether the name has been edited, which is when its error starts to show.
    pub touched: bool,
    /// Every other type's name, trimmed and lower-cased: the ones this name must not repeat.
    taken: Vec<String>,
}

impl DocumentTypeForm {
    /// A blank form for a new type among `types`.
    pub fn new(types: &[DocumentTypeRow]) -> Self {
        Self {
            name: TextField::default(),
            tracks_date: None,
            remind: None,
            financial_year: false,
            focused: FormField::Name,
            touched: false,
            taken: taken_names(types, None),
        }
    }

    /// A form editing `row`, whose own name is not taken.
    pub fn from_row(row: &DocumentTypeRow, types: &[DocumentTypeRow]) -> Self {
        Self {
            name: TextField::new(row.name.clone()),
            tracks_date: row.tracks_date,
            remind: row.remind,
            financial_year: row.financial_year,
            taken: taken_names(types, Some(row.id)),
            ..Self::new(types)
        }
    }

    /// The name's problem, if any.
    pub fn name_error(&self) -> Option<NameError> {
        let name = self.name.text().trim();
        if name.is_empty() {
            return Some(NameError::Empty);
        }
        if name.chars().count() > NAME_MAX {
            return Some(NameError::TooLong);
        }
        let wanted = name.to_lowercase();
        self.taken.contains(&wanted).then_some(NameError::Taken)
    }

    pub fn is_valid(&self) -> bool {
        self.name_error().is_none()
    }

    /// Remind is inert, and so disabled, while no date is tracked.
    pub fn remind_enabled(&self) -> bool {
        self.tracks_date.is_some()
    }

    pub fn set_tracks_date(&mut self, tracks_date: Option<TracksDate>) {
        self.tracks_date = tracks_date;
        if tracks_date.is_none() {
            self.remind = None;
        }
    }

    pub fn set_remind(&mut self, remind: Option<RemindLead>) {
        if self.remind_enabled() {
            self.remind = remind;
        }
    }

    pub fn toggle_financial_year(&mut self) {
        self.financial_year = !self.financial_year;
    }

    /// Moves focus to `field`.
    pub fn focus(&mut self, field: FormField) {
        self.focused = field;
    }

    /// `Tab` / `Shift-Tab`, skipping Remind while it is disabled.
    pub fn cycle_focus(&mut self, backward: bool) {
        let count = FormField::ORDER.len();
        let mut index = FormField::ORDER
            .iter()
            .position(|field| *field == self.focused)
            .unwrap_or(0);
        loop {
            index = if backward {
                (index + count - 1) % count
            } else {
                (index + 1) % count
            };
            if FormField::ORDER[index] != FormField::Remind || self.remind_enabled() {
                break;
            }
        }
        self.focused = FormField::ORDER[index];
    }

    /// `Left`/`Right` (`delta` -1/1) on the focused control; stops at either end.
    pub fn step_focused(&mut self, delta: isize) {
        match self.focused {
            FormField::TracksDate => {
                self.set_tracks_date(step(&TRACKS_OPTIONS, self.tracks_date, delta));
            }
            FormField::Remind => self.set_remind(step(&REMIND_OPTIONS, self.remind, delta)),
            FormField::FinancialYear => self.toggle_financial_year(),
            FormField::Name => {}
        }
    }
}

fn taken_names(types: &[DocumentTypeRow], own_id: Option<u32>) -> Vec<String> {
    types
        .iter()
        .filter(|row| Some(row.id) != own_id)
        .map(|row| row.name.trim().to_lowercase())
        .collect()
}

impl DocumentTypeForm {
    /// Keys this form takes before the shared typing: `Left`/`Right` step the focused control,
    /// `Space` toggles the Financial year flag, `Shift-Tab` goes back a field. Typing and
    /// `Backspace` fall through to the Name field, marking it touched so its error shows.
    fn handle_own_key(&mut self, key: DialogKey) -> Option<DialogOutcome> {
        match key {
            DialogKey::Left => self.step_focused(-1),
            DialogKey::Right => self.step_focused(1),
            DialogKey::BackTab => self.cycle_focus(true),
            DialogKey::Char(' ') if self.focused == FormField::FinancialYear => {
                self.toggle_financial_year();
            }
            DialogKey::Char(_) | DialogKey::Backspace if self.focused == FormField::Name => {
                self.touched = true;
                return None;
            }
            // A control swallows typing rather than letting it fall through to the shell.
            DialogKey::Char(_) | DialogKey::Backspace | DialogKey::Up | DialogKey::Down => {}
            DialogKey::Tab
            | DialogKey::Enter
            | DialogKey::CtrlEnter
            | DialogKey::Ctrl(_)
            | DialogKey::Other => return None,
        }
        Some(DialogOutcome::Handled)
    }
}

fn step<T: Copy + PartialEq>(options: &[T], current: T, delta: isize) -> T {
    let index = options
        .iter()
        .position(|option| *option == current)
        .unwrap_or(0);
    let next = index
        .saturating_add_signed(delta)
        .min(options.len().saturating_sub(1));
    options.get(next).copied().unwrap_or(current)
}

/// The Remove dialog's destination select. The destination names and whether one is needed are
/// copied in at open, so the dialog validates without `Shell`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoveForm {
    /// Where the removed type's Filed files can move: every other type, in order.
    pub destinations: Vec<String>,
    pub select: SelectState,
    needs_destination: bool,
}

impl RemoveForm {
    /// The form for removing the type with `id`, whose files move to Other unless told otherwise.
    pub fn new(types: &[DocumentTypeRow], id: u32) -> Self {
        Self {
            destinations: destination_names(types, id),
            select: destination_select(types, id),
            needs_destination: get(types, id).is_some_and(|row| row.files > 0),
        }
    }

    pub fn is_valid(&self) -> bool {
        !self.needs_destination || self.select.value().is_some()
    }

    /// `Up`/`Down` walk the open list or step the value; `Space` opens it or commits the
    /// highlight; `Enter` commits an open list, else falls through to confirm.
    fn handle_own_key(&mut self, key: DialogKey) -> Option<DialogOutcome> {
        match key {
            DialogKey::Up | DialogKey::Down => {
                let delta = if key == DialogKey::Up { -1 } else { 1 };
                if self.select.is_open() {
                    self.select.move_highlight(&self.destinations, delta);
                } else {
                    self.select.step(&self.destinations, delta);
                }
            }
            DialogKey::Char(' ') if self.select.is_open() => {
                self.select.commit(&self.destinations);
            }
            DialogKey::Char(' ') => self.select.open(&self.destinations),
            DialogKey::Enter if self.select.is_open() => self.select.commit(&self.destinations),
            _ => return None,
        }
        Some(DialogOutcome::Handled)
    }

    /// A click on the select toggles its list.
    pub fn click_select(&mut self) {
        if self.select.is_open() {
            self.select.cancel();
        } else {
            self.select.open(&self.destinations);
        }
    }

    pub fn choose(&mut self, index: usize) {
        self.select.choose(&self.destinations, index);
    }
}

/// The dialog open over the Documents page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocumentTypesDialog {
    Add(DocumentTypeForm),
    Edit(u32, DocumentTypeForm),
    /// Removing the type with this id; the select is where its Filed files move to.
    Remove(u32, RemoveForm),
    /// Other has no remove action, so this only says why.
    DefaultNotice,
}

impl DocumentTypesDialog {
    pub fn form_mut(&mut self) -> Option<&mut DocumentTypeForm> {
        match self {
            Self::Add(form) | Self::Edit(_, form) => Some(form),
            _ => None,
        }
    }
}

impl Dialog for DocumentTypesDialog {
    fn handle_own_key(&mut self, key: DialogKey) -> Option<DialogOutcome> {
        match self {
            Self::Add(form) | Self::Edit(_, form) => form.handle_own_key(key),
            Self::Remove(_, form) => form.handle_own_key(key),
            Self::DefaultNotice => None,
        }
    }

    fn focused_text(&mut self) -> Option<&mut TextField> {
        match self {
            Self::Add(form) | Self::Edit(_, form) if form.focused == FormField::Name => {
                Some(&mut form.name)
            }
            _ => None,
        }
    }

    fn cycle_field(&mut self) {
        if let Some(form) = self.form_mut() {
            form.cycle_focus(false);
        }
    }

    fn is_valid(&self) -> bool {
        match self {
            Self::Add(form) | Self::Edit(_, form) => form.is_valid(),
            Self::Remove(_, form) => form.is_valid(),
            Self::DefaultNotice => true,
        }
    }

    fn close_open_select(&mut self) -> bool {
        match self {
            Self::Remove(_, form) => {
                let was_open = form.select.is_open();
                form.select.cancel();
                was_open
            }
            _ => false,
        }
    }
}

/// Names a removed type's files can move to: every other type, in order.
pub fn destination_names(types: &[DocumentTypeRow], removing: u32) -> Vec<String> {
    types
        .iter()
        .filter(|row| row.id != removing)
        .map(|row| row.name.clone())
        .collect()
}

/// The Remove dialog's destination select, defaulting to Other.
pub fn destination_select(types: &[DocumentTypeRow], removing: u32) -> SelectState {
    SelectState::new(
        types
            .iter()
            .find(|row| row.is_default && row.id != removing)
            .map(|row| row.name.clone()),
    )
}

/// Adds a type from `form` at the end of the order and returns its new id. `next_id` only ever
/// counts up, so removing the newest type never frees its id for a saved `type:<id>` filter to
/// match against a different type.
pub fn add_type(
    types: &mut Vec<DocumentTypeRow>,
    next_id: &mut u32,
    form: &DocumentTypeForm,
) -> u32 {
    let id = *next_id;
    *next_id += 1;
    types.push(DocumentTypeRow {
        id,
        name: form.name.text().trim().to_string(),
        tracks_date: form.tracks_date,
        remind: form.remind,
        financial_year: form.financial_year,
        files: 0,
        is_default: false,
    });
    id
}

/// Applies `form` to the type with `id`. The id never changes, so saved filters survive a rename.
pub fn edit_type(types: &mut [DocumentTypeRow], id: u32, form: &DocumentTypeForm) {
    if let Some(row) = types.iter_mut().find(|row| row.id == id) {
        row.name = form.name.text().trim().to_string();
        row.tracks_date = form.tracks_date;
        row.remind = form.remind;
        row.financial_year = form.financial_year;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn form(name: &str) -> DocumentTypeForm {
        form_editing(name, None)
    }

    /// A draft named `name` over the seed, editing the type `own_id` if given.
    fn form_editing(name: &str, own_id: Option<u32>) -> DocumentTypeForm {
        let types = default_types();
        let mut form = match own_id.and_then(|id| get(&types, id)) {
            Some(row) => DocumentTypeForm::from_row(row, &types),
            None => DocumentTypeForm::new(&types),
        };
        form.name = TextField::new(name);
        form
    }

    #[test]
    fn names_are_required_trimmed_short_and_unique_ignoring_case() {
        assert_eq!(form("  ").name_error(), Some(NameError::Empty));
        assert_eq!(form(&"a".repeat(41)).name_error(), Some(NameError::TooLong));
        assert_eq!(form(&"a".repeat(40)).name_error(), None);
        assert_eq!(form(" tAx ").name_error(), Some(NameError::Taken));
        // Editing a type keeps its own name free, even in another case.
        assert_eq!(form_editing("TAX", Some(3)).name_error(), None);
        assert_eq!(
            form_editing("tax", Some(4)).name_error(),
            Some(NameError::Taken)
        );
    }

    #[test]
    fn clearing_tracks_date_clears_and_disables_remind() {
        let mut draft = form("Leases");
        draft.set_remind(Some(RemindLead::Days(7)));
        assert_eq!(draft.remind, None);
        draft.set_tracks_date(Some(TracksDate::Ends));
        draft.set_remind(Some(RemindLead::Days(7)));
        assert_eq!(draft.remind, Some(RemindLead::Days(7)));
        draft.set_tracks_date(None);
        assert_eq!(draft.remind, None);
    }

    #[test]
    fn tab_skips_remind_while_it_is_disabled() {
        let mut draft = form("x");
        draft.cycle_focus(false);
        assert_eq!(draft.focused, FormField::TracksDate);
        draft.cycle_focus(false);
        assert_eq!(draft.focused, FormField::FinancialYear);
        draft.set_tracks_date(Some(TracksDate::Renews));
        draft.cycle_focus(true);
        assert_eq!(draft.focused, FormField::Remind);
    }

    #[test]
    fn stepping_a_control_stops_at_either_end() {
        let mut draft = form("x");
        draft.focus(FormField::TracksDate);
        draft.step_focused(-1);
        assert_eq!(draft.tracks_date, None);
        for _ in 0..9 {
            draft.step_focused(1);
        }
        assert_eq!(draft.tracks_date, Some(TracksDate::Revalue));
    }

    #[test]
    fn add_appends_with_a_fresh_id_and_no_files() {
        let mut types = default_types();
        let mut next_id = first_free_id(&types);
        let id = add_type(&mut types, &mut next_id, &form("  Leases "));
        assert_eq!(id, 10);
        assert_eq!(next_id, 11);
        let row = types.last().unwrap();
        assert_eq!(
            (row.name.as_str(), row.files, row.is_default),
            ("Leases", 0, false)
        );
    }

    #[test]
    fn a_removed_types_id_is_never_given_to_a_new_type() {
        let mut types = default_types();
        let mut next_id = first_free_id(&types);
        let newest = add_type(&mut types, &mut next_id, &form("Leases"));
        assert_eq!(remove_type(&mut types, newest, None), Ok(()));
        assert_ne!(add_type(&mut types, &mut next_id, &form("Loans")), newest);
    }

    #[test]
    fn edit_renames_but_keeps_the_id_files_and_default_flag() {
        let mut types = default_types();
        let mut draft = DocumentTypeForm::from_row(&types[8], &types);
        draft.name = TextField::new("Misc");
        edit_type(&mut types, 9, &draft);
        assert_eq!(types[8].id, 9);
        assert_eq!(types[8].name, "Misc");
        assert_eq!(types[8].files, 2);
        assert!(types[8].is_default);
    }

    #[test]
    fn the_destination_defaults_to_other_and_excludes_the_removed_type() {
        let types = default_types();
        assert_eq!(destination_select(&types, 3).value(), Some("Other"));
        assert!(!destination_names(&types, 3).contains(&"Tax".to_string()));
        assert_eq!(destination_names(&types, 3).len(), 8);
    }
}
