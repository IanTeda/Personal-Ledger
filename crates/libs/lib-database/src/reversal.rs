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
//! ([`crate::LedgerPorts::step`]). Both write through the same stores with the journal paused,
//! the non-recording mark, so stepping through History records nothing. History is linear, so the rows always hold what the entry left them as: there is no
//! conflict check, only [`check_other_side`]'s `debug_assert!` that each row holds the image's
//! other side.
//!
//! Every store records: the Units, the Institutions (whose rows Unit delete reassigns), the
//! Transactions, Bills, Budgets and Documents (#1248), with the Document Types beside the
//! Documents, and the Accounts, Categories, Tags, Payees and Inventory (#1249).
//!
//! Where a delete's SQL cascade removes rows in another store (the Document Links to a deleted
//! Transaction, Account, Payee or Item, and an Account's place in the Budgets that count it), the
//! deleting store's recorder notes those rows' images first, ahead of the deleted row's own, so
//! Undo, which goes newest first, restores the row before the rows that name it. Which deletes
//! reach which domains is [`Deleted::cascades_to`](crate::writer::Deleted::cascades_to), the rule
//! the commit's reload reads too, and the store a cascade reaches says what the delete takes from
//! it (`DocumentsStore::unlinked_by`, `BudgetsStore::uncounted`) (#1276). Memory still holds the
//! rows when the delete is made, since the Client reloads the cascade only after the commit, and
//! a second cascade into the same row builds on the first's image ([`Journal::latest`]). The
//! journal itself only holds images.
//!
//! A Bill Plan, a Bill Schedule entry or a Budget new to the Ledger is recorded like any other
//! add, and Undo removes it through its store's delete, which only Undo calls (#1253): no Bills or
//! Budgets intent deletes one. A Plan's generated entries are written after it, so Undo removes
//! them before the Plan.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use lib_accounts::Account;
use lib_bills::{BillPlan, BillScheduleEntry, EntryId};
use lib_budgets::Budget;
use lib_categories::Category;
use lib_core::{
    AccountId, BillPlanId, BudgetId, CategoryId, DocumentId, DocumentTypeId, InstitutionId, ItemId,
    LocationId, PayeeId, PriceSourceId, RowID, TagId, TransactionId, UnitId,
};
use lib_documents::Document;
use lib_documents::types::DocumentTypeRow;
use lib_institutions::Institution;
use lib_inventory::{Item, Location};
use lib_payees::Payee;
use lib_shell::Step;
use lib_tags::Tag;
use lib_transactions::Transaction;
use lib_units::{PriceSourceRecord, UnitRow, UnitUse};

use crate::Domain;
use crate::adapter::store::SharedStores;

/// One row's image before an input and after it; `None` where the row was absent.
#[derive(Debug, Clone, PartialEq)]
pub struct Image<T> {
    pub before: Option<T>,
    pub after: Option<T>,
}

/// A row an [`Image`] is taken of: its id, the [`RowKey`] the journal merges its images by, and how
/// Undo and Redo read, save and delete it in its store, each written once here.
pub(crate) trait Row: Clone + PartialEq + std::fmt::Debug {
    type Id: Copy;

    /// What [`check_other_side`] calls the row.
    const NAME: &'static str;

    fn id(&self) -> Self::Id;

    fn key(id: Self::Id) -> RowKey;

    /// `image` as the [`RowImage`] of its kind.
    fn image(image: Image<Self>) -> RowImage;

    /// `image`'s image of this kind of row, if it is one.
    fn of(image: &RowImage) -> Option<&Image<Self>>;

    /// The row `id` as its store holds it.
    fn held(stores: &SharedStores, id: Self::Id) -> Option<Self>;

    /// Writes `row` to its store, adding it where absent.
    fn put(stores: &SharedStores, row: Self);

    /// Deletes the row `id` from its store.
    fn remove(stores: &SharedStores, id: Self::Id);

    /// The row as [`check_other_side`] compares it: all of it, unless a side of it may differ
    /// between memory and the file.
    fn checked(self) -> Self {
        self
    }
}

/// What one step writes to a row: what the row holds now, which the step expects, and what it
/// becomes.
struct Write<T: Row> {
    id: T::Id,
    expected: Option<T>,
    target: Option<T>,
}

// Each method bounds `T` itself, since `Row` is crate-private and `Image` public.
impl<T> Image<T> {
    fn key(&self) -> Option<RowKey>
    where
        T: Row,
    {
        let row = self.before.as_ref().or(self.after.as_ref())?;
        Some(T::key(row.id()))
    }

    fn unchanged(&self) -> bool
    where
        T: PartialEq,
    {
        self.before == self.after
    }

    fn merge(&mut self, later: Self) {
        self.after = later.after;
    }

    /// `None` where the row is absent on both sides, which no write makes.
    fn write(&self, step: Step) -> Option<Write<T>>
    where
        T: Row,
    {
        let (expected, target) = match step {
            Step::Undo => (self.after.clone(), self.before.clone()),
            Step::Redo => (self.before.clone(), self.after.clone()),
        };
        let id = target.as_ref().or(expected.as_ref())?.id();
        Some(Write {
            id,
            expected,
            target,
        })
    }

    /// Writes the image's side `step` goes to, checking the row holds the other side first.
    fn apply(&self, step: Step, stores: &SharedStores)
    where
        T: Row,
    {
        let Some(Write {
            id,
            expected,
            target,
        }) = self.write(step)
        else {
            return;
        };
        check_other_side(T::NAME, || {
            (
                T::held(stores, id).map(T::checked),
                expected.map(T::checked),
            )
        });
        match target {
            Some(row) => T::put(stores, row),
            None => T::remove(stores, id),
        }
    }
}

/// A row's [`Row::held`], [`Row::put`] and [`Row::remove`]: through its store's repository, by
/// the store's field, its read and write accessors and the repository's read, upsert and delete,
/// where the access is uniform, or the three written out where it isn't.
macro_rules! row_access {
    (($store:ident, $store_mut:ident, $get:ident, $upsert:ident, $delete:ident)) => {
        fn held(stores: &SharedStores, id: Self::Id) -> Option<Self> {
            stores.$store.borrow().$store().$get(id)
        }

        fn put(stores: &SharedStores, row: Self) {
            stores.$store.borrow_mut().$store_mut().$upsert(row);
        }

        fn remove(stores: &SharedStores, id: Self::Id) {
            stores.$store.borrow_mut().$store_mut().$delete(id);
        }
    };
    ({ $($access:tt)* }) => {
        $($access)*
    };
}

/// Declares [`RowImage`], its [`RowKey`] and each row's [`Row`] from one list, so a new row type
/// adds a line here and an arm to [`RowImage::domain`], and the merging and writing are generic.
macro_rules! row_images {
    ($(
        $(#[$attr:meta])*
        $variant:ident($row:ty) by $id:ty = |$held:ident| $read:expr, $name:literal $access:tt;
    )*) => {
        /// One row's [`Image`] before an input and after it, by the kind of row.
        #[derive(Debug, Clone, PartialEq)]
        pub enum RowImage {
            $($(#[$attr])* $variant(Image<$row>),)*
            /// The base Unit is one value rather than a row, so it is never absent.
            BaseUnit(UnitId, UnitId),
            /// A list in a stored order ([`crate::adapter::ordered`]): every row's place before
            /// and after one was moved or removed. A delete records it before the row's own image,
            /// so Undo, which goes newest first, restores the row before its place.
            Order(Domain, Vec<RowID>, Vec<RowID>),
        }

        /// Which row an image is of, so a second write to it merges into the first.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub(crate) enum RowKey {
            $($variant($id),)*
            BaseUnit,
            Order(Domain),
        }

        $(
            impl Row for $row {
                type Id = $id;

                const NAME: &'static str = $name;

                fn id(&self) -> $id {
                    let $held = self;
                    $read
                }

                fn key(id: $id) -> RowKey {
                    RowKey::$variant(id)
                }

                fn image(image: Image<Self>) -> RowImage {
                    RowImage::$variant(image)
                }

                fn of(image: &RowImage) -> Option<&Image<Self>> {
                    match image {
                        RowImage::$variant(image) => Some(image),
                        _ => None,
                    }
                }

                row_access!($access);
            }
        )*

        impl RowImage {
            fn key(&self) -> Option<RowKey> {
                match self {
                    $(Self::$variant(image) => image.key(),)*
                    Self::BaseUnit(..) => Some(RowKey::BaseUnit),
                    Self::Order(domain, ..) => Some(RowKey::Order(*domain)),
                }
            }

            /// Whether the input left the row as it found it, as an add then a delete does.
            fn unchanged(&self) -> bool {
                match self {
                    $(Self::$variant(image) => image.unchanged(),)*
                    Self::BaseUnit(before, after) => before == after,
                    Self::Order(_, before, after) => before == after,
                }
            }

            /// Keeps this image's `before` and takes `later`'s `after`.
            fn merge(&mut self, later: Self) {
                match (self, later) {
                    $((Self::$variant(image), Self::$variant(later)) => image.merge(later),)*
                    (Self::BaseUnit(_, after), Self::BaseUnit(_, later)) => *after = later,
                    (Self::Order(_, _, after), Self::Order(_, _, later)) => *after = later,
                    (image, later) => debug_assert!(false, "{image:?} merged with {later:?}"),
                }
            }

            /// Writes a by-id row's image the way `step` goes. The base Unit and an order are
            /// [`write`]'s own.
            fn apply_row(&self, step: Step, stores: &SharedStores) {
                match self {
                    $(Self::$variant(image) => image.apply(step, stores),)*
                    Self::BaseUnit(..) | Self::Order(..) => {
                        debug_assert!(false, "{self:?} is not a by-id row");
                    }
                }
            }
        }
    };
}

row_images! {
    // Units and Price Sources are written through the store's ports, lent for one write.
    Unit(UnitRow) by UnitId = |row| row.id, "Unit" {
        fn held(stores: &SharedStores, id: UnitId) -> Option<Self> {
            stores.units.borrow().units().unit(id)
        }

        fn put(stores: &SharedStores, row: Self) {
            stores
                .units
                .borrow_mut()
                .split_write(&UnitUse::default(), |ports| ports.units.upsert_unit(row));
        }

        fn remove(stores: &SharedStores, id: UnitId) {
            stores
                .units
                .borrow_mut()
                .split_write(&UnitUse::default(), |ports| ports.units.delete_unit(id));
        }
    };
    PriceSource(PriceSourceRecord) by PriceSourceId = |row| row.id(), "Price Source" {
        fn held(stores: &SharedStores, id: PriceSourceId) -> Option<Self> {
            stores
                .units
                .borrow()
                .price_sources()
                .price_source_records()
                .into_iter()
                .find(|record| record.id() == id)
        }

        fn put(stores: &SharedStores, row: Self) {
            stores
                .units
                .borrow_mut()
                .split_write(&UnitUse::default(), |ports| {
                    ports.price_sources.upsert_price_source(row);
                });
        }

        fn remove(stores: &SharedStores, id: PriceSourceId) {
            stores
                .units
                .borrow_mut()
                .split_write(&UnitUse::default(), |ports| {
                    ports.price_sources.delete_price_source(id);
                });
        }
    };
    Institution(Institution) by InstitutionId = |row| row.id, "Institution"
        (institutions, institutions_mut, institution, upsert_institution, delete_institution);
    /// A whole Transaction with its Splits.
    Transaction(Transaction) by TransactionId = |row| row.id, "Transaction"
        (transactions, transactions_mut, transaction, upsert_transaction, delete_transaction);
    // Only Undo deletes a Bill Plan, an entry or a Budget, through the store (#1253).
    BillPlan(BillPlan) by BillPlanId = |row| row.id, "Bill Plan" {
        fn held(stores: &SharedStores, id: BillPlanId) -> Option<Self> {
            stores.bills.borrow().plans().bill_plan(id)
        }

        fn put(stores: &SharedStores, row: Self) {
            stores.bills.borrow_mut().plans_mut().upsert_bill_plan(row);
        }

        fn remove(stores: &SharedStores, id: BillPlanId) {
            stores.bills.borrow_mut().delete_bill_plan(id);
        }
    };
    BillEntry(BillScheduleEntry) by EntryId = |row| row.id(), "Bill Schedule entry" {
        fn held(stores: &SharedStores, id: EntryId) -> Option<Self> {
            stores.bills.borrow().entries().entry(id)
        }

        fn put(stores: &SharedStores, row: Self) {
            stores.bills.borrow_mut().entries_mut().upsert_entry(row);
        }

        fn remove(stores: &SharedStores, id: EntryId) {
            stores.bills.borrow_mut().delete_entry(id);
        }
    };
    /// A whole Budget with its Accounts and every chain.
    Budget(Budget) by BudgetId = |row| row.id, "Budget" {
        fn held(stores: &SharedStores, id: BudgetId) -> Option<Self> {
            stores.budgets.borrow().budgets().budget(id)
        }

        fn put(stores: &SharedStores, row: Self) {
            stores.budgets.borrow_mut().budgets_mut().upsert_budget(&row);
        }

        fn remove(stores: &SharedStores, id: BudgetId) {
            stores.budgets.borrow_mut().delete_budget(id);
        }
    };
    /// A whole Document with its Intake and Links.
    Document(Document) by DocumentId = |row| row.id, "Document" {
        fn held(stores: &SharedStores, id: DocumentId) -> Option<Self> {
            stores
                .documents
                .borrow()
                .documents()
                .documents()
                .into_iter()
                .find(|document| document.id == id)
        }

        fn put(stores: &SharedStores, row: Self) {
            stores.documents.borrow_mut().documents_mut().upsert_document(&row);
        }

        fn remove(stores: &SharedStores, id: DocumentId) {
            stores.documents.borrow_mut().documents_mut().delete_document(id);
        }

        /// Without its Links: a cascade changes a Document's Links in the file and not in memory
        /// until the Client reloads, which the in-memory stores never do.
        fn checked(self) -> Self {
            Document {
                links: Vec::new(),
                ..self
            }
        }
    };
    DocumentType(DocumentTypeRow) by DocumentTypeId = |row| row.id, "Document Type" {
        fn held(stores: &SharedStores, id: DocumentTypeId) -> Option<Self> {
            stores
                .documents
                .borrow()
                .documents()
                .document_types()
                .into_iter()
                .find(|row| row.id == id)
        }

        fn put(stores: &SharedStores, row: Self) {
            stores.documents.borrow_mut().documents_mut().upsert_document_type(&row);
        }

        fn remove(stores: &SharedStores, id: DocumentTypeId) {
            stores.documents.borrow_mut().documents_mut().delete_document_type(id);
        }
    };
    Account(Account) by AccountId = |row| row.id, "Account"
        (accounts, accounts_mut, account, upsert_account, delete_account);
    // Only Category delete and Tag remove delete a Category or a Tag, through the store.
    Category(Category) by CategoryId = |row| row.id, "Category" {
        fn held(stores: &SharedStores, id: CategoryId) -> Option<Self> {
            stores.categories.borrow().categories().category(id)
        }

        fn put(stores: &SharedStores, row: Self) {
            stores.categories.borrow_mut().categories_mut().upsert_category(row);
        }

        fn remove(stores: &SharedStores, id: CategoryId) {
            stores.categories.borrow_mut().delete_category(id);
        }
    };
    Tag(Tag) by TagId = |row| row.id, "Tag" {
        fn held(stores: &SharedStores, id: TagId) -> Option<Self> {
            stores.tags.borrow().tags().tag(id)
        }

        fn put(stores: &SharedStores, row: Self) {
            stores.tags.borrow_mut().tags_mut().upsert_tag(row);
        }

        fn remove(stores: &SharedStores, id: TagId) {
            stores.tags.borrow_mut().delete_tag(id);
        }
    };
    Payee(Payee) by PayeeId = |row| row.id, "Payee"
        (payees, payees_mut, payee, upsert_payee, delete_payee);
    /// A whole Location with its cover and Areas.
    Location(Location) by LocationId = |row| row.id, "Location"
        (inventory, inventory_mut, location, upsert_location, delete_location);
    Item(Item) by ItemId = |row| row.id, "Item"
        (inventory, inventory_mut, item, upsert_item, delete_item);
}

impl RowImage {
    /// The store the row is kept in.
    pub fn domain(&self) -> Domain {
        match self {
            Self::Unit(..) | Self::PriceSource(..) | Self::BaseUnit(..) => Domain::Units,
            Self::Order(domain, ..) => *domain,
            Self::Institution(..) => Domain::Institutions,
            Self::Transaction(..) => Domain::Transactions,
            Self::BillPlan(..) | Self::BillEntry(..) => Domain::Bills,
            Self::Budget(..) => Domain::Budgets,
            Self::Document(..) => Domain::Documents,
            Self::DocumentType(..) => Domain::DocumentTypes,
            Self::Account(..) => Domain::Accounts,
            Self::Category(..) => Domain::Categories,
            Self::Tag(..) => Domain::Tags,
            Self::Payee(..) => Domain::Payees,
            Self::Location(..) | Self::Item(..) => Domain::Inventory,
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

/// Writes `reversal`'s images to `stores` the way `step` goes, unrecorded in `journal`: Undo the
/// newest image first, each row back to its `before`, and Redo in the order written, each row to
/// its `after`. Each row is expected to hold the image's other side, which linear History
/// guarantees (`debug_assert!`).
pub(crate) fn apply(
    reversal: &LedgerReversal,
    step: Step,
    stores: &SharedStores,
    journal: &Journal,
) {
    let images: Box<dyn Iterator<Item = &RowImage>> = match step {
        Step::Undo => Box::new(reversal.images.iter().rev()),
        Step::Redo => Box::new(reversal.images.iter()),
    };
    journal.unrecorded(|| {
        for image in images {
            write(image, step, stores);
        }
    });
}

/// Checks the row holds the image's other side before a step writes it, the only conflict check
/// linear History needs (`debug_assert!`). `sides` gives what the row holds and what it is
/// expected to hold, read only in a debug build.
fn check_other_side<T>(row: &str, sides: impl FnOnce() -> (T, T))
where
    T: PartialEq + std::fmt::Debug,
{
    if cfg!(debug_assertions) {
        let (held, expected) = sides();
        debug_assert_eq!(held, expected, "the {row} as History left it");
    }
}

fn write(image: &RowImage, step: Step, stores: &SharedStores) {
    match image {
        RowImage::BaseUnit(before, after) => {
            let (expected, target) = match step {
                Step::Undo => (*after, *before),
                Step::Redo => (*before, *after),
            };
            let unread = UnitUse::default();
            stores.units.borrow_mut().split_write(&unread, |ports| {
                check_other_side("base Unit", || (ports.base_unit.base_unit(), expected));
                ports.base_unit.set_base_unit(target);
            });
        }
        // Not checked: Undo of a delete restores the row before its place, so the list holds
        // the row again, where the store put it, when the order is written back.
        RowImage::Order(domain, before, after) => {
            let target = match step {
                Step::Undo => before,
                Step::Redo => after,
            };
            match stores.ordered_mut(*domain) {
                Some(mut ordered) => ordered.place(target),
                None => debug_assert!(false, "{domain:?} keeps no stored order"),
            }
        }
        row => row.apply_row(step, stores),
    }
}

#[derive(Debug, Default)]
struct JournalState {
    images: Vec<RowImage>,
    /// Where each row's image sits in `images`, so recording and reading one is O(1) however
    /// many rows the input writes (an Import writes one per Transaction).
    index: HashMap<RowKey, usize>,
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
        let state = &mut *state;
        let Some(key) = image.key() else {
            state.images.push(image);
            return;
        };
        match state.index.get(&key) {
            Some(&at) => state.images[at].merge(image),
            None => {
                state.index.insert(key, state.images.len());
                state.images.push(image);
            }
        }
    }

    /// Notes a write of `after` over `before`, the row as its store held it.
    pub(crate) fn record_upsert<T: Row>(&self, before: Option<T>, after: T) {
        self.record(T::image(Image {
            before,
            after: Some(after),
        }));
    }

    /// Notes the delete of `before`, the row as its store held it; nothing where it was absent.
    pub(crate) fn record_delete<T: Row>(&self, before: Option<T>) {
        if let Some(before) = before {
            self.record(T::image(Image {
                before: Some(before),
                after: None,
            }));
        }
    }

    /// The row `id` as the input's writes so far left it: `Some(None)` where they deleted it, and
    /// `None` where the input has not written it. What a cascade builds on, so a second cascade
    /// into the same row keeps the first's change.
    pub(crate) fn latest<T: Row>(&self, id: T::Id) -> Option<Option<T>> {
        let state = self.0.borrow();
        let at = *state.index.get(&T::key(id))?;
        state
            .images
            .get(at)
            .and_then(T::of)
            .map(|image| image.after.clone())
    }

    /// Whether a write now would be recorded: not while Undo or Redo steps.
    pub(crate) fn recording(&self) -> bool {
        !self.0.borrow().paused
    }

    /// The images noted since the last take, leaving none: `None` where the input changed no row.
    pub fn take(&self) -> Option<LedgerReversal> {
        let mut state = self.0.borrow_mut();
        state.index.clear();
        let images: Vec<RowImage> = std::mem::take(&mut state.images)
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
    use lib_documents::DocumentLink;
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

        journal.record(RowImage::Unit(Image {
            before: Some(unit("aud")),
            after: Some(unit("audx")),
        }));
        journal.record(RowImage::BaseUnit(UnitId::SEED_AUD, UnitId::SEED_BTC));
        journal.record(RowImage::Unit(Image {
            before: Some(unit("audx")),
            after: Some(unit("audy")),
        }));

        assert_eq!(
            journal.take().map(|reversal| reversal.images),
            Some(vec![
                RowImage::Unit(Image {
                    before: Some(unit("aud")),
                    after: Some(unit("audy"))
                }),
                RowImage::BaseUnit(UnitId::SEED_AUD, UnitId::SEED_BTC),
            ])
        );
        assert_eq!(journal.take(), None, "taken");
    }

    #[test]
    fn a_row_written_many_times_in_one_input_is_one_image() {
        let journal = Journal::default();
        let codes: Vec<String> = (0..500).map(|step| format!("u{step}")).collect();

        journal.record(RowImage::Unit(Image {
            before: Some(unit("aud")),
            after: Some(unit(&codes[0])),
        }));
        for written in codes.windows(2) {
            journal.record(RowImage::BaseUnit(UnitId::SEED_AUD, UnitId::SEED_BTC));
            journal.record(RowImage::Unit(Image {
                before: Some(unit(&written[0])),
                after: Some(unit(&written[1])),
            }));
        }

        assert_eq!(
            journal.take().map(|reversal| reversal.images),
            Some(vec![
                RowImage::Unit(Image {
                    before: Some(unit("aud")),
                    after: Some(unit("u499")),
                }),
                RowImage::BaseUnit(UnitId::SEED_AUD, UnitId::SEED_BTC),
            ])
        );
    }

    #[test]
    fn a_row_left_as_found_is_no_change() {
        let journal = Journal::default();

        journal.record(RowImage::Unit(Image {
            before: None,
            after: Some(unit("nzd")),
        }));
        journal.record(RowImage::Unit(Image {
            before: Some(unit("nzd")),
            after: None,
        }));

        assert_eq!(journal.take(), None);
    }

    #[test]
    fn nothing_is_recorded_while_unrecorded() {
        let journal = Journal::default();

        journal.unrecorded(|| {
            journal.record(RowImage::Unit(Image {
                before: None,
                after: Some(unit("nzd")),
            }))
        });

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

    fn accounts(ports: &crate::LedgerPorts) -> Vec<Account> {
        ports.stores().accounts.borrow().accounts().accounts()
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
                RowImage::Unit(Image {
                    before: None,
                    after: Some(renamed)
                }),
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

        ports.step(&reversal, Step::Undo);
        assert_eq!(units(&ports), before, "every row back under its own id");
        assert_eq!(ports.take_reversal(), None, "Undo records nothing");

        ports.step(&reversal, Step::Redo);
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

        ports.step(&reversal, Step::Undo);

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

        ports.step(&reversal, Step::Undo);

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
        ports.step(reversal, Step::Undo);
        assert_eq!(&rows(ports), before, "every row back under its own id");
        assert_eq!(ports.take_reversal(), None, "Undo records nothing");
        ports.step(reversal, Step::Redo);
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
        ports.step(&reversal, Step::Undo);
        assert_eq!(sorted(rows(&ports)), sorted(before.clone()));
        ports.step(&reversal, Step::Redo);
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

    /// The Bill Plans and entries, each sorted by id, so memory and a file read afresh compare.
    fn bills(ports: &crate::LedgerPorts) -> (Vec<BillPlan>, Vec<BillScheduleEntry>) {
        let store = ports.stores().bills.clone();
        let store = store.borrow();
        let mut plans = store.plans().bill_plans();
        plans.sort_by_key(|plan| plan.id);
        let mut entries = store.entries().entries();
        entries.sort_by_key(BillScheduleEntry::id);
        (plans, entries)
    }

    /// The sample's first active Monthly Plan as a draft due from `first_due`, under `name`.
    fn monthly_draft(
        ports: &crate::LedgerPorts,
        name: &str,
        first_due: chrono::NaiveDate,
    ) -> (BillPlanId, lib_bills::BillPlanDraft) {
        let plan = bills(ports)
            .0
            .into_iter()
            .find(|plan| plan.is_active && plan.recurrence == lib_bills::Recurrence::Monthly)
            .expect("the sample has an active Monthly Plan");
        let draft = lib_bills::BillPlanDraft {
            name: name.into(),
            category_id: plan.category_id,
            unit: plan.unit,
            account_id: plan.account_id,
            payee_id: plan.payee_id,
            planned_amount: plan.planned_amount,
            amount_kind: plan.amount_kind,
            recurrence: plan.recurrence,
            first_due,
            ends_on: None,
            attention_lead: plan.attention_lead,
        };
        (plan.id, draft)
    }

    /// Undoes and redoes `reversal` on a Ledger file, checking `rows` in memory and read afresh
    /// from `domain`'s tables after each step.
    fn undone_and_redone_through_the_file<T: PartialEq + std::fmt::Debug>(
        ports: &crate::LedgerPorts,
        reversal: &LedgerReversal,
        domain: Domain,
        rows: impl Fn(&crate::LedgerPorts) -> T,
        before: &T,
        after: &T,
    ) {
        ports.step(reversal, Step::Undo);
        ports.commit().commit();
        assert_eq!(&rows(ports), before, "undone in memory");
        ports.reload(domain).expect("the rows read back");
        assert_eq!(&rows(ports), before, "undone in the file");
        assert_eq!(ports.take_reversal(), None, "Undo records nothing");

        ports.step(reversal, Step::Redo);
        ports.commit().commit();
        assert_eq!(&rows(ports), after, "redone in memory");
        ports.reload(domain).expect("the rows read back");
        assert_eq!(
            &rows(ports),
            after,
            "redone in the file, under the same ids"
        );
        assert_eq!(ports.take_reversal(), None, "Redo records nothing");
    }

    #[test]
    fn a_bill_plan_added_is_undone_with_its_schedule_and_redone_under_the_same_ids() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let ports = crate::ports::test_support::file(dir.path());
        let today = crate::ports::test_support::today();
        let (_, draft) = monthly_draft(&ports, "Water", today);
        let before = bills(&ports);

        let id = {
            let shared = ports.bills();
            let mut bills = shared.borrow_mut();
            let write = bills.split_mut();
            lib_bills::interactor::AddBillPlan { draft, today }
                .run(write.bills, write.accounts, write.categories)
                .expect("the draft holds")
        };
        ports.commit().commit();
        let after = bills(&ports);
        let generated = after.1.iter().filter(|entry| entry.plan_id == id).count();
        assert!(generated > 0, "the add generated its Schedule");
        let reversal = ports.take_reversal().expect("the add was recorded");

        assert_eq!(reversal.domains(), [Domain::Bills]);
        undone_and_redone_through_the_file(
            &ports,
            &reversal,
            Domain::Bills,
            bills,
            &before,
            &after,
        );
    }

    #[test]
    fn a_budget_added_is_undone_and_redone_under_its_own_id() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let ports = crate::ports::test_support::file(dir.path());
        let before = budgets(&ports);
        let budget = Budget {
            id: BudgetId::new(),
            name: "Holidays".into(),
            is_default: false,
            ..before[0].clone()
        };

        ports
            .stores()
            .budgets
            .borrow_mut()
            .budgets_mut()
            .upsert_budget(&budget);
        ports.commit().commit();
        let after = budgets(&ports);
        assert!(after.contains(&budget));
        let reversal = ports.take_reversal().expect("the add was recorded");

        assert_eq!(reversal.domains(), [Domain::Budgets]);
        undone_and_redone_through_the_file(
            &ports,
            &reversal,
            Domain::Budgets,
            budgets,
            &before,
            &after,
        );
    }

    #[test]
    fn undoing_a_bill_plan_edit_removes_the_entries_it_generated() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let ports = crate::ports::test_support::file(dir.path());
        let today = crate::ports::test_support::today();
        let (id, mut draft) = monthly_draft(&ports, "Rescheduled", today);
        draft.recurrence = lib_bills::Recurrence::Weekly;
        draft.first_due = today + chrono::Days::new(3);
        let before = bills(&ports);
        let dues = |rows: &(Vec<BillPlan>, Vec<BillScheduleEntry>)| -> Vec<chrono::NaiveDate> {
            rows.1
                .iter()
                .filter(|entry| entry.plan_id == id)
                .map(|entry| entry.due)
                .collect()
        };

        {
            let shared = ports.bills();
            let mut bills = shared.borrow_mut();
            let write = bills.split_mut();
            lib_bills::interactor::EditBillPlan {
                id,
                draft,
                is_active: true,
                today,
            }
            .run(write.bills, write.accounts, write.categories)
            .expect("the edit holds");
        }
        ports.commit().commit();
        let after = bills(&ports);
        let new_dates: Vec<chrono::NaiveDate> = dues(&after)
            .into_iter()
            .filter(|due| !dues(&before).contains(due))
            .collect();
        assert!(
            !new_dates.is_empty(),
            "the edit generated new-dated entries"
        );
        let reversal = ports.take_reversal().expect("the edit was recorded");

        undone_and_redone_through_the_file(
            &ports,
            &reversal,
            Domain::Bills,
            bills,
            &before,
            &after,
        );
        ports.step(&reversal, Step::Undo);
        ports.commit().commit();
        ports.reload(Domain::Bills).expect("the rows read back");
        assert!(
            dues(&bills(&ports))
                .iter()
                .all(|due| !new_dates.contains(due)),
            "no generated entry left"
        );
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

    fn type_ids(ports: &crate::LedgerPorts) -> Vec<DocumentTypeId> {
        let store = ports.stores().documents.clone();
        let store = store.borrow();
        store
            .documents()
            .document_types()
            .iter()
            .map(|row| row.id)
            .collect()
    }

    #[test]
    fn adding_a_document_type_records_only_its_row_and_undo_removes_it() {
        let ports = ports();
        let before = type_ids(&ports);
        let mut added = ports
            .stores()
            .documents
            .borrow()
            .documents()
            .document_types()[0]
            .clone();
        added.id = DocumentTypeId::new();
        added.is_default = false;
        added.name = "Warranty".into();

        ports
            .stores()
            .documents
            .borrow_mut()
            .documents_mut()
            .upsert_document_type(&added);
        let reversal = ports.take_reversal().expect("the add was recorded");

        assert_eq!(
            reversal.images(),
            [RowImage::DocumentType(Image {
                before: None,
                after: Some(added.clone())
            })],
            "an add appends, so it records no order"
        );
        ports.step(&reversal, Step::Undo);
        assert_eq!(type_ids(&ports), before);
    }

    #[test]
    fn a_document_type_move_is_undone_to_its_order_and_redone_through_the_file() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let ports = crate::ports::test_support::file(dir.path());
        let before = type_ids(&ports);
        assert!(before.len() > 2, "the Ledger has types to reorder");
        let moved = before[0];

        ports
            .stores()
            .documents
            .borrow_mut()
            .documents_mut()
            .move_document_type(moved, 2);
        ports.commit().commit();
        let after = type_ids(&ports);
        assert_eq!(after[2], moved);
        let reversal = ports.take_reversal().expect("the move was recorded");
        assert_eq!(
            reversal.images(),
            [RowImage::Order(
                Domain::DocumentTypes,
                before.iter().map(DocumentTypeId::row_id).collect(),
                after.iter().map(DocumentTypeId::row_id).collect(),
            )]
        );

        ports.step(&reversal, Step::Undo);
        ports.commit().commit();
        assert_eq!(type_ids(&ports), before);
        ports
            .reload(Domain::DocumentTypes)
            .expect("the types read back");
        assert_eq!(
            type_ids(&ports),
            before,
            "in its order after reloading the file"
        );
        assert_eq!(ports.take_reversal(), None, "Undo records nothing");

        ports.step(&reversal, Step::Redo);
        ports.commit().commit();
        ports
            .reload(Domain::DocumentTypes)
            .expect("the types read back");
        assert_eq!(type_ids(&ports), after, "moved again");
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
            RowImage::Document(Image {
                before: Some(before),
                after: Some(after),
            }),
            RowImage::Transaction(Image {
                before: Some(_),
                after: None,
            }),
            RowImage::Transaction(Image {
                before: Some(_),
                after: None,
            }),
        ] = reversal.images()
        else {
            panic!("the Document's cascade first, then each Transaction: {reversal:?}");
        };
        assert_eq!(before, &document);
        assert!(after.links.is_empty(), "both links go with their records");

        ports.step(&reversal, Step::Undo);
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

        ports.step(&reversal, Step::Undo);
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
    fn budgeted_account(ports: &crate::LedgerPorts) -> Account {
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

        ports.step(&reversal, Step::Undo);
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

    /// Writes through `write`, then checks Undo and Redo bring `rows` back and forward.
    fn written_undone_and_redone<T: PartialEq + std::fmt::Debug>(
        ports: &crate::LedgerPorts,
        domain: Domain,
        rows: impl Fn(&crate::LedgerPorts) -> T,
        write: impl FnOnce(&crate::LedgerPorts),
    ) {
        let before = rows(ports);
        write(ports);
        let after = rows(ports);
        assert_ne!(before, after, "the write changed the {domain:?}");
        let reversal = ports.take_reversal().expect("the write was recorded");
        // A delete also notes the Document Links it cascades into.
        assert!(reversal.domains().contains(&domain), "{reversal:?}");
        undone_and_redone(ports, &reversal, rows, &before, &after);
    }

    fn sorted<T, K: Ord>(mut rows: Vec<T>, key: impl Fn(&T) -> K) -> Vec<T> {
        rows.sort_by_key(key);
        rows
    }

    #[test]
    fn an_account_added_edited_and_deleted_is_each_undone_and_redone() {
        let rows = |ports: &crate::LedgerPorts| sorted(accounts(ports), |row| row.id);
        let ports = ports();
        let original = rows(&ports)[0].clone();
        let mut added = original.clone();
        added.id = AccountId::new();
        let added_id = added.id;
        let mut edited = original.clone();
        edited.name = "Renamed".into();
        let stores = |ports: &crate::LedgerPorts| ports.stores().accounts.clone();

        written_undone_and_redone(&ports, Domain::Accounts, rows, |ports| {
            stores(ports)
                .borrow_mut()
                .accounts_mut()
                .upsert_account(added.clone());
        });
        written_undone_and_redone(&ports, Domain::Accounts, rows, |ports| {
            stores(ports)
                .borrow_mut()
                .accounts_mut()
                .upsert_account(edited);
        });
        // The added Account, which no Budget counts: the memory fakes don't cascade a delete into
        // the Budgets, and the file-backed test below covers the Account that one counts.
        written_undone_and_redone(&ports, Domain::Accounts, rows, |ports| {
            stores(ports)
                .borrow_mut()
                .accounts_mut()
                .delete_account(added_id);
        });
    }

    #[test]
    fn a_category_added_edited_and_deleted_is_each_undone_and_redone() {
        let rows = |ports: &crate::LedgerPorts| {
            sorted(
                ports.stores().categories.borrow().categories().categories(),
                |row| row.id,
            )
        };
        let ports = ports();
        let original = rows(&ports)[0].clone();
        let mut added = original.clone();
        added.id = CategoryId::new();
        let mut edited = original.clone();
        edited.name = "Renamed".into();
        let stores = |ports: &crate::LedgerPorts| ports.stores().categories.clone();

        written_undone_and_redone(&ports, Domain::Categories, rows, |ports| {
            stores(ports)
                .borrow_mut()
                .categories_mut()
                .upsert_category(added);
        });
        written_undone_and_redone(&ports, Domain::Categories, rows, |ports| {
            stores(ports)
                .borrow_mut()
                .categories_mut()
                .upsert_category(edited);
        });
        written_undone_and_redone(&ports, Domain::Categories, rows, |ports| {
            stores(ports).borrow_mut().delete_category(original.id);
        });
    }

    #[test]
    fn a_tag_added_edited_and_deleted_is_each_undone_and_redone() {
        let rows = |ports: &crate::LedgerPorts| {
            sorted(ports.stores().tags.borrow().tags().tags(), |row| row.id)
        };
        let ports = ports();
        let original = rows(&ports)[0].clone();
        let mut added = original.clone();
        added.id = TagId::new();
        let mut edited = original.clone();
        edited.name = "Renamed".into();
        let stores = |ports: &crate::LedgerPorts| ports.stores().tags.clone();

        written_undone_and_redone(&ports, Domain::Tags, rows, |ports| {
            stores(ports).borrow_mut().tags_mut().upsert_tag(added);
        });
        written_undone_and_redone(&ports, Domain::Tags, rows, |ports| {
            stores(ports).borrow_mut().tags_mut().upsert_tag(edited);
        });
        written_undone_and_redone(&ports, Domain::Tags, rows, |ports| {
            stores(ports).borrow_mut().delete_tag(original.id);
        });
    }

    #[test]
    fn a_payee_added_edited_and_deleted_is_each_undone_and_redone() {
        let rows = |ports: &crate::LedgerPorts| {
            sorted(ports.stores().payees.borrow().payees().payees(), |row| {
                row.id
            })
        };
        let ports = ports();
        let original = rows(&ports)[0].clone();
        let mut added = original.clone();
        added.id = PayeeId::new();
        let mut edited = original.clone();
        edited.name = "Renamed".into();
        let stores = |ports: &crate::LedgerPorts| ports.stores().payees.clone();

        written_undone_and_redone(&ports, Domain::Payees, rows, |ports| {
            stores(ports).borrow_mut().payees_mut().upsert_payee(added);
        });
        written_undone_and_redone(&ports, Domain::Payees, rows, |ports| {
            stores(ports).borrow_mut().payees_mut().upsert_payee(edited);
        });
        written_undone_and_redone(&ports, Domain::Payees, rows, |ports| {
            stores(ports)
                .borrow_mut()
                .payees_mut()
                .delete_payee(original.id);
        });
    }

    #[test]
    fn an_institution_added_edited_and_deleted_is_each_undone_and_redone() {
        let rows = |ports: &crate::LedgerPorts| sorted(institutions(ports), |row| row.id);
        let ports = ports();
        let original = rows(&ports)[0].clone();
        let mut added = original.clone();
        added.id = InstitutionId::new();
        let mut edited = original.clone();
        edited.name = "Renamed".into();
        let stores = |ports: &crate::LedgerPorts| ports.stores().institutions.clone();

        written_undone_and_redone(&ports, Domain::Institutions, rows, |ports| {
            stores(ports)
                .borrow_mut()
                .institutions_mut()
                .upsert_institution(added);
        });
        written_undone_and_redone(&ports, Domain::Institutions, rows, |ports| {
            stores(ports)
                .borrow_mut()
                .institutions_mut()
                .upsert_institution(edited);
        });
        written_undone_and_redone(&ports, Domain::Institutions, rows, |ports| {
            stores(ports)
                .borrow_mut()
                .institutions_mut()
                .delete_institution(original.id);
        });
    }

    #[test]
    fn an_inventory_location_and_item_added_edited_and_deleted_are_undone_and_redone() {
        let rows = |ports: &crate::LedgerPorts| {
            let store = ports.stores().inventory.borrow();
            let inventory = store.inventory();
            (
                sorted(inventory.locations(), |row| row.id),
                sorted(inventory.items(), |row| row.id),
            )
        };
        let ports = ports();
        let location = rows(&ports).0[0].clone();
        let item = rows(&ports)
            .1
            .into_iter()
            .find(|item| item.location == location.id)
            .expect("the first Location holds an Item");
        let mut added = item.clone();
        added.id = ItemId::new();
        let mut edited = item.clone();
        edited.name = "Renamed".into();
        let stores = |ports: &crate::LedgerPorts| ports.stores().inventory.clone();

        written_undone_and_redone(&ports, Domain::Inventory, rows, |ports| {
            stores(ports)
                .borrow_mut()
                .inventory_mut()
                .upsert_item(added);
        });
        written_undone_and_redone(&ports, Domain::Inventory, rows, |ports| {
            stores(ports)
                .borrow_mut()
                .inventory_mut()
                .upsert_item(edited);
        });
        written_undone_and_redone(&ports, Domain::Inventory, rows, |ports| {
            stores(ports)
                .borrow_mut()
                .inventory_mut()
                .delete_item(item.id);
        });
        let mut renamed = location.clone();
        renamed.name = "Renamed".into();
        written_undone_and_redone(&ports, Domain::Inventory, rows, |ports| {
            stores(ports)
                .borrow_mut()
                .inventory_mut()
                .upsert_location(renamed);
        });
        written_undone_and_redone(&ports, Domain::Inventory, rows, |ports| {
            stores(ports)
                .borrow_mut()
                .inventory_mut()
                .delete_location(location.id);
        });
    }

    #[test]
    fn undoing_a_category_delete_restores_its_splits_plans_payee_defaults_and_limits() {
        let ports = ports();
        let rows = |ports: &crate::LedgerPorts| {
            let stores = ports.stores();
            (
                sorted(
                    stores.categories.borrow().categories().categories(),
                    |row| row.id,
                ),
                transactions(ports),
                sorted(stores.bills.borrow().plans().bill_plans(), |row| row.id),
                sorted(stores.payees.borrow().payees().payees(), |row| row.id),
                budgets(ports),
            )
        };
        let before = rows(&ports);
        let ids: Vec<CategoryId> = before.0.iter().map(|category| category.id).collect();
        let deleted = ids
            .into_iter()
            .find_map(|id| {
                let outcome = lib_category_deletion::interactor::DeleteCategory { id }
                    .run(&mut *ports.category_deletion().deletion.borrow_mut());
                outcome
                    .ok()
                    .filter(|done| done.splits_moved > 0 && done.payee_defaults_cleared > 0)
            })
            .expect("the sample has a Category with Splits and a Payee default");
        assert!(deleted.splits_moved > 0);
        let after = rows(&ports);
        let reversal = ports.take_reversal().expect("the delete was recorded");
        assert_eq!(
            reversal.domains().first(),
            Some(&Domain::Categories),
            "the home and the moved rows are written before the delete"
        );

        ports.step(&reversal, Step::Undo);
        assert_eq!(rows(&ports), before);
        ports.step(&reversal, Step::Redo);
        assert_eq!(rows(&ports), after);
    }

    #[test]
    fn undoing_a_tag_merge_restores_both_tags_and_their_splits() {
        let ports = ports();
        let rows = |ports: &crate::LedgerPorts| {
            (
                sorted(ports.stores().tags.borrow().tags().tags(), |row| row.id),
                transactions(ports),
            )
        };
        let before = rows(&ports);
        let ids: Vec<TagId> = before.0.iter().map(|tag| tag.id).collect();
        let mut merged = false;
        'pairs: for source in &ids {
            for target in &ids {
                if source == target {
                    continue;
                }
                let outcome = lib_tag_removal::interactor::MergeTags {
                    source: *source,
                    target: *target,
                }
                .run(&mut *ports.tag_removal().removal.borrow_mut());
                if outcome.is_ok() && rows(&ports).1 != before.1 {
                    merged = true;
                    break 'pairs;
                }
                // A merge that moved no Split is undone by hand so the next pair starts clean.
                if let Some(reversal) = ports.take_reversal() {
                    ports.step(&reversal, Step::Undo);
                }
            }
        }
        assert!(merged, "a merge moved a Split");
        let after = rows(&ports);
        assert_eq!(after.0.len(), before.0.len() - 1);
        let reversal = ports.take_reversal().expect("the merge was recorded");

        ports.step(&reversal, Step::Undo);
        assert_eq!(rows(&ports), before);
        ports.step(&reversal, Step::Redo);
        assert_eq!(rows(&ports), after);
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
        ports.step(&added, Step::Undo);
        ports.commit().commit();
        ports.step(&added, Step::Redo);
        ports.commit().commit();

        ports.reload(Domain::Units).expect("the Units read back");

        assert!(units(&ports).contains(&nzd), "redone under its own id");
    }

    /// Adds three rows with `add`, deletes the middle one with `delete`, and checks Undo puts it
    /// back at its index, in the memory and in the file read afresh, and Redo removes it again.
    fn a_delete_undone_and_redone_through_the_file<T: PartialEq + std::fmt::Debug + Copy>(
        domain: Domain,
        ids: impl Fn(&crate::LedgerPorts) -> Vec<T>,
        add: impl Fn(&crate::LedgerPorts) -> [T; 3],
        delete: impl Fn(&crate::LedgerPorts, T),
    ) {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let ports = crate::ports::test_support::file(dir.path());
        let [_, middle, _] = add(&ports);
        ports.commit().commit();
        let _ = ports.take_reversal();
        let with = ids(&ports);
        let index = with.iter().position(|id| *id == middle).expect("held");

        delete(&ports, middle);
        ports.commit().commit();
        let reversal = ports.take_reversal().expect("the delete was recorded");
        let without = ids(&ports);
        assert_eq!(without.len() + 1, with.len());

        ports.step(&reversal, Step::Undo);
        ports.commit().commit();
        assert_eq!(ids(&ports).iter().position(|id| *id == middle), Some(index));
        ports.reload(domain).expect("the rows read back");
        assert_eq!(ids(&ports), with, "in its place after reloading the file");

        ports.step(&reversal, Step::Redo);
        ports.commit().commit();
        ports.reload(domain).expect("the rows read back");
        assert_eq!(ids(&ports), without, "removed again");
    }

    #[test]
    fn an_institution_delete_is_undone_to_its_place_and_redone() {
        a_delete_undone_and_redone_through_the_file(
            Domain::Institutions,
            |ports| institutions(ports).iter().map(|row| row.id).collect(),
            |ports| {
                let ids = [
                    InstitutionId::new(),
                    InstitutionId::new(),
                    InstitutionId::new(),
                ];
                let mut row = institutions(ports)[0].clone();
                for (n, id) in ids.iter().enumerate() {
                    row.id = *id;
                    row.name = format!("Placed {n}");
                    row.usual_unit = None;
                    ports
                        .stores()
                        .institutions
                        .borrow_mut()
                        .institutions_mut()
                        .upsert_institution(row.clone());
                }
                ids
            },
            |ports, id| {
                ports
                    .stores()
                    .institutions
                    .borrow_mut()
                    .institutions_mut()
                    .delete_institution(id);
            },
        );
    }

    #[test]
    fn an_account_delete_is_undone_to_its_place_and_redone() {
        a_delete_undone_and_redone_through_the_file(
            Domain::Accounts,
            |ports| accounts(ports).iter().map(|row| row.id).collect(),
            |ports| {
                let ids = [AccountId::new(), AccountId::new(), AccountId::new()];
                let mut row = accounts(ports)[0].clone();
                for (n, id) in ids.iter().enumerate() {
                    row.id = *id;
                    row.name = format!("Placed {n}");
                    ports
                        .stores()
                        .accounts
                        .borrow_mut()
                        .accounts_mut()
                        .upsert_account(row.clone());
                }
                ids
            },
            |ports, id| {
                ports
                    .stores()
                    .accounts
                    .borrow_mut()
                    .accounts_mut()
                    .delete_account(id);
            },
        );
    }
}
