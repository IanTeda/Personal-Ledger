//! The ports the Documents domain reads the other domains and the file system through, each
//! returning `lib-documents`' own rows, so the crate depends on no other feature crate and never
//! touches the disk. A row carries what the rules and the Views read of the record a Link names;
//! each row's text is filled by its owning domain. `fake` implements each in memory.
//!
//! The rules read the records together, through [`LinkRecords`], read once per query.
//!
//! [`DocumentRepository`] is where the Documents and Document Types themselves are kept,
//! synchronous as `lib-budgets`' is (ADR-0046): an adapter serves reads from memory and queues each
//! write, so a write never fails here; a failed save is the adapter's to report. `lib-database`'s
//! Documents adapter implements it (#963).

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use chrono::NaiveDate;
use lib_core::{
    AccountId, BillPlanId, DocumentId, DocumentTypeId, ItemId, Money, PayeeId, TransactionId,
};
use lib_toast::Notify;

use crate::types::DocumentTypeRow;
use crate::{Document, DocumentLink, DocumentType, FilingUndo, LibraryScope};

/// The Documents and Document Types on the Ledger. A Document is never deleted (#959), so there is
/// no Document delete.
pub trait DocumentRepository {
    /// Every Document, in the order it was added.
    fn documents(&self) -> Vec<Document>;

    /// Every Document Type, in the user's order.
    fn document_types(&self) -> Vec<DocumentTypeRow>;

    /// Writes `document` whole, replacing the Document with its id or adding it: its row, its
    /// Intake and its Links, so a Link it no longer holds is gone.
    fn upsert_document(&mut self, document: &Document);

    /// Writes Document Type `row`, replacing the type with its id where it stands in the order, or
    /// adding it at the end.
    fn upsert_document_type(&mut self, row: &DocumentTypeRow);

    /// Removes Document Type `id`. The Documents of that type move to another in the same write,
    /// as Remove does, or storage refuses it.
    fn delete_document_type(&mut self, id: DocumentTypeId);

    /// Moves Document Type `id` to place `to` in the order (the end, past it), the others keeping
    /// theirs around it.
    fn move_document_type(&mut self, id: DocumentTypeId, to: usize);
}

/// The repository as the Document Types page and its Dialogs hold it: one, shared, so a type a
/// Dialog adds is on the page as soon as the Dialog closes. Single-threaded, as both Clients' UI
/// threads are.
pub type SharedRepository = Rc<RefCell<dyn DocumentRepository>>;

/// A Document Types Dialog, as the page asks the Router to open it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentTypesDialog {
    Add,
    Edit(DocumentTypeId),
    Remove(DocumentTypeId),
    /// What Other, the Default, gets in place of Remove.
    DefaultNotice,
}

/// What a Dialog did to the Document Types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeChange {
    /// Added or edited.
    Saved,
    /// Removed, its Filed Documents moved.
    Removed,
}

/// What a confirmed Document Types Dialog did, handed to the page when the Dialog closes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DocumentTypesOutcome {
    pub change: TypeChange,
    /// The type the page selects: the one saved, or after a removal the one now where it stood.
    pub select: Option<DocumentTypeId>,
}

/// Navigation out of the Document Types Presenters, implemented by each Client. The Client passes
/// it into each `handle` call rather than the Presenter holding it, since the Desktop can only
/// open a Dialog with the gpui context that call runs in. A status-line message goes to the Shell
/// through [`Notify`] (ADR-0048).
pub trait DocumentTypesRouter: Notify {
    /// Builds `dialog`'s Presenter on the shared repository and hosts its View. Returns whether it
    /// opened: a Dialog on a type removed since it was asked for has no Presenter to build.
    fn open_dialog(&mut self, dialog: DocumentTypesDialog) -> bool;

    /// Closes the open Document Types Dialog. `outcome` is what its confirm did, `None` when it was
    /// cancelled; the Client hands it to the page as `DocumentTypesPageEvent::DialogClosed`.
    fn close_dialog(&mut self, outcome: Option<DocumentTypesOutcome>);
}

/// The other domains as the Documents page reads them, shared with the Client that answers them.
pub type SharedReads = Rc<dyn DocumentReads>;

/// The file system as the Documents page reaches it, shared with the Client that implements it.
pub type SharedFiles = Rc<dyn DocumentFiles>;

/// A Documents Dialog, as the page asks the Router to open it. The Dialogs aren't Presenters yet
/// (#968): the Client builds each from what is named here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentsDialog {
    /// `+ Add`, its Document Type defaulting to the Library's `scope`.
    Add(LibraryScope),
    /// Edit Filed Document `id`.
    Edit(DocumentId),
    /// `Import…`.
    Import,
    /// Edit Unfiled Document `id`'s Extracted Facts.
    Facts(DocumentId),
    /// The count-first confirm of Accept all strong matches, with how many it files.
    AcceptAll(usize),
    /// The link picker on Filed Document `id`, toggling a Link.
    Link(DocumentId),
    /// The link picker on Unfiled Document `id`, filing it by hand.
    File(DocumentId),
    /// The link picker over Filed Document `id`'s Links, following one.
    Follow(DocumentId),
}

/// What a confirmed Documents Dialog did, handed to the page when it closes.
#[derive(Debug, Clone, PartialEq)]
pub enum DocumentsOutcome {
    /// Added or saved Document `id`: the Library shows it.
    Shown(DocumentId),
    /// Imported into the Inbox, which shows.
    Imported,
    /// Saved Document `id`'s Extracted Facts: its skip is cleared and it keeps focus.
    FactsSaved(DocumentId),
    /// Accept all filed `batch`, the action `u` reverses.
    AcceptedAll(FilingUndo),
    /// The picker chose a Document Type and at most one Link to file Document `id` under.
    FileByHand {
        id: DocumentId,
        doc_type: DocumentType,
        link: Option<DocumentLink>,
    },
    /// The picker chose a Link to follow.
    Follow(DocumentLink),
}

/// Navigation out of the Documents page's Presenter, implemented by each Client and passed into
/// each `handle` call. A status-line message or a Toast goes to the Shell through [`Notify`]
/// (ADR-0048).
pub trait DocumentsRouter: Notify {
    /// Shows the record `link` names, selected, in the View that owns it: a Transaction, an
    /// Account, a Payee, a Bill Plan or an Inventory Item.
    fn reveal(&mut self, link: DocumentLink);

    /// Opens `dialog`. Returns whether it opened.
    fn open_dialog(&mut self, dialog: DocumentsDialog) -> bool;

    /// Hands the file at `path`, which exists, to the platform: opened in its application, or
    /// shown in its folder when `in_folder`.
    fn open_file(&mut self, path: &Path, in_folder: bool);
}

/// [`DocumentRepository::move_document_type`] over a list: moves `id` to `to`, clamped to the end.
/// Returns whether it was there to move.
pub fn move_type_to(types: &mut Vec<DocumentTypeRow>, id: DocumentTypeId, to: usize) -> bool {
    let Some(from) = types.iter().position(|row| row.id == id) else {
        return false;
    };
    let row = types.remove(from);
    types.insert(to.min(types.len()), row);
    true
}

/// A Transaction as Documents reads it: a candidate for the Suggested Link, a Link's label and a
/// row of the link picker.
#[derive(Debug, Clone, PartialEq)]
pub struct DocumentTransaction {
    pub id: TransactionId,
    pub date: NaiveDate,
    /// The Transaction's whole total, in the Ledger's sign: expenses are negative.
    pub amount: Money,
    /// The Account the Transaction is posted against.
    pub account_id: AccountId,
    /// The Payee each Split names, in Split order: `None` for a Split without one.
    pub payee_ids: Vec<Option<PayeeId>>,
    pub memo: Option<String>,
}

impl DocumentTransaction {
    /// The first Payee any Split names.
    pub fn first_payee(&self) -> Option<PayeeId> {
        self.payee_ids.iter().find_map(|payee| *payee)
    }

    /// Whether any Split names `payee`.
    pub fn names_payee(&self, payee: PayeeId) -> bool {
        self.payee_ids.contains(&Some(payee))
    }
}

/// A Payee as Documents reads it: the merchant on an Unfiled Document's facts is matched against
/// its name and aliases.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentPayee {
    pub id: PayeeId,
    pub name: String,
    /// The match text: each alias, upper case, as the Payees store it.
    pub aliases: Vec<String>,
    pub is_active: bool,
}

/// An Account as Documents reads it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentAccount {
    pub id: AccountId,
    pub name: String,
    /// Its Institution's name, empty when it has none.
    pub institution: String,
}

/// A Bill Plan as Documents reads it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentBillPlan {
    pub id: BillPlanId,
    pub name: String,
    /// The Recurrence as the Bills show it ("Monthly"), filled by the side that answers
    /// [`DocumentBillPlans`].
    pub cadence: String,
    pub payee_id: Option<PayeeId>,
}

/// An Inventory Item as Documents reads it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentItem {
    pub id: ItemId,
    pub name: String,
}

/// The Transactions a Document can link to.
pub trait DocumentTransactions {
    /// Every Transaction, in the Transactions' order.
    fn document_transactions(&self) -> Vec<DocumentTransaction>;
}

/// The Payees a Document can link to and a merchant is matched against.
pub trait DocumentPayees {
    /// Every Payee, active or not.
    fn document_payees(&self) -> Vec<DocumentPayee>;
}

/// The Accounts a Document can link to.
pub trait DocumentAccounts {
    fn document_accounts(&self) -> Vec<DocumentAccount>;
}

/// The Bill Plans a Document can link to.
pub trait DocumentBillPlans {
    fn document_bill_plans(&self) -> Vec<DocumentBillPlan>;
}

/// The Inventory Items a Document can link to.
pub trait DocumentItems {
    fn document_items(&self) -> Vec<DocumentItem>;
}

/// Every Documents read port, as the one value a Client reads the records through.
pub trait DocumentReads:
    DocumentTransactions + DocumentPayees + DocumentAccounts + DocumentBillPlans + DocumentItems
{
}

impl<T> DocumentReads for T where
    T: DocumentTransactions
        + DocumentPayees
        + DocumentAccounts
        + DocumentBillPlans
        + DocumentItems
        + ?Sized
{
}

/// Every record a Document can link to, each port read once.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct LinkRecords {
    pub transactions: Vec<DocumentTransaction>,
    pub payees: Vec<DocumentPayee>,
    pub accounts: Vec<DocumentAccount>,
    pub plans: Vec<DocumentBillPlan>,
    pub items: Vec<DocumentItem>,
}

impl LinkRecords {
    pub fn read(reads: &dyn DocumentReads) -> Self {
        Self {
            transactions: reads.document_transactions(),
            payees: reads.document_payees(),
            accounts: reads.document_accounts(),
            plans: reads.document_bill_plans(),
            items: reads.document_items(),
        }
    }

    pub fn transaction(&self, id: TransactionId) -> Option<&DocumentTransaction> {
        self.transactions.iter().find(|row| row.id == id)
    }

    pub fn payee(&self, id: PayeeId) -> Option<&DocumentPayee> {
        self.payees.iter().find(|row| row.id == id)
    }

    pub fn account(&self, id: AccountId) -> Option<&DocumentAccount> {
        self.accounts.iter().find(|row| row.id == id)
    }

    pub fn plan(&self, id: BillPlanId) -> Option<&DocumentBillPlan> {
        self.plans.iter().find(|row| row.id == id)
    }

    pub fn item(&self, id: ItemId) -> Option<&DocumentItem> {
        self.items.iter().find(|row| row.id == id)
    }
}

/// The file system as Documents reads it: where a typed path points, and what is there. A port,
/// so the rules deciding what Add and Import refuse are tested without a disk.
pub trait DocumentFiles {
    /// A path as the user typed it, made absolute: `~` is the home directory and a relative path
    /// is taken from the working directory. Never canonicalised, so it never touches the disk.
    fn resolve(&self, typed: &str) -> PathBuf;

    /// Whether a file is at `path`.
    fn exists(&self, path: &Path) -> bool;

    /// The file's size in bytes, or `None` when it can't be read.
    fn size(&self, path: &Path) -> Option<u64>;
}

/// [`DocumentFiles::resolve`]'s rule, for each implementation to share: `home` and `working_dir`
/// are `None` when the platform can't say.
pub fn resolve_typed(typed: &str, home: Option<&Path>, working_dir: Option<&Path>) -> PathBuf {
    let typed = typed.trim();
    if let Some(home) = home {
        if let Some(rest) = typed.strip_prefix("~/") {
            return home.join(rest);
        }
        if typed == "~" {
            return home.to_path_buf();
        }
    }
    let path = PathBuf::from(typed);
    match working_dir {
        Some(dir) if !path.is_absolute() => dir.join(path),
        _ => path,
    }
}
