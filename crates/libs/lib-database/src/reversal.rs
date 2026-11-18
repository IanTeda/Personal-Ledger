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
//! The Units, the Institutions (whose rows Unit delete reassigns), the Transactions, Bills,
//! Budgets and Documents record (#1248), with the Document Types beside the Documents; the other
//! stores join with #1249.
//!
//! Where a delete's SQL cascade removes rows in another store (the Document Links to a deleted
//! Transaction, Account, Payee or Item, and an Account's place in the Budgets that count it), the
//! recorder notes those rows' images first ([`Journal::unlink`], [`Journal::unlink_account`]),
//! ahead of the deleted row's own, so Undo, which goes newest first, restores the row before the
//! rows that name it. Memory still holds the rows when the delete is made, since the Client reloads
//! the cascade only after the commit, so the recorder reads them from the stores it is attached to
//! ([`Journal::attach`]), and a second cascade into the same row builds on the first's image.
//!
//! A Bill Plan, a Bill Schedule entry or a Budget new to the Ledger has no image before it: the
//! repositories have no delete, so such a write can't be undone and isn't recorded (the Bill
//! Schedule's generation writes only these).

use std::cell::RefCell;
use std::rc::{Rc, Weak};

use lib_bills::{BillPlan, BillScheduleEntry, EntryId};
use lib_budgets::Budget;
use lib_core::{
    AccountId, BillPlanId, BudgetId, DocumentId, DocumentTypeId, InstitutionId, PriceSourceId,
    TransactionId, UnitId,
};
use lib_documents::types::DocumentTypeRow;
use lib_documents::{Document, DocumentLink};
use lib_institutions::Institution;
use lib_transactions::Transaction;
use lib_units::{PriceSourceRecord, UnitRow, UnitUse};

use crate::Domain;
use crate::adapter::store::{
    BudgetsStore, DocumentsStore, SharedBudgetsStore, SharedDocumentsStore, SharedStores,
};

/// One row's image before an input and after it; `None` where the row was absent.
#[derive(Debug, Clone, PartialEq)]
pub enum RowImage {
    Unit(Option<UnitRow>, Option<UnitRow>),
    PriceSource(Option<PriceSourceRecord>, Option<PriceSourceRecord>),
    /// The base Unit is one value rather than a row, so it is never absent.
    BaseUnit(UnitId, UnitId),
    Institution(Option<Institution>, Option<Institution>),
    /// A whole Transaction with its Splits.
    Transaction(Option<Transaction>, Option<Transaction>),
    BillPlan(Option<BillPlan>, Option<BillPlan>),
    BillEntry(Option<BillScheduleEntry>, Option<BillScheduleEntry>),
    /// A whole Budget with its Accounts and every chain.
    Budget(Option<Budget>, Option<Budget>),
    /// A whole Document with its Intake and Links.
    Document(Option<Document>, Option<Document>),
    DocumentType(Option<DocumentTypeRow>, Option<DocumentTypeRow>),
    /// Every Document Type's place, before and after one was added, moved or removed.
    DocumentTypeOrder(Vec<DocumentTypeId>, Vec<DocumentTypeId>),
}

/// Which row an image is of, so a second write to it merges into the first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RowKey {
    Unit(UnitId),
    PriceSource(PriceSourceId),
    BaseUnit,
    Institution(InstitutionId),
    Transaction(TransactionId),
    BillPlan(BillPlanId),
    BillEntry(EntryId),
    Budget(BudgetId),
    Document(DocumentId),
    DocumentType(DocumentTypeId),
    DocumentTypeOrder,
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
            Self::Transaction(before, after) => {
                RowKey::Transaction(before.as_ref().or(after.as_ref())?.id)
            }
            Self::BillPlan(before, after) => {
                RowKey::BillPlan(before.as_ref().or(after.as_ref())?.id)
            }
            Self::BillEntry(before, after) => {
                RowKey::BillEntry(before.as_ref().or(after.as_ref())?.id())
            }
            Self::Budget(before, after) => RowKey::Budget(before.as_ref().or(after.as_ref())?.id),
            Self::Document(before, after) => {
                RowKey::Document(before.as_ref().or(after.as_ref())?.id)
            }
            Self::DocumentType(before, after) => {
                RowKey::DocumentType(before.as_ref().or(after.as_ref())?.id)
            }
            Self::DocumentTypeOrder(..) => RowKey::DocumentTypeOrder,
        })
    }

    /// Whether Undo could put the row back: a Bill Plan, an entry or a Budget new to the Ledger
    /// couldn't be removed again, since their repositories have no delete.
    fn undoable(&self) -> bool {
        match self {
            Self::BillPlan(before, _) => before.is_some(),
            Self::BillEntry(before, _) => before.is_some(),
            Self::Budget(before, _) => before.is_some(),
            _ => true,
        }
    }

    /// The store the row is kept in.
    pub fn domain(&self) -> Domain {
        match self {
            Self::Unit(..) | Self::PriceSource(..) | Self::BaseUnit(..) => Domain::Units,
            Self::Institution(..) => Domain::Institutions,
            Self::Transaction(..) => Domain::Transactions,
            Self::BillPlan(..) | Self::BillEntry(..) => Domain::Bills,
            Self::Budget(..) => Domain::Budgets,
            Self::Document(..) => Domain::Documents,
            Self::DocumentType(..) | Self::DocumentTypeOrder(..) => Domain::DocumentTypes,
        }
    }

    /// Whether the input left the row as it found it, as an add then a delete does.
    fn unchanged(&self) -> bool {
        match self {
            Self::Unit(before, after) => before == after,
            Self::PriceSource(before, after) => before == after,
            Self::BaseUnit(before, after) => before == after,
            Self::Institution(before, after) => before == after,
            Self::Transaction(before, after) => before == after,
            Self::BillPlan(before, after) => before == after,
            Self::BillEntry(before, after) => before == after,
            Self::Budget(before, after) => before == after,
            Self::Document(before, after) => before == after,
            Self::DocumentType(before, after) => before == after,
            Self::DocumentTypeOrder(before, after) => before == after,
        }
    }

    /// Keeps this image's `before` and takes `later`'s `after`.
    fn merge(&mut self, later: Self) {
        match (self, later) {
            (Self::Unit(_, after), Self::Unit(_, later)) => *after = later,
            (Self::PriceSource(_, after), Self::PriceSource(_, later)) => *after = later,
            (Self::BaseUnit(_, after), Self::BaseUnit(_, later)) => *after = later,
            (Self::Institution(_, after), Self::Institution(_, later)) => *after = later,
            (Self::Transaction(_, after), Self::Transaction(_, later)) => *after = later,
            (Self::BillPlan(_, after), Self::BillPlan(_, later)) => *after = later,
            (Self::BillEntry(_, after), Self::BillEntry(_, later)) => *after = later,
            (Self::Budget(_, after), Self::Budget(_, later)) => *after = later,
            (Self::Document(_, after), Self::Document(_, later)) => *after = later,
            (Self::DocumentType(_, after), Self::DocumentType(_, later)) => *after = later,
            (Self::DocumentTypeOrder(_, after), Self::DocumentTypeOrder(_, later)) => {
                *after = later
            }
            (image, later) => debug_assert!(false, "{image:?} merged with {later:?}"),
        }
    }
}

/// One input's Ledger change: its row images in the order first written, each tagged by the store
/// it is kept in.
#[derive(Debug, Clone, Default, PartialEq)]
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
        RowImage::Transaction(before, after) => {
            let (expected, target) = sides(before, after, step);
            let Some(id) = target.as_ref().or(expected.as_ref()).map(|row| row.id) else {
                return;
            };
            let mut store = stores.transactions.borrow_mut();
            let transactions = store.transactions_mut();
            debug_assert_eq!(
                transactions.transaction(id),
                expected,
                "the Transaction as History left it"
            );
            match target {
                Some(transaction) => transactions.upsert_transaction(transaction),
                None => transactions.delete_transaction(id),
            }
        }
        // A Bill Plan, an entry and a Budget are never deleted, and only ones that existed before
        // the input are recorded, so every side written is a row.
        RowImage::BillPlan(before, after) => {
            let (expected, target) = sides(before, after, step);
            let mut store = stores.bills.borrow_mut();
            if let Some(plan) = target {
                debug_assert_eq!(
                    store.plans().bill_plan(plan.id),
                    expected,
                    "the Bill Plan as History left it"
                );
                store.plans_mut().upsert_bill_plan(plan);
            }
        }
        RowImage::BillEntry(before, after) => {
            let (expected, target) = sides(before, after, step);
            let mut store = stores.bills.borrow_mut();
            if let Some(entry) = target {
                debug_assert_eq!(
                    store.entries().entry(entry.id()),
                    expected,
                    "the Bill Schedule entry as History left it"
                );
                store.entries_mut().upsert_entry(entry);
            }
        }
        RowImage::Budget(before, after) => {
            let (expected, target) = sides(before, after, step);
            let mut store = stores.budgets.borrow_mut();
            if let Some(budget) = target {
                debug_assert_eq!(
                    store.budgets().budget(budget.id),
                    expected,
                    "the Budget as History left it"
                );
                store.budgets_mut().upsert_budget(&budget);
            }
        }
        // Not checked against the row: a cascade changes a Document's Links in the file and not
        // in memory until the Client reloads, which the in-memory stores never do.
        RowImage::Document(before, after) => {
            let (expected, target) = sides(before, after, step);
            let Some(id) = target.as_ref().or(expected.as_ref()).map(|row| row.id) else {
                return;
            };
            let mut store = stores.documents.borrow_mut();
            match target {
                Some(document) => store.documents_mut().upsert_document(&document),
                None => store.documents_mut().delete_document(id),
            }
        }
        RowImage::DocumentType(before, after) => {
            let (expected, target) = sides(before, after, step);
            let Some(id) = target.as_ref().or(expected.as_ref()).map(|row| row.id) else {
                return;
            };
            let mut store = stores.documents.borrow_mut();
            let documents = store.documents_mut();
            match target {
                Some(row) => documents.upsert_document_type(&row),
                None => documents.delete_document_type(id),
            }
        }
        RowImage::DocumentTypeOrder(before, after) => {
            let (_, target) = sides(&Some(before.clone()), &Some(after.clone()), step);
            let mut store = stores.documents.borrow_mut();
            let documents = store.documents_mut();
            let held: Vec<DocumentTypeId> = documents
                .document_types()
                .into_iter()
                .map(|row| row.id)
                .collect();
            let present = target.into_iter().flatten().filter(|id| held.contains(id));
            for (place, id) in present.enumerate() {
                documents.move_document_type(id, place);
            }
        }
    }
}

#[derive(Debug, Default)]
struct JournalState {
    images: Vec<RowImage>,
    /// The non-recording mark: set while Undo or Redo writes, so stepping makes no entry.
    paused: bool,
    /// The stores a delete's cascade reads the rows it will change from.
    cascades: Option<Cascades>,
}

/// The stores a cascade's rows are read from, held weakly since they hold the journal.
#[derive(Debug, Clone)]
struct Cascades {
    documents: Weak<RefCell<dyn DocumentsStore>>,
    budgets: Weak<RefCell<dyn BudgetsStore>>,
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
        if state.paused || !image.undoable() {
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

    /// Points the journal at the stores a delete's cascade changes rows in, which
    /// [`Journal::unlink`] and [`Journal::unlink_account`] read. Held weakly, since the stores
    /// hold the journal.
    pub(crate) fn attach(&self, documents: &SharedDocumentsStore, budgets: &SharedBudgetsStore) {
        self.0.borrow_mut().cascades = Some(Cascades {
            documents: Rc::downgrade(documents),
            budgets: Rc::downgrade(budgets),
        });
    }

    /// Notes the Documents that lose `link` when the record it names is deleted, each as its
    /// image before and after. To be called before the deleted row's own image is recorded.
    pub(crate) fn unlink(&self, link: DocumentLink) {
        let Some(documents) = self.cascade_documents() else {
            return;
        };
        for document in documents {
            let latest = self.latest(&RowKey::Document(document.id), |image| match image {
                RowImage::Document(_, after) => Some(after.clone()),
                _ => None,
            });
            // A Document an earlier write in the input deleted has no Links left to lose.
            let Some(base) = latest.unwrap_or_else(|| Some(document.clone())) else {
                continue;
            };
            if !base.links.contains(&link) {
                continue;
            }
            let mut after = base;
            after.links.retain(|held| *held != link);
            self.record(RowImage::Document(Some(document), Some(after)));
        }
    }

    /// Notes what a deleted Account takes with it in the file: the Document Links to it, and its
    /// place in the Budgets that count it. To be called before the Account's own image.
    pub(crate) fn unlink_account(&self, account: AccountId) {
        self.unlink(DocumentLink::Account(account));
        let Some(budgets) = self.cascade_budgets() else {
            return;
        };
        for budget in budgets {
            let latest = self.latest(&RowKey::Budget(budget.id), |image| match image {
                RowImage::Budget(_, after) => Some(after.clone()),
                _ => None,
            });
            let Some(base) = latest.unwrap_or_else(|| Some(budget.clone())) else {
                continue;
            };
            if !base.account_ids.contains(&account) {
                continue;
            }
            let mut after = base;
            after.account_ids.retain(|held| *held != account);
            self.record(RowImage::Budget(Some(budget), Some(after)));
        }
    }

    /// What the input's image of the row `key` names says about it now, from `read`.
    fn latest<T>(&self, key: &RowKey, read: impl Fn(&RowImage) -> Option<T>) -> Option<T> {
        let state = self.0.borrow();
        state
            .images
            .iter()
            .find(|image| image.key().as_ref() == Some(key))
            .and_then(read)
    }

    /// The Documents as stored now, unless nothing is to be recorded or the store is busy.
    fn cascade_documents(&self) -> Option<Vec<Document>> {
        let state = self.0.borrow();
        if state.paused {
            return None;
        }
        let store = state.cascades.as_ref()?.documents.upgrade()?;
        let store = store.try_borrow().ok()?;
        Some(store.documents().documents())
    }

    /// The Budgets as stored now, unless nothing is to be recorded or the store is busy.
    fn cascade_budgets(&self) -> Option<Vec<Budget>> {
        let state = self.0.borrow();
        if state.paused {
            return None;
        }
        let store = state.cascades.as_ref()?.budgets.upgrade()?;
        let store = store.try_borrow().ok()?;
        Some(store.budgets().budgets())
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

    fn transactions(ports: &crate::LedgerPorts) -> Vec<Transaction> {
        let mut rows = ports.stores().transactions.borrow().all_transactions();
        rows.sort_by_key(|row| row.id);
        rows
    }

    fn documents(ports: &crate::LedgerPorts) -> Vec<Document> {
        let mut rows = ports.stores().documents.borrow().documents().documents();
        rows.sort_by_key(|row| row.id);
        rows
    }

    fn budgets(ports: &crate::LedgerPorts) -> Vec<Budget> {
        ports.stores().budgets.borrow().budgets().budgets()
    }

    /// Undoes and redoes `reversal`, checking the rows come back as they were and as they were left.
    fn undone_and_redone<T: PartialEq + std::fmt::Debug>(
        ports: &crate::LedgerPorts,
        reversal: &LedgerReversal,
        rows: impl Fn(&crate::LedgerPorts) -> T,
        before: &T,
        after: &T,
    ) {
        ports.reverse(reversal);
        assert_eq!(&rows(ports), before, "every row back under its own id");
        assert_eq!(ports.take_reversal(), None, "Undo records nothing");
        ports.replay(reversal);
        assert_eq!(&rows(ports), after);
        assert_eq!(ports.take_reversal(), None, "Redo records nothing");
    }

    #[test]
    fn a_transaction_added_edited_and_deleted_is_each_undone_and_redone() {
        let ports = ports();
        let original = transactions(&ports)[0].clone();
        let mut added = original.clone();
        added.id = TransactionId::new();
        let mut edited = original.clone();
        edited.description = Some("edited".into());

        for write in [
            Box::new(|ports: &crate::LedgerPorts| {
                ports
                    .stores()
                    .transactions
                    .borrow_mut()
                    .upsert_transaction(added.clone());
            }) as Box<dyn Fn(&crate::LedgerPorts)>,
            Box::new(|ports| {
                ports
                    .stores()
                    .transactions
                    .borrow_mut()
                    .upsert_transaction(edited.clone());
            }),
            Box::new(|ports| {
                ports
                    .stores()
                    .transactions
                    .borrow_mut()
                    .transactions_mut()
                    .delete_transaction(original.id);
            }),
        ] {
            let before = transactions(&ports);
            write(&ports);
            let after = transactions(&ports);
            assert_ne!(before, after);
            let reversal = ports.take_reversal().expect("the write was recorded");
            assert_eq!(reversal.domains(), [Domain::Transactions]);
            undone_and_redone(&ports, &reversal, transactions, &before, &after);
        }
    }

    #[test]
    fn a_bill_payment_is_undone_and_redone_with_its_transaction_and_entry() {
        let ports = ports();
        let entry = ports
            .stores()
            .bills
            .borrow()
            .entries()
            .entries()
            .into_iter()
            .find(BillScheduleEntry::is_open)
            .expect("the sample has an open entry");
        let plan = ports
            .stores()
            .bills
            .borrow()
            .plans()
            .bill_plan(entry.plan_id)
            .expect("its plan");
        let rows = |ports: &crate::LedgerPorts| {
            (
                transactions(ports),
                ports.stores().bills.borrow().entries().entries(),
            )
        };
        let before = rows(&ports);

        lib_bill_payments::interactor::PayBill {
            entry: entry.id(),
            amount: plan.planned_amount,
            date: entry.due,
        }
        .run(&mut *ports.bill_payments().borrow_mut())
        .expect("the entry is open");
        let after = rows(&ports);
        let reversal = ports.take_reversal().expect("the payment was recorded");

        assert_eq!(reversal.domains(), [Domain::Transactions, Domain::Bills]);
        assert_ne!(before, after);
        let sorted = |mut rows: (Vec<Transaction>, Vec<BillScheduleEntry>)| {
            rows.1.sort_by_key(|entry| entry.id());
            rows
        };
        ports.reverse(&reversal);
        assert_eq!(sorted(rows(&ports)), sorted(before.clone()));
        ports.replay(&reversal);
        assert_eq!(sorted(rows(&ports)), sorted(after));
    }

    #[test]
    fn an_import_is_undone_and_redone_with_every_transaction_it_booked() {
        let ports = ports();
        let original = transactions(&ports)[0].clone();
        let booked: Vec<Transaction> = (0..3)
            .map(|_| Transaction {
                id: TransactionId::new(),
                ..original.clone()
            })
            .collect();
        let before = transactions(&ports);

        let import = ports
            .transaction_import(Rc::new(lib_transaction_import::SeededStatement::stub(
                crate::ports::test_support::today(),
            )))
            .import;
        import
            .borrow_mut()
            .commit(lib_transaction_import::ImportPlan {
                payees: Vec::new(),
                transactions: booked,
            })
            .expect("the plan holds");
        let after = transactions(&ports);
        let reversal = ports.take_reversal().expect("the import was recorded");

        assert_eq!(reversal.domains(), [Domain::Transactions]);
        assert_eq!(after.len(), before.len() + 3);
        undone_and_redone(&ports, &reversal, transactions, &before, &after);
    }

    #[test]
    fn a_budget_limit_change_is_undone_and_redone() {
        let ports = ports();
        let before = budgets(&ports);
        let mut changed = before[0].clone();
        let chain = changed
            .limits
            .values_mut()
            .find(|chain| chain.len() > 1)
            .expect("a Budget with a chain to shorten");
        chain.pop();
        ports
            .stores()
            .budgets
            .borrow_mut()
            .budgets_mut()
            .upsert_budget(&changed);
        let after = budgets(&ports);
        let reversal = ports.take_reversal().expect("the change was recorded");

        assert_eq!(reversal.domains(), [Domain::Budgets]);
        undone_and_redone(&ports, &reversal, budgets, &before, &after);
    }

    #[test]
    fn a_bill_plan_or_budget_new_to_the_ledger_is_not_recorded() {
        let ports = ports();
        let mut plan = ports.stores().bills.borrow().plans().bill_plans()[0].clone();
        plan.id = BillPlanId::new();
        let mut budget = budgets(&ports)[0].clone();
        budget.id = BudgetId::new();

        ports
            .stores()
            .bills
            .borrow_mut()
            .plans_mut()
            .upsert_bill_plan(plan);
        ports
            .stores()
            .budgets
            .borrow_mut()
            .budgets_mut()
            .upsert_budget(&budget);

        assert_eq!(ports.take_reversal(), None, "neither can be deleted again");
    }

    #[test]
    fn filing_adding_and_deleting_a_document_are_each_undone_and_redone() {
        let ports = ports();
        let unfiled = documents(&ports)
            .into_iter()
            .find(|document| !document.is_filed())
            .expect("the sample has an Unfiled Document");
        let doc_type = ports
            .stores()
            .documents
            .borrow()
            .documents()
            .document_types()
            .into_iter()
            .find(|row| row.is_default)
            .expect("the default type");
        let mut added = unfiled.clone();
        added.id = DocumentId::new();

        let filed = |ports: &crate::LedgerPorts| {
            lib_documents::interactor::FileByHand {
                id: unfiled.id,
                doc_type: lib_documents::DocumentType(doc_type.id),
                link: Some(DocumentLink::Transaction(transactions(ports)[0].id)),
            }
            .run(&mut *ports.documents().borrow_mut())
            .expect("an Unfiled Document files");
        };
        for write in [
            Box::new(filed) as Box<dyn Fn(&crate::LedgerPorts)>,
            Box::new(|ports| {
                ports
                    .stores()
                    .documents
                    .borrow_mut()
                    .documents_mut()
                    .upsert_document(&added);
            }),
            Box::new(|ports| {
                ports
                    .stores()
                    .documents
                    .borrow_mut()
                    .documents_mut()
                    .delete_document(unfiled.id);
            }),
        ] {
            let before = documents(&ports);
            write(&ports);
            let after = documents(&ports);
            assert_ne!(before, after);
            let reversal = ports.take_reversal().expect("the write was recorded");
            assert_eq!(reversal.domains(), [Domain::Documents]);
            undone_and_redone(&ports, &reversal, documents, &before, &after);
        }
    }

    #[test]
    fn a_document_type_added_moved_and_removed_is_undone_and_redone_in_its_place() {
        let ports = ports();
        let types = |ports: &crate::LedgerPorts| {
            ports
                .stores()
                .documents
                .borrow()
                .documents()
                .document_types()
        };
        let first = types(&ports)[0].clone();
        let mut added = first.clone();
        added.id = DocumentTypeId::new();
        added.is_default = false;
        added.name = "Warranty".into();

        let writes: [Write<'_>; 3] = [
            Box::new(|ports| {
                ports
                    .stores()
                    .documents
                    .borrow_mut()
                    .documents_mut()
                    .upsert_document_type(&added);
            }),
            Box::new(|ports| {
                ports
                    .stores()
                    .documents
                    .borrow_mut()
                    .documents_mut()
                    .move_document_type(first.id, 3);
            }),
            Box::new(|ports| {
                ports
                    .stores()
                    .documents
                    .borrow_mut()
                    .documents_mut()
                    .delete_document_type(added.id);
            }),
        ];
        for write in writes {
            let before = types(&ports);
            write(&ports);
            let after = types(&ports);
            assert_ne!(before, after);
            let reversal = ports.take_reversal().expect("the write was recorded");
            assert_eq!(reversal.domains(), [Domain::DocumentTypes]);
            undone_and_redone(&ports, &reversal, types, &before, &after);
        }
    }

    /// One write to the ports, for the tests that run each in turn.
    type Write<'a> = Box<dyn Fn(&crate::LedgerPorts) + 'a>;

    /// A Document linked to two Transactions, which the Documents' cascade tests delete.
    fn linked_to_two(ports: &crate::LedgerPorts) -> (Document, [TransactionId; 2]) {
        let [first, second] = [0, 1].map(|at| transactions(ports)[at].id);
        let mut document = documents(ports)[0].clone();
        document.links = vec![
            DocumentLink::Transaction(first),
            DocumentLink::Transaction(second),
        ];
        ports
            .stores()
            .documents
            .borrow_mut()
            .documents_mut()
            .upsert_document(&document);
        ports.take_reversal();
        (document, [first, second])
    }

    #[test]
    fn deleting_two_transactions_a_document_links_restores_both_links_on_undo() {
        let ports = ports();
        let (document, [first, second]) = linked_to_two(&ports);
        for id in [first, second] {
            ports
                .stores()
                .transactions
                .borrow_mut()
                .transactions_mut()
                .delete_transaction(id);
        }
        let reversal = ports.take_reversal().expect("the deletes were recorded");

        let [
            RowImage::Document(Some(before), Some(after)),
            RowImage::Transaction(Some(_), None),
            RowImage::Transaction(Some(_), None),
        ] = reversal.images()
        else {
            panic!("the Document's cascade first, then each Transaction: {reversal:?}");
        };
        assert_eq!(before, &document);
        assert!(after.links.is_empty(), "both links go with their records");

        ports.reverse(&reversal);
        assert_eq!(
            documents(&ports)
                .into_iter()
                .find(|held| held.id == document.id),
            Some(document)
        );
    }

    #[test]
    fn undoing_a_transaction_delete_restores_the_links_the_cascade_removed_in_the_file() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let ports = crate::ports::test_support::file(dir.path());
        let (document, [linked, _]) = linked_to_two(&ports);
        ports.commit().commit();
        let before = (transactions(&ports), documents(&ports));

        ports
            .stores()
            .transactions
            .borrow_mut()
            .transactions_mut()
            .delete_transaction(linked);
        let committed = ports.commit().commit();
        assert_eq!(committed.cascaded, [Domain::Documents]);
        ports
            .reload(Domain::Documents)
            .expect("the Documents read back");
        let trimmed = documents(&ports)
            .into_iter()
            .find(|held| held.id == document.id)
            .expect("the Document stays");
        assert!(!trimmed.links.contains(&DocumentLink::Transaction(linked)));
        let reversal = ports.take_reversal().expect("the delete was recorded");

        ports.reverse(&reversal);
        ports.commit().commit();
        ports
            .reload(Domain::Documents)
            .expect("the Documents read back");
        ports
            .reload(Domain::Transactions)
            .expect("the Transactions read back");

        assert_eq!((transactions(&ports), documents(&ports)), before);
    }

    /// An Account that a Budget counts and no Bill Plan pays from, so it can be deleted.
    fn budgeted_account(ports: &crate::LedgerPorts) -> lib_accounts::Account {
        let deletion = ports.account_deletion();
        let stores = ports.stores();
        let accounts = stores.accounts.borrow().accounts().accounts();
        let counted: Vec<_> = budgets(ports)
            .into_iter()
            .flat_map(|budget| budget.account_ids)
            .collect();
        accounts
            .into_iter()
            .find(|account| {
                counted.contains(&account.id)
                    && deletion
                        .deletion
                        .borrow()
                        .bill_plans_paying_from(account.id)
                        == 0
            })
            .expect("a budgeted Account no Bill Plan pays from")
    }

    #[test]
    fn an_account_delete_reloads_its_budgets_and_undoing_it_restores_them() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let ports = crate::ports::test_support::file(dir.path());
        let account = budgeted_account(&ports);
        let before = budgets(&ports);
        let documents_before = documents(&ports);

        lib_account_deletion::interactor::DeleteAccount { id: account.id }
            .run(&mut *ports.account_deletion().deletion.borrow_mut())
            .expect("the Account deletes");
        let committed = ports.commit().commit();
        assert!(
            committed.cascaded.contains(&Domain::Budgets),
            "the file's cascade reaches the Budgets: {committed:?}"
        );
        // The Client reloads each cascaded domain, so memory matches the file.
        for domain in &committed.cascaded {
            ports.reload(*domain).expect("the domain reads back");
        }
        let after = budgets(&ports);
        assert!(
            after
                .iter()
                .all(|budget| !budget.account_ids.contains(&account.id)),
            "no Budget still counts the deleted Account"
        );
        let reversal = ports.take_reversal().expect("the delete was recorded");
        assert!(reversal.domains().contains(&Domain::Budgets));

        // The Accounts record with #1249, so the Account is put back by hand.
        ports
            .stores()
            .accounts
            .borrow_mut()
            .accounts_mut()
            .upsert_account(account);
        ports.reverse(&reversal);
        ports.commit().commit();
        for domain in [
            Domain::Budgets,
            Domain::Documents,
            Domain::Transactions,
            Domain::Bills,
        ] {
            ports.reload(domain).expect("the domain reads back");
        }

        assert_eq!(budgets(&ports), before);
        assert_eq!(documents(&ports), documents_before);
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
