//! What undoes and redoes one input's Ledger change (#1240, #1247): the whole-row images the
//! stores wrote, each row's image before the input and after it, recorded below the Interactors so
//! neither they nor the cross-domain ports know of History.
//!
//! [`Recorded`](crate::Recorded) hands out a recording wrapper around a store's repository, which
//! reads the row by id before each write as its `before` and notes it in the [`Journal`] every store
//! shares (through [`Written`](crate::Written)). A row written twice in one input keeps its first
//! `before` and its last `after`, where it was first written. The Client takes the input's images
//! as one [`LedgerReversal`] once the input closes ([`crate::LedgerPorts::take_reversal`]).
//!
//! Undo applies the images newest first, writing each `before` back (deleting the row where it was
//! absent), which keeps foreign keys valid; Redo applies them in the order written, with `after`
//! ([`crate::LedgerPorts::reverse`], [`crate::LedgerPorts::replay`]). Both write through the same
//! stores with the journal paused, the non-recording mark, so stepping through History records
//! nothing. History is linear, so the rows always hold what the entry left them as: there is no
//! conflict check, only a `debug_assert!`.
//!
//! Only the Units store records yet, and the Institutions store, whose rows Unit delete reassigns;
//! the other stores join with their own tickets (#1248, #1249).

use std::cell::RefCell;
use std::rc::Rc;

use lib_core::{InstitutionId, PriceSourceId, UnitId};
use lib_institutions::Institution;
use lib_units::{PriceSourceRecord, UnitRow, UnitUse};

use crate::Domain;
use crate::adapter::store::SharedStores;

/// One row's image before an input and after it; `None` where the row was absent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RowImage {
    Unit(Option<UnitRow>, Option<UnitRow>),
    PriceSource(Option<PriceSourceRecord>, Option<PriceSourceRecord>),
    /// The base Unit is one value rather than a row, so it is never absent.
    BaseUnit(UnitId, UnitId),
    Institution(Option<Institution>, Option<Institution>),
}

/// Which row an image is of, so a second write to it merges into the first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RowKey {
    Unit(UnitId),
    PriceSource(PriceSourceId),
    BaseUnit,
    Institution(InstitutionId),
}

impl RowImage {
    fn key(&self) -> Option<RowKey> {
        Some(match self {
            Self::Unit(before, after) => RowKey::Unit(before.as_ref().or(after.as_ref())?.id),
            Self::PriceSource(before, after) => {
                RowKey::PriceSource(before.as_ref().or(after.as_ref())?.id())
            }
            Self::BaseUnit(..) => RowKey::BaseUnit,
            Self::Institution(before, after) => {
                RowKey::Institution(before.as_ref().or(after.as_ref())?.id)
            }
        })
    }

    /// The store the row is kept in.
    pub fn domain(&self) -> Domain {
        match self {
            Self::Unit(..) | Self::PriceSource(..) | Self::BaseUnit(..) => Domain::Units,
            Self::Institution(..) => Domain::Institutions,
        }
    }

    /// Whether the input left the row as it found it, as an add then a delete does.
    fn unchanged(&self) -> bool {
        match self {
            Self::Unit(before, after) => before == after,
            Self::PriceSource(before, after) => before == after,
            Self::BaseUnit(before, after) => before == after,
            Self::Institution(before, after) => before == after,
        }
    }

    /// Keeps this image's `before` and takes `later`'s `after`.
    fn merge(&mut self, later: Self) {
        match (self, later) {
            (Self::Unit(_, after), Self::Unit(_, later)) => *after = later,
            (Self::PriceSource(_, after), Self::PriceSource(_, later)) => *after = later,
            (Self::BaseUnit(_, after), Self::BaseUnit(_, later)) => *after = later,
            (Self::Institution(_, after), Self::Institution(_, later)) => *after = later,
            (image, later) => debug_assert!(false, "{image:?} merged with {later:?}"),
        }
    }
}

/// One input's Ledger change: its row images in the order first written, each tagged by the store
/// it is kept in.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LedgerReversal {
    images: Vec<RowImage>,
}

impl LedgerReversal {
    pub fn images(&self) -> &[RowImage] {
        &self.images
    }

    /// The domains the change wrote, each once, in the order first written.
    pub fn domains(&self) -> Vec<Domain> {
        let mut domains = Vec::new();
        for image in &self.images {
            let domain = image.domain();
            if !domains.contains(&domain) {
                domains.push(domain);
            }
        }
        domains
    }
}

impl From<Vec<RowImage>> for LedgerReversal {
    fn from(images: Vec<RowImage>) -> Self {
        Self { images }
    }
}

/// Which way a [`LedgerReversal`] is applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Step {
    /// Undo: newest image first, each row back to its `before`.
    Reverse,
    /// Redo: in the order written, each row to its `after`.
    Replay,
}

/// Writes `reversal`'s images to `stores` the way `step` goes, unrecorded in `journal`. Each row
/// is expected to hold the image's other side, which linear History guarantees (`debug_assert!`).
pub(crate) fn apply(
    reversal: &LedgerReversal,
    step: Step,
    stores: &SharedStores,
    journal: &Journal,
) {
    let images: Box<dyn Iterator<Item = &RowImage>> = match step {
        Step::Reverse => Box::new(reversal.images.iter().rev()),
        Step::Replay => Box::new(reversal.images.iter()),
    };
    journal.unrecorded(|| {
        for image in images {
            write(image, step, stores);
        }
    });
}

/// `(expected, target)`: what the row holds now and what it is written to.
fn sides<T>(before: &Option<T>, after: &Option<T>, step: Step) -> (Option<T>, Option<T>)
where
    T: Clone,
{
    match step {
        Step::Reverse => (after.clone(), before.clone()),
        Step::Replay => (before.clone(), after.clone()),
    }
}

fn write(image: &RowImage, step: Step, stores: &SharedStores) {
    let unread = UnitUse::default();
    match image {
        RowImage::Unit(before, after) => {
            let (expected, target) = sides(before, after, step);
            let Some(id) = target.as_ref().or(expected.as_ref()).map(|unit| unit.id) else {
                return;
            };
            stores.units.borrow_mut().split_write(&unread, |ports| {
                debug_assert_eq!(
                    ports.units.unit(id),
                    expected,
                    "the Unit as History left it"
                );
                match target {
                    Some(unit) => ports.units.upsert_unit(unit),
                    None => ports.units.delete_unit(id),
                }
            });
        }
        RowImage::PriceSource(before, after) => {
            let (expected, target) = sides(before, after, step);
            let Some(id) = target
                .as_ref()
                .or(expected.as_ref())
                .map(PriceSourceRecord::id)
            else {
                return;
            };
            stores.units.borrow_mut().split_write(&unread, |ports| {
                debug_assert_eq!(
                    ports
                        .price_sources
                        .price_source_records()
                        .into_iter()
                        .find(|record| record.id() == id),
                    expected,
                    "the Price Source as History left it"
                );
                match target {
                    Some(record) => ports.price_sources.upsert_price_source(record),
                    None => ports.price_sources.delete_price_source(id),
                }
            });
        }
        RowImage::BaseUnit(before, after) => {
            let (expected, target) = match step {
                Step::Reverse => (*after, *before),
                Step::Replay => (*before, *after),
            };
            stores.units.borrow_mut().split_write(&unread, |ports| {
                debug_assert_eq!(
                    ports.base_unit.base_unit(),
                    expected,
                    "the base as History left it"
                );
                ports.base_unit.set_base_unit(target);
            });
        }
        RowImage::Institution(before, after) => {
            let (expected, target) = sides(before, after, step);
            let Some(id) = target.as_ref().or(expected.as_ref()).map(|row| row.id) else {
                return;
            };
            let mut store = stores.institutions.borrow_mut();
            let institutions = store.institutions_mut();
            debug_assert_eq!(
                institutions.institution(id),
                expected,
                "the Institution as History left it"
            );
            match target {
                Some(institution) => institutions.upsert_institution(institution),
                None => institutions.delete_institution(id),
            }
        }
    }
}

#[derive(Debug, Default)]
struct JournalState {
    images: Vec<RowImage>,
    /// The non-recording mark: set while Undo or Redo writes, so stepping makes no entry.
    paused: bool,
}

/// The running input's row images, shared by every store's recorder. Single-threaded, as the
/// ports are; a clone shares the same journal.
#[derive(Debug, Clone, Default)]
pub struct Journal(Rc<RefCell<JournalState>>);

impl Journal {
    /// Notes `image`, merging it into an earlier image of the same row: the first `before` and
    /// the last `after` stand. Nothing is noted while paused.
    pub(crate) fn record(&self, image: RowImage) {
        let mut state = self.0.borrow_mut();
        if state.paused {
            return;
        }
        let key = image.key();
        match state
            .images
            .iter_mut()
            .find(|earlier| key.is_some() && earlier.key() == key)
        {
            Some(earlier) => earlier.merge(image),
            None => state.images.push(image),
        }
    }

    /// The images noted since the last take, leaving none: `None` where the input changed no row.
    pub fn take(&self) -> Option<LedgerReversal> {
        let images: Vec<RowImage> = std::mem::take(&mut self.0.borrow_mut().images)
            .into_iter()
            .filter(|image| !image.unchanged())
            .collect();
        (!images.is_empty()).then_some(LedgerReversal { images })
    }

    /// Runs `write` with the journal paused, so what it writes is not recorded.
    pub(crate) fn unrecorded<T>(&self, write: impl FnOnce() -> T) -> T {
        let paused = std::mem::replace(&mut self.0.borrow_mut().paused, true);
        let written = write();
        self.0.borrow_mut().paused = paused;
        written
    }
}

#[cfg(test)]
mod tests {
    use lib_units::UnitsPorts as _;
    use lib_units::fake::InMemoryPorts;

    use super::*;

    fn unit(code: &str) -> UnitRow {
        let epoch = chrono::DateTime::from_timestamp(0, 0).expect("the epoch is a moment");
        let mut unit = InMemoryPorts::sample(epoch)
            .units()
            .units()
            .into_iter()
            .find(|unit| unit.id == UnitId::SEED_AUD)
            .expect("the sample's aud");
        unit.code = code.into();
        unit
    }

    #[test]
    fn a_row_written_twice_keeps_its_first_before_and_its_last_after() {
        let journal = Journal::default();

        journal.record(RowImage::Unit(Some(unit("aud")), Some(unit("audx"))));
        journal.record(RowImage::BaseUnit(UnitId::SEED_AUD, UnitId::SEED_BTC));
        journal.record(RowImage::Unit(Some(unit("audx")), Some(unit("audy"))));

        assert_eq!(
            journal.take().map(|reversal| reversal.images),
            Some(vec![
                RowImage::Unit(Some(unit("aud")), Some(unit("audy"))),
                RowImage::BaseUnit(UnitId::SEED_AUD, UnitId::SEED_BTC),
            ])
        );
        assert_eq!(journal.take(), None, "taken");
    }

    #[test]
    fn a_row_left_as_found_is_no_change() {
        let journal = Journal::default();

        journal.record(RowImage::Unit(None, Some(unit("nzd"))));
        journal.record(RowImage::Unit(Some(unit("nzd")), None));

        assert_eq!(journal.take(), None);
    }

    #[test]
    fn nothing_is_recorded_while_unrecorded() {
        let journal = Journal::default();

        journal.unrecorded(|| journal.record(RowImage::Unit(None, Some(unit("nzd")))));

        assert_eq!(journal.take(), None);
    }

    /// The Sample Ledger's ports, as the Desktop holds them.
    fn ports() -> crate::LedgerPorts {
        crate::ports::test_support::fakes()
    }

    fn units(ports: &crate::LedgerPorts) -> Vec<UnitRow> {
        let mut units = ports.units().borrow().units().units();
        units.sort_by_key(|unit| unit.code.clone());
        units
    }

    fn institutions(ports: &crate::LedgerPorts) -> Vec<Institution> {
        ports.institutions().borrow().institutions().institutions()
    }

    #[test]
    fn a_units_write_through_the_ports_is_recorded_as_whole_row_images() {
        let ports = ports();
        let mut nzd = unit("nzd");
        nzd.id = UnitId::new();
        let mut renamed = nzd.clone();
        renamed.name = "Kiwi".into();

        {
            let shared = ports.units();
            let mut units = shared.borrow_mut();
            let write = units.split_mut();
            write.units.upsert_unit(nzd.clone());
            write.units.upsert_unit(renamed.clone());
            write.base_unit.set_base_unit(nzd.id);
        }

        assert_eq!(
            ports.take_reversal().map(|reversal| reversal.images),
            Some(vec![
                RowImage::Unit(None, Some(renamed)),
                RowImage::BaseUnit(UnitId::SEED_AUD, nzd.id),
            ])
        );
    }

    #[test]
    fn reverse_puts_the_rows_back_and_replay_writes_them_again_recording_neither() {
        let ports = ports();
        let before = units(&ports);
        let mut nzd = unit("nzd");
        nzd.id = UnitId::new();
        {
            let shared = ports.units();
            let mut units = shared.borrow_mut();
            let write = units.split_mut();
            write.units.upsert_unit(nzd.clone());
            write.units.delete_unit(UnitId::SEED_BTC);
        }
        let after = units(&ports);
        let reversal = ports.take_reversal().expect("the writes were recorded");

        ports.reverse(&reversal);
        assert_eq!(units(&ports), before, "every row back under its own id");
        assert_eq!(ports.take_reversal(), None, "Undo records nothing");

        ports.replay(&reversal);
        assert_eq!(units(&ports), after);
        assert_eq!(ports.take_reversal(), None, "Redo records nothing");
    }

    #[test]
    fn undoing_a_unit_delete_restores_the_unit_the_base_and_the_reassigned_institutions() {
        let ports = ports();
        let before = (units(&ports), institutions(&ports));
        let mut reassigned: Vec<Institution> = institutions(&ports)
            .into_iter()
            .filter(|institution| institution.usual_unit == Some(UnitId::SEED_AUD))
            .collect();
        let crypto = institutions(&ports)
            .into_iter()
            .find(|institution| institution.usual_unit == Some(UnitId::SEED_BTC))
            .expect("the sample's crypto exchange counts in btc");
        reassigned.push(Institution {
            usual_unit: Some(UnitId::SEED_AUD),
            ..crypto
        });
        ports
            .unit_deletion()
            .borrow_mut()
            .remove_unit(lib_unit_deletion::UnitRemoval {
                unit: UnitId::SEED_BTC,
                new_base: None,
                reassigned,
            });
        let reversal = ports.take_reversal().expect("the delete was recorded");
        assert_eq!(
            reversal.domains(),
            [Domain::Institutions, Domain::Units],
            "the Institutions first, then the Unit"
        );

        ports.reverse(&reversal);

        assert_eq!((units(&ports), institutions(&ports)), before);
    }

    #[test]
    fn undo_writes_reach_the_commit_as_the_domains_they_wrote() {
        let ports = ports();
        let commit = ports.commit();
        {
            let shared = ports.units();
            let mut units = shared.borrow_mut();
            units.split_mut().units.delete_unit(UnitId::SEED_VAS);
        }
        commit.commit();
        let reversal = ports.take_reversal().expect("the delete was recorded");

        ports.reverse(&reversal);

        assert_eq!(commit.commit().wrote, [Domain::Units]);
    }

    #[test]
    fn an_undone_delete_lands_in_the_ledger_file_under_its_own_id() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let ports = crate::ports::test_support::file(dir.path());
        let mut nzd = unit("nzd");
        nzd.id = UnitId::new();
        {
            let shared = ports.units();
            let mut units = shared.borrow_mut();
            units.split_mut().units.upsert_unit(nzd.clone());
        }
        ports.commit().commit();
        let added = ports.take_reversal().expect("the add was recorded");
        ports.reverse(&added);
        ports.commit().commit();
        ports.replay(&added);
        ports.commit().commit();

        ports.reload(Domain::Units).expect("the Units read back");

        assert!(units(&ports).contains(&nzd), "redone under its own id");
    }
}
