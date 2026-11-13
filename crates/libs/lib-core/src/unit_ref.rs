//! A Unit as the modules that don't own Units read it.

use crate::UnitId;

/// A Unit as another module reads it: a shared read value, one type for every module's Unit read
/// port rather than one per crate. It carries the union of its readers' fields, so a reader may
/// ignore some of them, and a field added here recompiles every reader.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnitRef {
    pub id: UnitId,
    /// Lower case, as Units hold it.
    pub code: String,
    pub name: String,
    /// A deactivated Unit is not offered for a new pick, though a row already naming it keeps
    /// showing its code.
    pub is_active: bool,
    /// The Unit's Precision (GLOSSARY): the places an amount in it shows at. The number, not
    /// `lib_units::Precision`, since `lib-core` can't depend on the Units crate.
    pub precision: u8,
}
