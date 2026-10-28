//! The Unit Deletion adapter: `lib-unit-deletion`'s atomic port over the Units and Institutions
//! stores (ADR-0046), after the Account Deletion adapter. It holds no rows of its own. A delete's
//! reassigned Institutions go to the Institutions store first, then the base's move and the Unit's
//! removal to the Units store, and every write joins the [`PendingWrites`] every adapter shares, so
//! the bin's one commit after the call sends them as one transaction: a refused Unit delete takes
//! the reassignments and the base's move down with it. `institutions.usual_unit_id` has no cascade,
//! so the Institutions must stop naming the Unit before it goes.
//!
//! [`PendingWrites`]: crate::PendingWrites

use lib_core::UnitId;
use lib_institutions::Institution;
use lib_unit_deletion::{UnitDeletion, UnitRemoval};
use lib_units::{UnitRow, UnitUse};

use super::store::{SharedInstitutionsStore, SharedUnitsStore};

/// The deletion port over the stores the Units and Institutions modules also hold.
pub struct UnitDeletionAdapter {
    units: SharedUnitsStore,
    institutions: SharedInstitutionsStore,
}

impl UnitDeletionAdapter {
    pub fn new(units: SharedUnitsStore, institutions: SharedInstitutionsStore) -> Self {
        Self {
            units,
            institutions,
        }
    }
}

impl UnitDeletion for UnitDeletionAdapter {
    fn units(&self) -> Vec<UnitRow> {
        self.units.borrow().units().units()
    }

    fn base_unit(&self) -> UnitId {
        self.units.borrow().base_unit().base_unit()
    }

    fn institutions_naming(&self, unit: UnitId) -> Vec<Institution> {
        lib_unit_deletion::protocols::naming(
            self.institutions.borrow().institutions().institutions(),
            unit,
        )
    }

    /// The Institutions first, then the base, then the Unit: nothing may name the Unit when it
    /// goes, and the Ledger is never without a base.
    fn remove_unit(&mut self, removal: UnitRemoval) {
        let UnitRemoval {
            unit,
            new_base,
            reassigned,
        } = removal;
        {
            let mut store = self.institutions.borrow_mut();
            for institution in reassigned {
                store.institutions_mut().upsert_institution(institution);
            }
        }
        let mut store = self.units.borrow_mut();
        // The usage `split_mut` lends beside the write ports goes unread: the Interactor checked
        // the usage before writing.
        let unread = UnitUse::default();
        let ports = store.split_mut(&unread);
        if let Some(new_base) = new_base {
            ports.base_unit.set_base_unit(new_base);
        }
        ports.units.delete_unit(unit);
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::sync::mpsc;

    use lib_institutions::InstitutionRepository as _;
    use lib_unit_deletion::contract::{self, DeletionFactory, Stored};
    use lib_unit_deletion::fake::InMemoryUnitDeletion;
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

    use super::*;
    use crate::commit::Commit;
    use crate::writer::WriteFailure;
    use crate::{InstitutionsAdapter, PendingWrites, UnitsAdapter, Writer};

    /// The two adapters on one test database and one pending batch, as start-up opens them.
    struct AdapterFactory {
        writer: Writer,
        pending: PendingWrites,
        failures: mpsc::Receiver<WriteFailure>,
    }

    impl AdapterFactory {
        fn new(options: SqliteConnectOptions) -> Self {
            let (writer, failures) = Writer::spawn(options).expect("the test database opens");
            Self {
                pending: PendingWrites::new(writer.clone()),
                writer,
                failures,
            }
        }

        /// Commits, settles the writes, and reads back what is stored with fresh adapters, as a
        /// restart would.
        fn settle(&mut self) -> (Stored, usize) {
            self.pending.commit();
            self.writer.flush().expect("the writer is running");
            let failures = std::iter::from_fn(|| self.failures.try_recv().ok()).count();
            let fresh = PendingWrites::new(self.writer.clone());
            let units = UnitsAdapter::load(fresh.clone()).expect("the Units load");
            let institutions = InstitutionsAdapter::load(fresh).expect("the Institutions load");
            let stored = Stored::new(
                units.units().units().iter().map(|unit| unit.id).collect(),
                units.base_unit().base_unit(),
                institutions.institutions(),
            );
            (stored, failures)
        }
    }

    impl DeletionFactory for AdapterFactory {
        type Ports = UnitDeletionAdapter;

        fn create(&mut self, rows: InMemoryUnitDeletion) -> UnitDeletionAdapter {
            let units = lib_units::fake::InMemoryPorts {
                units: rows.units,
                base_unit: rows.base_unit,
                ..lib_units::fake::InMemoryPorts::stub(lib_unit_deletion::fake::world::now())
            };
            UnitsAdapter::seed_from(&self.writer, units).expect("the Units are written");
            InstitutionsAdapter::seed(&self.writer, rows.institutions.institutions)
                .expect("the Institutions are written");
            let units = UnitsAdapter::load(self.pending.clone()).expect("the Units load");
            let institutions =
                InstitutionsAdapter::load(self.pending.clone()).expect("the Institutions load");
            UnitDeletionAdapter::new(
                Rc::new(RefCell::new(units)),
                Rc::new(RefCell::new(institutions)),
            )
        }

        fn stored(&mut self, _ports: UnitDeletionAdapter) -> Stored {
            let (stored, failures) = self.settle();
            assert_eq!(failures, 0, "a write failed");
            stored
        }

        fn break_unit_deletes(&mut self, _ports: &mut UnitDeletionAdapter) {
            self.writer
                .run(|pool| async move {
                    sqlx::query(
                        "CREATE TRIGGER refuse_unit_delete BEFORE DELETE ON units \
                         BEGIN SELECT RAISE(ABORT, 'deletes are refused'); END",
                    )
                    .execute(&pool)
                    .await?;
                    Ok(())
                })
                .expect("the trigger is created");
        }

        fn recover(&mut self, _ports: UnitDeletionAdapter) -> (Stored, usize) {
            self.settle()
        }
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn a_delete_lands_the_unit_the_base_move_and_the_reassignment(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        contract::a_delete_lands_the_unit_the_base_move_and_the_reassignment(
            &mut AdapterFactory::new(options),
        );
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn a_delete_left_on_none_lands_the_institutions_with_none(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        contract::a_delete_left_on_none_lands_the_institutions_with_none(&mut AdapterFactory::new(
            options,
        ));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn a_failing_delete_lands_nothing(_: SqlitePoolOptions, options: SqliteConnectOptions) {
        contract::a_failing_delete_lands_nothing(&mut AdapterFactory::new(options));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn a_refused_delete_writes_nothing(_: SqlitePoolOptions, options: SqliteConnectOptions) {
        contract::a_refused_delete_writes_nothing(&mut AdapterFactory::new(options));
    }
}
