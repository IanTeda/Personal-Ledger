//! The Ledger's user-managed Document Types (ADR-0031), as the Settings › Documents page shows
//! them: in the user's order, each with its Tracks date, Remind lead time and Financial year flag.
//! `gpui`-free and side-effect free, so every rule is unit-tested without a window.
//!
//! The Documents surface still reads the fixed `documents::DocumentType` enum; rewiring it to this
//! list is a later ticket (#482), which is why the types here carry their own stable `id`.

use crate::select::SelectState;

/// The kind of Key Date a type tracks (`CONTEXT.md`'s Key Date).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TracksDate {
    Renews,
    Ends,
    Expires,
    Revalue,
}

/// How far ahead of the Key Date a type's reminder is raised.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemindLead {
    Days(u32),
    Months(u32),
}

/// A Document Type. `id` is stable across renames, so saved filters survive one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentTypeRow {
    pub id: u32,
    pub name: String,
    pub tracks_date: Option<TracksDate>,
    /// Inert while `tracks_date` is `None`.
    pub remind: Option<RemindLead>,
    pub financial_year: bool,
    /// Filed Documents of this type.
    pub files: u32,
    /// The Default Document Type (Other): never removed, though it can be renamed and reordered.
    pub is_default: bool,
}

/// Unfiled Documents in the Inbox, which have no type yet: the "of" in "392 of 412 files typed".
pub const SEED_UNFILED: u32 = 20;

/// A new Ledger's nine types (ADR-0031), in order, ending with the Default, Other.
pub fn default_types() -> Vec<DocumentTypeRow> {
    let row = |id: u32,
               name: &str,
               tracks_date: Option<TracksDate>,
               remind: Option<RemindLead>,
               financial_year: bool,
               files: u32| DocumentTypeRow {
        id,
        name: name.to_string(),
        tracks_date,
        remind,
        financial_year,
        files,
        is_default: false,
    };
    let mut types = vec![
        row(1, "Receipts", None, None, true, 186),
        row(2, "Statements", None, None, true, 94),
        row(3, "Tax", None, None, true, 31),
        row(
            4,
            "Insurance",
            Some(TracksDate::Renews),
            Some(RemindLead::Days(30)),
            false,
            18,
        ),
        row(
            5,
            "Warranties & manuals",
            Some(TracksDate::Ends),
            Some(RemindLead::Days(30)),
            false,
            42,
        ),
        row(
            6,
            "Contracts",
            Some(TracksDate::Ends),
            Some(RemindLead::Days(60)),
            false,
            9,
        ),
        row(
            7,
            "Identity",
            Some(TracksDate::Expires),
            Some(RemindLead::Months(6)),
            false,
            6,
        ),
        row(8, "Bills", None, None, false, 4),
        row(9, "Other", None, None, false, 2),
    ];
    if let Some(other) = types.last_mut() {
        other.is_default = true;
    }
    types
}

/// Filed Documents across every type.
pub fn typed_files(types: &[DocumentTypeRow]) -> u32 {
    types.iter().map(|row| row.files).sum()
}

/// Every Document, Filed or Unfiled.
pub fn all_files(types: &[DocumentTypeRow]) -> u32 {
    typed_files(types) + SEED_UNFILED
}

/// The position of the type with `id`.
pub fn position(types: &[DocumentTypeRow], id: u32) -> Option<usize> {
    types.iter().position(|row| row.id == id)
}

/// Swaps the type with `id` one place down (`delta` 1) or up (`delta` -1), stopping at either end.
/// Returns whether it moved.
pub fn move_by(types: &mut [DocumentTypeRow], id: u32, delta: isize) -> bool {
    let Some(from) = position(types, id) else {
        return false;
    };
    let Some(to) = from
        .checked_add_signed(delta)
        .filter(|to| *to < types.len())
    else {
        return false;
    };
    types.swap(from, to);
    true
}

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

/// The Add/Edit dialog's draft.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentTypeForm {
    pub name: String,
    pub tracks_date: Option<TracksDate>,
    pub remind: Option<RemindLead>,
    pub financial_year: bool,
    pub focused: FormField,
    /// Whether the name has been edited, which is when its error starts to show.
    pub touched: bool,
}

impl Default for DocumentTypeForm {
    fn default() -> Self {
        Self {
            name: String::new(),
            tracks_date: None,
            remind: None,
            financial_year: false,
            focused: FormField::Name,
            touched: false,
        }
    }
}

impl DocumentTypeForm {
    pub fn from_row(row: &DocumentTypeRow) -> Self {
        Self {
            name: row.name.clone(),
            tracks_date: row.tracks_date,
            remind: row.remind,
            financial_year: row.financial_year,
            ..Self::default()
        }
    }

    /// The name's problem, if any. `own_id` is the type being edited, whose own name is not taken.
    pub fn name_error(&self, types: &[DocumentTypeRow], own_id: Option<u32>) -> Option<NameError> {
        let name = self.name.trim();
        if name.is_empty() {
            return Some(NameError::Empty);
        }
        if name.chars().count() > NAME_MAX {
            return Some(NameError::TooLong);
        }
        let wanted = name.to_lowercase();
        types
            .iter()
            .any(|row| Some(row.id) != own_id && row.name.trim().to_lowercase() == wanted)
            .then_some(NameError::Taken)
    }

    pub fn is_valid(&self, types: &[DocumentTypeRow], own_id: Option<u32>) -> bool {
        self.name_error(types, own_id).is_none()
    }

    /// Remind is inert, and so disabled, while no date is tracked.
    pub fn remind_enabled(&self) -> bool {
        self.tracks_date.is_some()
    }

    pub fn push_char(&mut self, ch: char) {
        if self.focused == FormField::Name && !ch.is_control() {
            self.name.push(ch);
            self.touched = true;
        }
    }

    pub fn backspace(&mut self) {
        if self.focused == FormField::Name {
            self.name.pop();
            self.touched = true;
        }
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

/// The dialog open over the Documents page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocumentTypesDialog {
    Add(DocumentTypeForm),
    Edit(u32, DocumentTypeForm),
    /// Removing the type with this id; the select is where its Filed files move to.
    Remove(u32, SelectState),
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

/// Adds a type from `form` at the end of the order and returns its new id.
pub fn add_type(types: &mut Vec<DocumentTypeRow>, form: &DocumentTypeForm) -> u32 {
    let id = types
        .iter()
        .map(|row| row.id)
        .max()
        .map_or(1, |max| max + 1);
    types.push(DocumentTypeRow {
        id,
        name: form.name.trim().to_string(),
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
        row.name = form.name.trim().to_string();
        row.tracks_date = form.tracks_date;
        row.remind = form.remind;
        row.financial_year = form.financial_year;
    }
}

/// Why a type can't be removed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoveError {
    Unknown,
    /// Other, the Default Document Type.
    IsDefault,
    /// The type has Filed files and no destination type was given.
    NeedsDestination,
}

/// Removes the type with `id`, moving its Filed files to the type named `destination`.
pub fn remove_type(
    types: &mut Vec<DocumentTypeRow>,
    id: u32,
    destination: Option<&str>,
) -> Result<(), RemoveError> {
    let position = position(types, id).ok_or(RemoveError::Unknown)?;
    if types[position].is_default {
        return Err(RemoveError::IsDefault);
    }
    let files = types[position].files;
    if files > 0 {
        let target = types
            .iter()
            .position(|row| row.id != id && Some(row.name.as_str()) == destination)
            .ok_or(RemoveError::NeedsDestination)?;
        types[target].files += files;
    }
    types.remove(position);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn form(name: &str) -> DocumentTypeForm {
        DocumentTypeForm {
            name: name.to_string(),
            ..DocumentTypeForm::default()
        }
    }

    #[test]
    fn names_are_required_trimmed_short_and_unique_ignoring_case() {
        let types = default_types();
        assert_eq!(form("  ").name_error(&types, None), Some(NameError::Empty));
        assert_eq!(
            form(&"a".repeat(41)).name_error(&types, None),
            Some(NameError::TooLong)
        );
        assert_eq!(form(&"a".repeat(40)).name_error(&types, None), None);
        assert_eq!(
            form(" tAx ").name_error(&types, None),
            Some(NameError::Taken)
        );
        // Editing a type keeps its own name free, even in another case.
        assert_eq!(form("TAX").name_error(&types, Some(3)), None);
        assert_eq!(
            form("tax").name_error(&types, Some(4)),
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
        let id = add_type(&mut types, &form("  Leases "));
        assert_eq!(id, 10);
        let row = types.last().unwrap();
        assert_eq!(
            (row.name.as_str(), row.files, row.is_default),
            ("Leases", 0, false)
        );
    }

    #[test]
    fn edit_renames_but_keeps_the_id_files_and_default_flag() {
        let mut types = default_types();
        let mut draft = DocumentTypeForm::from_row(&types[8]);
        draft.name = "Misc".to_string();
        edit_type(&mut types, 9, &draft);
        assert_eq!(types[8].id, 9);
        assert_eq!(types[8].name, "Misc");
        assert_eq!(types[8].files, 2);
        assert!(types[8].is_default);
    }

    #[test]
    fn remove_moves_files_to_the_destination() {
        let mut types = default_types();
        assert_eq!(remove_type(&mut types, 3, Some("Other")), Ok(()));
        assert_eq!(types.len(), 8);
        assert_eq!(types.last().unwrap().files, 33);
        assert_eq!(typed_files(&types), 392);
    }

    #[test]
    fn remove_needs_a_real_destination_when_files_exist() {
        let mut types = default_types();
        assert_eq!(
            remove_type(&mut types, 3, None),
            Err(RemoveError::NeedsDestination)
        );
        assert_eq!(
            remove_type(&mut types, 3, Some("Tax")),
            Err(RemoveError::NeedsDestination)
        );
        assert_eq!(types.len(), 9);
    }

    #[test]
    fn an_empty_type_removes_with_no_destination_and_other_never_does() {
        let mut types = default_types();
        types[2].files = 0;
        assert_eq!(remove_type(&mut types, 3, None), Ok(()));
        assert_eq!(
            remove_type(&mut types, 9, Some("Bills")),
            Err(RemoveError::IsDefault)
        );
        assert_eq!(remove_type(&mut types, 99, None), Err(RemoveError::Unknown));
    }

    #[test]
    fn the_destination_defaults_to_other_and_excludes_the_removed_type() {
        let types = default_types();
        assert_eq!(destination_select(&types, 3).value(), Some("Other"));
        assert!(!destination_names(&types, 3).contains(&"Tax".to_string()));
        assert_eq!(destination_names(&types, 3).len(), 8);
    }

    #[test]
    fn the_seed_is_nine_types_ending_with_the_default() {
        let types = default_types();
        assert_eq!(types.len(), 9);
        assert_eq!(types.iter().filter(|row| row.is_default).count(), 1);
        let other = types.last().unwrap();
        assert!(other.is_default);
        assert_eq!(other.name, "Other");
    }

    #[test]
    fn the_seed_reads_392_of_412_files_typed() {
        let types = default_types();
        assert_eq!(typed_files(&types), 392);
        assert_eq!(all_files(&types), 412);
    }

    #[test]
    fn move_by_swaps_neighbours_and_stops_at_the_ends() {
        let mut types = default_types();
        assert!(!move_by(&mut types, 1, -1));
        assert!(move_by(&mut types, 1, 1));
        assert_eq!(types[0].id, 2);
        assert_eq!(types[1].id, 1);
        assert!(!move_by(&mut types, 9, 1));
        assert!(move_by(&mut types, 9, -1));
        assert_eq!(types[7].id, 9);
        assert!(!move_by(&mut types, 99, 1));
    }
}
