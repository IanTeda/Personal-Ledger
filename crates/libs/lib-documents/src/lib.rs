//! Pure Documents model, rules and stub seed (`docs/ux/desktop-mockups/04-documents/`): `gpui`-free and
//! side-effect free, so every rule is unit-tested without a window. The other domains are read
//! through [`protocols`]' read ports as `lib-documents`' own rows, and the file system through
//! [`DocumentFiles`], so nothing here reads `lib_database` or the disk.
//!
//! The rules are the Desktop Documents Surface map's decision tickets (#431–#433, #435):
//!
//! - There is one entity, the [`Document`], Unfiled (the Inbox) or Filed. Only the user files a
//!   Document, never gaining a Link, so a Filed Document with no Links is valid.
//! - The Financial Year is always derived from the document date ([`Document::financial_year`]),
//!   never stored.
//! - A Key Date [`band`]s as Need Review inside [`KEY_DATE_ATTENTION_DAYS`] either side of today,
//!   both ends inclusive; further ahead it is Upcoming, further back Stale. The reminder flag
//!   changes neither.
//! - The Suggested Link is derived on every call ([`suggestion`]) and never stored: only
//!   Transactions within [`CANDIDATE_WINDOW_DAYS`] of the Document's date are candidates, each must
//!   agree on amount or payee, and the date Signal allows [`DATE_SIGNAL_DAYS`].
//! - Skip is session-only, held by the Client as a set of ids and never written, and undo is one
//!   level: [`undo`] reverses the last [`FilingUndo`] whole.

#[cfg(feature = "contract-tests")]
#[expect(
    clippy::expect_used,
    reason = "a contract check is test code a consumer runs, and fails by panicking as a test does"
)]
pub mod contract;
mod entity;
pub mod fake;
pub mod interactor;
pub mod keymap;
pub mod msg;
pub mod presenter;
pub mod protocols;
pub mod types;

pub use crate::protocols::{
    DocumentAccount, DocumentAccounts, DocumentBillPlan, DocumentBillPlans, DocumentFiles,
    DocumentItem, DocumentItems, DocumentPayee, DocumentPayees, DocumentReads, DocumentRepository,
    DocumentTransaction, DocumentTransactions, DocumentTypesDialog, DocumentTypesOutcome,
    DocumentTypesRouter, DocumentsDialog, DocumentsOutcome, DocumentsRouter, LinkRecords,
    SharedFiles, SharedReads, SharedRepository, TypeChange,
};
pub use entity::*;
