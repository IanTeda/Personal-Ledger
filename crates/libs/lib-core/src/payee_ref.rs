//! A Payee as the modules that don't own Payees read it.

use crate::PayeeId;

/// A Payee as another module reads it: a shared read value, one type for every module's Payee read
/// port rather than one per crate. It carries the union of its readers' fields, so a reader may
/// ignore some of them (Transactions ignores `aliases` and `is_active`, Bills `aliases`), and a
/// field added here recompiles every reader.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PayeeRef {
    pub id: PayeeId,
    pub name: String,
    /// The match text: each alias, upper case, as the Payees store it.
    pub aliases: Vec<String>,
    pub is_active: bool,
}
