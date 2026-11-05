//! A count of the writes a page's ports have made, so a page decides whether an input wrote by
//! reading the count before and after it, not by cloning its collection both times and comparing
//! the copies.

use std::cell::Cell;
use std::rc::Rc;

/// How many writes went through the ports holding a clone. A clone shares the one count.
///
/// Only a difference is meaningful: a write made between inputs, by another module sharing the
/// store, moves the count before the next input reads it and so is never mistaken for that
/// input's own.
#[derive(Clone, Default)]
pub struct WriteCount(Rc<Cell<u64>>);

impl WriteCount {
    /// Notes one write.
    pub fn bump(&self) {
        self.0.set(self.0.get().wrapping_add(1));
    }

    /// The writes so far.
    pub fn get(&self) -> u64 {
        self.0.get()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_clone_sees_the_writes_the_other_notes() {
        let count = WriteCount::default();
        let shared = count.clone();
        let before = count.get();

        shared.bump();

        assert_ne!(count.get(), before);
    }

    #[test]
    fn a_read_does_not_move_the_count() {
        let count = WriteCount::default();
        assert_eq!(count.get(), count.get());
    }
}
