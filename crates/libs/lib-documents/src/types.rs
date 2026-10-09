//! The Ledger's user-managed Document Types (ADR-0031), as the Settings › Documents page shows
//! them: in the user's order, each with its Tracks date, Remind lead time and Financial year flag.
//! `gpui`-free and side-effect free, so every rule is unit-tested without a window.
//!
//! The Documents surface reads this list through `documents::DocumentType`, which is just a row's
//! stable `id`, so a rename or a reorder never touches a Document.

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

/// The row with `id`.
pub fn get(types: &[DocumentTypeRow], id: u32) -> Option<&DocumentTypeRow> {
    types.iter().find(|row| row.id == id)
}

/// The id a Document falls back to when its type is missing or none was read: the Default type
/// (Other), else the first in order.
pub fn fallback_id(types: &[DocumentTypeRow]) -> u32 {
    types
        .iter()
        .find(|row| row.is_default)
        .or_else(|| types.first())
        .map_or(0, |row| row.id)
}

/// The id of the type named `name`, ignoring case: how an Inbox file's extracted type is matched.
pub fn id_by_name(types: &[DocumentTypeRow], name: &str) -> Option<u32> {
    types
        .iter()
        .find(|row| row.name.to_lowercase() == name.to_lowercase())
        .map(|row| row.id)
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

/// The id the next added type takes: past every seeded id.
pub fn first_free_id(types: &[DocumentTypeRow]) -> u32 {
    types
        .iter()
        .map(|row| row.id)
        .max()
        .map_or(1, |max| max + 1)
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
