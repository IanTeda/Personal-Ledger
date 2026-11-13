//! An Account as the modules that don't own Accounts read it.

use crate::{AccountId, UnitId};

/// An Account as another module reads it: a shared read value, one type for every module's
/// Account read port rather than one per crate. It carries the union of its readers' fields, so a
/// reader may ignore some of them, and a field added here recompiles every reader.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountRef {
    pub id: AccountId,
    pub name: String,
    pub unit: UnitId,
    /// The Accounts' rule (`lib_transactions::takes_transactions`), filled by the side that answers
    /// the port, so a reader checks the flag rather than knowing Account types.
    pub takes_transactions: bool,
}
