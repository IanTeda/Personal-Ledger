//! Lists in a stored order (#1267): the rows whose order the Ledger keeps rather than sorts by a
//! key, the Document Types (ordered by the person), the Institutions and the Accounts (kept in the
//! order they were added). Each keeps it in a `position` column.
//!
//! A delete or a move records the whole order before and after as one
//! [`RowImage::Order`](crate::reversal::RowImage::Order), which Undo and Redo hand back to the
//! store through [`Ordered::place`], reached by its [`Domain`] through
//! [`SharedStores::ordered_mut`](crate::adapter::store::SharedStores::ordered_mut). An add appends
//! (`MAX(position) + 1`, or the end of memory), so it records no order. The file's side is one
//! writer change, [`PlaceRows`], naming its table.

use std::collections::HashMap;
use std::hash::Hash;

use lib_core::RowID;
use sqlx::SqliteConnection;

use crate::Result;
use crate::writer::{BoxFuture, Change, Domain};

/// A store holding one list in a stored order.
pub trait Ordered {
    /// The rows' ids, in their order.
    fn order(&self) -> Vec<RowID>;

    /// The domain whose order this store keeps.
    fn ordered_domain(&self) -> Domain;

    /// Puts the rows `order` names first, in that order, and any it doesn't name after them, as
    /// Undo and Redo of a delete or a move do.
    fn place(&mut self, order: &[RowID]);
}

/// Sorts `rows` by where `id_of` finds them in `order`, rows it doesn't name going last.
pub(crate) fn place_by<T, I: Hash + Eq>(rows: &mut [T], order: &[I], id_of: impl Fn(&T) -> I) {
    let places: HashMap<&I, usize> = order.iter().enumerate().map(|(at, id)| (id, at)).collect();
    rows.sort_by_cached_key(|row| places.get(&id_of(row)).copied().unwrap_or(usize::MAX));
}

/// Every row's place in `domain`'s list, written as its `position` in `table`.
#[derive(Debug, Clone)]
pub(crate) struct PlaceRows {
    domain: Domain,
    table: &'static str,
    ids: Vec<RowID>,
}

impl PlaceRows {
    /// `store`'s order as it stands, written to `table`, a literal of the adapter's, never input.
    pub(crate) fn of(store: &(impl Ordered + ?Sized), table: &'static str) -> Self {
        Self {
            domain: store.ordered_domain(),
            table,
            ids: store.order(),
        }
    }
}

impl Change for PlaceRows {
    fn domain(&self) -> Domain {
        self.domain
    }

    fn apply<'c>(self: Box<Self>, conn: &'c mut SqliteConnection) -> BoxFuture<'c, Result<()>> {
        Box::pin(async move {
            let sql = format!("UPDATE {} SET position = ? WHERE id = ?", self.table);
            for (place, id) in self.ids.into_iter().enumerate() {
                let place = i64::try_from(place).unwrap_or(i64::MAX);
                sqlx::query(&sql)
                    .bind(place)
                    .bind(id)
                    .execute(&mut *conn)
                    .await?;
            }
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placing_puts_the_named_rows_first_and_keeps_the_rest_after() {
        let mut rows = vec![1, 2, 3, 4];

        place_by(&mut rows, &[3, 1], |row| *row);

        assert_eq!(rows, [3, 1, 2, 4]);
    }

    #[test]
    fn unnamed_rows_keep_their_old_order_after_the_named_ones() {
        let mut rows = vec![5, 4, 3, 2, 1];

        place_by(&mut rows, &[2, 9], |row| *row);

        assert_eq!(rows, [2, 5, 4, 3, 1]);
    }
}
