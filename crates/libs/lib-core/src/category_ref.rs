//! A Category as the modules that don't own Categories read it.

use crate::{CategoryId, CategoryTypes};

/// A Category as another module reads it: a shared read value, one type for every module's
/// Category read port rather than one per crate. It carries the union of its readers' fields, so
/// a reader may ignore some of them, and a field added here recompiles every reader. `path`
/// (ancestors then `name`) and `is_leaf` are the Categories' rules, filled by the side that
/// answers the port.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategoryRef {
    pub id: CategoryId,
    pub name: String,
    pub path: String,
    pub parent: Option<CategoryId>,
    pub category_type: CategoryTypes,
    pub is_leaf: bool,
}

impl CategoryRef {
    pub fn is_expense(&self) -> bool {
        self.category_type == CategoryTypes::Expense
    }
}
