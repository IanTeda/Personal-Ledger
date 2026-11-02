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

use std::path::{Path, PathBuf};

use chrono::NaiveDate;
use lib_core::{AccountId, BillPlanId, DocumentTypeId, ItemId, Money, PayeeId, TransactionId};

use crate::Document;
use crate::types::DocumentTypeRow;

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
