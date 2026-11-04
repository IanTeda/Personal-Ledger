//! The footer's grand total, shared by the Transactions table and the Bills schedule summary.

use crate::Money;

/// The footer's grand total under the no-cross-Unit rule.
#[derive(Debug, Clone, PartialEq)]
pub enum Total {
    /// No rows are visible.
    Empty,
    /// Every visible row is in one Unit: the sum of the rows' amounts, in that Unit's code. The
    /// code always resolves, since a Unit an Account or Bill Plan names can't be deleted (Unit
    /// delete refuses it and the foreign keys forbid it), so a builder's empty-code fallback is
    /// unreachable (#691).
    Single { unit: String, amount: Money },
    /// The visible rows span more than one Unit, so no sum exists.
    Mixed,
}
