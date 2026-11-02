//! In-memory ports for tests: [`InMemoryDocuments`], the Documents repository over plain lists,
//! [`InMemoryFiles`], a file system of the files it is given, and [`LinkRecords`] answering every
//! read port from its lists.

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use lib_core::DocumentTypeId;

use crate::protocols::{
    DocumentAccount, DocumentAccounts, DocumentBillPlan, DocumentBillPlans, DocumentFiles,
    DocumentItem, DocumentItems, DocumentPayee, DocumentPayees, DocumentRepository,
    DocumentTransaction, DocumentTransactions, LinkRecords, move_type_to, resolve_typed,
};
use crate::types::DocumentTypeRow;
use crate::{Document, DocumentLink};

/// The Documents repository over plain lists: the Documents in the order they were added, the
/// Document Types in the user's order.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct InMemoryDocuments {
    pub documents: Vec<Document>,
    pub types: Vec<DocumentTypeRow>,
}

impl InMemoryDocuments {
    /// What the file's `ON DELETE CASCADE` does when the records `gone` names are deleted: every
    /// Link to one goes, and the Documents stay (GLOSSARY Document Link).
    pub fn cascade_delete(&mut self, gone: &[DocumentLink]) {
        for document in &mut self.documents {
            document.links.retain(|link| !gone.contains(link));
        }
    }
}

impl DocumentRepository for InMemoryDocuments {
    fn documents(&self) -> Vec<Document> {
        self.documents.clone()
    }

    fn document_types(&self) -> Vec<DocumentTypeRow> {
        self.types.clone()
    }

    fn upsert_document(&mut self, document: &Document) {
        match self.documents.iter_mut().find(|row| row.id == document.id) {
            Some(row) => *row = document.clone(),
            None => self.documents.push(document.clone()),
        }
    }

    fn upsert_document_type(&mut self, row: &DocumentTypeRow) {
        match self.types.iter_mut().find(|held| held.id == row.id) {
            Some(held) => *held = row.clone(),
            None => self.types.push(row.clone()),
        }
    }

    fn delete_document_type(&mut self, id: DocumentTypeId) {
        self.types.retain(|row| row.id != id);
    }

    fn move_document_type(&mut self, id: DocumentTypeId, to: usize) {
        move_type_to(&mut self.types, id, to);
    }
}

/// A file system holding only the files it is given, with a fixed home and working directory.
#[derive(Debug, Clone)]
pub struct InMemoryFiles {
    pub home: PathBuf,
    pub working_dir: PathBuf,
    /// Each file on "disk", with its size in bytes.
    pub files: HashMap<PathBuf, u64>,
}

impl Default for InMemoryFiles {
    fn default() -> Self {
        Self {
            home: PathBuf::from("/home/user"),
            working_dir: PathBuf::from("/work"),
            files: HashMap::new(),
        }
    }
}

impl InMemoryFiles {
    /// Adds a file of `bytes` at `path`.
    pub fn with(mut self, path: impl Into<PathBuf>, bytes: u64) -> Self {
        self.files.insert(path.into(), bytes);
        self
    }
}

impl DocumentFiles for InMemoryFiles {
    fn resolve(&self, typed: &str) -> PathBuf {
        resolve_typed(typed, Some(&self.home), Some(&self.working_dir))
    }

    fn exists(&self, path: &Path) -> bool {
        self.files.contains_key(path)
    }

    fn size(&self, path: &Path) -> Option<u64> {
        self.files.get(path).copied()
    }
}

/// The records as the read ports answer them, each port handing back its list.
impl DocumentTransactions for LinkRecords {
    fn document_transactions(&self) -> Vec<DocumentTransaction> {
        self.transactions.clone()
    }
}

impl DocumentPayees for LinkRecords {
    fn document_payees(&self) -> Vec<DocumentPayee> {
        self.payees.clone()
    }
}

impl DocumentAccounts for LinkRecords {
    fn document_accounts(&self) -> Vec<DocumentAccount> {
        self.accounts.clone()
    }
}

impl DocumentBillPlans for LinkRecords {
    fn document_bill_plans(&self) -> Vec<DocumentBillPlan> {
        self.plans.clone()
    }
}

impl DocumentItems for LinkRecords {
    fn document_items(&self) -> Vec<DocumentItem> {
        self.items.clone()
    }
}
