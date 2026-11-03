//! The Budgets Entity: the rules the Budgets aggregate holds, with no I/O. `budget` is the Budget
//! and its Category Limit chain, `chain` the chain's edits on one Budget, `budgets` the set and the
//! rules across it (a unique name, one default), and `seed` the stub Budgets.
//!
//! Every operation the old `store.rs` held is here rather than in an Interactor: each is a rule the
//! aggregate keeps whichever intent runs it, checked on the Budget or the set alone, and the
//! Interactors only load a copy, run one of these and write what changed (#996).

mod budget;
mod budgets;
mod chain;
mod seed;

pub use budget::*;
pub use budgets::*;
pub use chain::*;
pub use seed::*;
