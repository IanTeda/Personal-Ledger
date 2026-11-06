//! The Documents adapter: `lib-documents`' `DocumentRepository` over the `document_types`,
//! `documents` and `document_links` tables (ADR-0046, #963).
//!
//! It loads every Document Type, in the user's order, and every Document with its Intake and Links
//! into memory, blocking, and serves them from there. A write changes memory at once and joins the
//! [`PendingWrites`] every adapter shares, which the bin commits after each input. Writing a
//! Document writes it whole: its row, then its Links replaced with the ones it holds, so a Link it
//! dropped is gone. A type's place in the order is its `position`, rewritten for every type
//! whenever one moves or goes, so it always matches memory.
//!
//! Deleting a record a Link names cascades to the Link in the file, and the commit that deleted it
//! asks the Client to [`reload`](DocumentsAdapter::reload) the Documents, queued behind it on the
//! writer, so memory drops the Link too (ADR-0046's 2026-11-02 amendment, #964). Should a Document
//! still holding such a Link be written, the Link whose record the file no longer has is skipped
//! rather than failing on its foreign key. A fact's Document Type is written the same way: null
//! once its type has gone.

use chrono::NaiveDate;
use lib_core::{
    AccountId, BillPlanId, DocumentId, DocumentTypeId, ItemId, Money, PayeeId, TransactionId,
};
use lib_documents::protocols::move_type_to;
use lib_documents::types::{DocumentTypeRow, RemindLead, TracksDate};
use lib_documents::{
    Document, DocumentLink, DocumentRepository, DocumentType, ExtractedFacts, FileKind, Filing,
    Intake, KeyDate, KeyDateKind, Source,
};
use sqlx::SqliteConnection;

use crate::commit::PendingWrites;
use crate::writer::{BoxFuture, Change, Domain, Writer};
use crate::{Error, Result};

/// One port write, saved in the order it was made.
#[derive(Debug, Clone)]
enum DocumentsChange {
    UpsertDocument(Box<Document>),
    /// A Document and its Links, in one write.
    DeleteDocument(DocumentId),
    /// A type's row, with its place in the order should it be new.
    UpsertType(DocumentTypeRow, usize),
    DeleteType(DocumentTypeId),
    /// Every type's place, after one moved or went.
    Order(Vec<DocumentTypeId>),
}

/// The Documents domain, held in memory and saved through the shared writer.
pub struct DocumentsAdapter {
    documents: Vec<Document>,
    types: Vec<DocumentTypeRow>,
    pending: PendingWrites,
}

impl DocumentsAdapter {
    /// Loads every Document Type and Document through `pending`'s writer, blocking until they are
    /// read. The adapter's writes join `pending`. An empty database loads no rows.
    ///
    /// # Errors
    /// Returns the error of the read, or of a row naming a Document that isn't stored or holding a
    /// value no Document has.
    #[tracing::instrument(name = "Load the Documents", level = "debug", skip_all, err)]
    pub fn load(pending: PendingWrites) -> Result<Self> {
        let (types, documents) = pending.writer().run(|pool| async move {
            let mut conn = pool.acquire().await?;
            let types = read_types(&mut conn).await?;
            let documents = read_documents(&mut conn).await?;
            Ok((types, documents))
        })?;
        Ok(Self {
            documents,
            types,
            pending,
        })
    }

    /// Writes `types`, in order, and `documents` whole in one transaction, blocking until it
    /// lands. Each is an upsert, so seeding again leaves the same rows. For after the
    /// Transactions, Accounts, Payees, Bills and Inventory the Links name.
    ///
    /// # Errors
    /// Returns the error of the write; nothing is written then.
    #[tracing::instrument(name = "Seed the Documents", level = "debug", skip_all, err)]
    pub fn seed(
        writer: &Writer,
        types: Vec<DocumentTypeRow>,
        documents: Vec<Document>,
    ) -> Result<()> {
        writer.run(move |pool| async move {
            let mut conn = pool.begin().await?;
            Self::seed_in(&mut conn, &types, &documents).await?;
            conn.commit().await?;
            Ok(())
        })
    }

    /// [`Self::seed`]'s writes on `conn`, so a seed of several domains commits them in one
    /// transaction (#886).
    ///
    /// # Errors
    /// Returns the error of a write.
    pub(crate) async fn seed_in(
        conn: &mut SqliteConnection,
        types: &[DocumentTypeRow],
        documents: &[Document],
    ) -> Result<()> {
        for (position, row) in types.iter().enumerate() {
            upsert_type(conn, row, position).await?;
        }
        for document in documents {
            upsert_document(conn, document).await?;
        }
        Ok(())
    }

    /// Whether the file holds no Document Type, blocking: one [`crate::open_storage`] refuses.
    ///
    /// # Errors
    /// Returns the error of the read.
    pub(crate) fn has_no_types(writer: &Writer) -> Result<bool> {
        writer.run(|pool| async move {
            let count =
                sqlx::query_scalar!(r#"SELECT COUNT(*) AS "count!: i64" FROM document_types"#)
                    .fetch_one(&pool)
                    .await?;
            Ok(count == 0)
        })
    }

    /// Drops the Documents changes not yet committed and loads the rows again, blocking. For after
    /// a [`WriteFailure`](crate::writer::WriteFailure) names [`Domain::Documents`], or a commit's
    /// cascade does, so memory matches the file again.
    ///
    /// # Errors
    /// Returns the error of the read; memory is left as it was then.
    #[tracing::instrument(name = "Reload the Documents", level = "debug", skip_all, err)]
    pub fn reload(&mut self) -> Result<()> {
        let fresh = Self::load(self.pending.clone())?;
        self.documents = fresh.documents;
        self.types = fresh.types;
        self.pending.discard(Domain::Documents);
        Ok(())
    }

    fn push_order(&mut self) {
        let order = self.types.iter().map(|row| row.id).collect();
        self.pending.push(DocumentsChange::Order(order));
    }
}

impl DocumentRepository for DocumentsAdapter {
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
        self.pending
            .push(DocumentsChange::UpsertDocument(Box::new(document.clone())));
    }

    fn delete_document(&mut self, id: DocumentId) {
        self.documents.retain(|row| row.id != id);
        self.pending.push(DocumentsChange::DeleteDocument(id));
    }

    fn upsert_document_type(&mut self, row: &DocumentTypeRow) {
        let position = match self.types.iter().position(|held| held.id == row.id) {
            Some(at) => {
                self.types[at] = row.clone();
                at
            }
            None => {
                self.types.push(row.clone());
                self.types.len() - 1
            }
        };
        self.pending
            .push(DocumentsChange::UpsertType(row.clone(), position));
    }

    fn delete_document_type(&mut self, id: DocumentTypeId) {
        self.types.retain(|row| row.id != id);
        self.pending.push(DocumentsChange::DeleteType(id));
        self.push_order();
    }

    fn move_document_type(&mut self, id: DocumentTypeId, to: usize) {
        if move_type_to(&mut self.types, id, to) {
            self.push_order();
        }
    }
}

impl Change for DocumentsChange {
    fn domain(&self) -> Domain {
        Domain::Documents
    }

    fn apply<'c>(self: Box<Self>, conn: &'c mut SqliteConnection) -> BoxFuture<'c, Result<()>> {
        Box::pin(async move {
            match *self {
                Self::UpsertDocument(document) => upsert_document(conn, &document).await,
                Self::DeleteDocument(id) => delete_document(conn, id).await,
                Self::UpsertType(row, position) => upsert_type(conn, &row, position).await,
                Self::DeleteType(id) => {
                    sqlx::query!("DELETE FROM document_types WHERE id = ?", id)
                        .execute(&mut *conn)
                        .await?;
                    Ok(())
                }
                Self::Order(order) => {
                    for (position, id) in order.into_iter().enumerate() {
                        let position = i64::try_from(position).unwrap_or(i64::MAX);
                        sqlx::query!(
                            "UPDATE document_types SET position = ? WHERE id = ?",
                            position,
                            id,
                        )
                        .execute(&mut *conn)
                        .await?;
                    }
                    Ok(())
                }
            }
        })
    }
}

fn tracks_text(tracks: TracksDate) -> &'static str {
    match tracks {
        TracksDate::Renews => "renews",
        TracksDate::Ends => "ends",
        TracksDate::Expires => "expires",
        TracksDate::Revalue => "revalue",
    }
}

fn tracks_of(text: &str) -> Result<TracksDate> {
    match text {
        "renews" => Ok(TracksDate::Renews),
        "ends" => Ok(TracksDate::Ends),
        "expires" => Ok(TracksDate::Expires),
        "revalue" => Ok(TracksDate::Revalue),
        other => Err(Error::Validation(format!("unknown Tracks date {other:?}"))),
    }
}

/// A Key Date's kind, held in the same words as a type's Tracks date.
fn key_kind_text(kind: KeyDateKind) -> &'static str {
    match kind {
        KeyDateKind::Renews => "renews",
        KeyDateKind::Ends => "ends",
        KeyDateKind::Expires => "expires",
        KeyDateKind::Revalue => "revalue",
    }
}

fn key_kind_of(text: &str) -> Result<KeyDateKind> {
    Ok(match tracks_of(text)? {
        TracksDate::Renews => KeyDateKind::Renews,
        TracksDate::Ends => KeyDateKind::Ends,
        TracksDate::Expires => KeyDateKind::Expires,
        TracksDate::Revalue => KeyDateKind::Revalue,
    })
}

/// A Remind lead time as its unit and count columns.
fn remind_columns(remind: Option<RemindLead>) -> (Option<&'static str>, Option<i64>) {
    match remind {
        Some(RemindLead::Days(days)) => (Some("days"), Some(i64::from(days))),
        Some(RemindLead::Months(months)) => (Some("months"), Some(i64::from(months))),
        None => (None, None),
    }
}

fn remind_of(unit: Option<&str>, count: Option<i64>) -> Result<Option<RemindLead>> {
    let lead = |count: i64| {
        u32::try_from(count)
            .map_err(|_| Error::Validation(format!("a Remind lead time of {count}")))
    };
    match (unit, count) {
        (None, None) => Ok(None),
        (Some("days"), Some(days)) => Ok(Some(RemindLead::Days(lead(days)?))),
        (Some("months"), Some(months)) => Ok(Some(RemindLead::Months(lead(months)?))),
        (unit, count) => Err(Error::Validation(format!(
            "a Remind lead time of {count:?} {unit:?}"
        ))),
    }
}

fn kind_text(kind: FileKind) -> &'static str {
    match kind {
        FileKind::Pdf => "pdf",
        FileKind::Jpg => "jpg",
        FileKind::Png => "png",
    }
}

fn kind_of(text: &str) -> Result<FileKind> {
    match text {
        "pdf" => Ok(FileKind::Pdf),
        "jpg" => Ok(FileKind::Jpg),
        "png" => Ok(FileKind::Png),
        other => Err(Error::Validation(format!("unknown file kind {other:?}"))),
    }
}

fn filing_text(filing: Filing) -> &'static str {
    match filing {
        Filing::Unfiled => "unfiled",
        Filing::Filed => "filed",
    }
}

fn filing_of(text: &str) -> Result<Filing> {
    match text {
        "unfiled" => Ok(Filing::Unfiled),
        "filed" => Ok(Filing::Filed),
        other => Err(Error::Validation(format!("unknown filing {other:?}"))),
    }
}

fn source_text(source: Source) -> &'static str {
    match source {
        Source::Scanned => "scanned",
        Source::Emailed => "emailed",
        Source::Downloads => "downloads",
        Source::WatchedFolder => "watched_folder",
        Source::Dropped => "dropped",
        Source::Import => "import",
    }
}

fn source_of(text: &str) -> Result<Source> {
    match text {
        "scanned" => Ok(Source::Scanned),
        "emailed" => Ok(Source::Emailed),
        "downloads" => Ok(Source::Downloads),
        "watched_folder" => Ok(Source::WatchedFolder),
        "dropped" => Ok(Source::Dropped),
        "import" => Ok(Source::Import),
        other => Err(Error::Validation(format!("unknown source {other:?}"))),
    }
}

/// Writes the type's row, at `position` should it be new; an existing type keeps its place, which
/// only [`DocumentsChange::Order`] rewrites.
async fn upsert_type(
    conn: &mut SqliteConnection,
    row: &DocumentTypeRow,
    position: usize,
) -> Result<()> {
    let position = i64::try_from(position).unwrap_or(i64::MAX);
    let tracks_date = row.tracks_date.map(tracks_text);
    let (remind_unit, remind_count) = remind_columns(row.remind);
    let files = i64::from(row.files);
    sqlx::query!(
        "INSERT INTO document_types
             (id, name, position, tracks_date, remind_unit, remind_count, financial_year, files,
              is_default)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT (id) DO UPDATE SET
             name = excluded.name,
             tracks_date = excluded.tracks_date,
             remind_unit = excluded.remind_unit,
             remind_count = excluded.remind_count,
             financial_year = excluded.financial_year,
             files = excluded.files,
             is_default = excluded.is_default",
        row.id,
        row.name,
        position,
        tracks_date,
        remind_unit,
        remind_count,
        row.financial_year,
        files,
        row.is_default,
    )
    .execute(&mut *conn)
    .await?;
    Ok(())
}

/// Writes the Document's row with its Intake, then replaces its Links with the ones it holds, in
/// order. A Link whose record, or a fact's type, the file no longer has is skipped (see the module
/// docs).
async fn upsert_document(conn: &mut SqliteConnection, document: &Document) -> Result<()> {
    let path = document.path.to_string_lossy().into_owned();
    let kind = kind_text(document.kind);
    let pages = i64::from(document.pages);
    let bytes = i64::try_from(document.bytes)
        .map_err(|_| Error::Validation(format!("a file of {} bytes", document.bytes)))?;
    let key_kind = document.key_date.map(|key| key_kind_text(key.kind));
    let key_date = document.key_date.map(|key| key.date);
    let key_reminder = document.key_date.map(|key| key.reminder);
    let filing = filing_text(document.filing);
    let intake = document.intake.as_ref();
    let source = intake.map(|intake| source_text(intake.source));
    let received_on = intake.map(|intake| intake.received_at);
    let facts = intake.map(|intake| &intake.facts);
    let fact_merchant = facts.and_then(|facts| facts.merchant.as_deref());
    let fact_date = facts.and_then(|facts| facts.date);
    let fact_total = facts.and_then(|facts| facts.total.as_ref());
    let fact_type = facts.and_then(|facts| facts.doc_type).map(|kind| kind.0);
    sqlx::query!(
        "INSERT INTO documents
             (id, path, kind, pages, bytes, title, document_type_id, date, key_date_kind, key_date,
              key_date_reminder, extracted_text, filing, source, received_on, fact_merchant,
              fact_date, fact_total, fact_document_type_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18,
                 (SELECT id FROM document_types WHERE id = ?19))
         ON CONFLICT (id) DO UPDATE SET
             path = excluded.path,
             kind = excluded.kind,
             pages = excluded.pages,
             bytes = excluded.bytes,
             title = excluded.title,
             document_type_id = excluded.document_type_id,
             date = excluded.date,
             key_date_kind = excluded.key_date_kind,
             key_date = excluded.key_date,
             key_date_reminder = excluded.key_date_reminder,
             extracted_text = excluded.extracted_text,
             filing = excluded.filing,
             source = excluded.source,
             received_on = excluded.received_on,
             fact_merchant = excluded.fact_merchant,
             fact_date = excluded.fact_date,
             fact_total = excluded.fact_total,
             fact_document_type_id = excluded.fact_document_type_id",
        document.id,
        path,
        kind,
        pages,
        bytes,
        document.title,
        document.doc_type.0,
        document.date,
        key_kind,
        key_date,
        key_reminder,
        document.extracted_text,
        filing,
        source,
        received_on,
        fact_merchant,
        fact_date,
        fact_total,
        fact_type,
    )
    .execute(&mut *conn)
    .await?;
    sqlx::query!(
        "DELETE FROM document_links WHERE document_id = ?",
        document.id
    )
    .execute(&mut *conn)
    .await?;
    for link in &document.links {
        insert_link(conn, document.id, *link).await?;
    }
    Ok(())
}

/// Deletes a Document and its Links, which the one change makes a single write. The file on disk
/// and the records the Links named are not this adapter's to touch.
async fn delete_document(conn: &mut SqliteConnection, id: DocumentId) -> Result<()> {
    sqlx::query!("DELETE FROM document_links WHERE document_id = ?", id)
        .execute(&mut *conn)
        .await?;
    sqlx::query!("DELETE FROM documents WHERE id = ?", id)
        .execute(&mut *conn)
        .await?;
    Ok(())
}

/// Writes one Link, unless the file no longer has its record.
async fn insert_link(
    conn: &mut SqliteConnection,
    id: DocumentId,
    link: DocumentLink,
) -> Result<()> {
    match link {
        DocumentLink::Transaction(target) => {
            sqlx::query!(
                "INSERT INTO document_links (document_id, transaction_id)
             SELECT ?1, ?2 WHERE EXISTS (SELECT 1 FROM transactions WHERE id = ?2)",
                id,
                target,
            )
            .execute(&mut *conn)
            .await?
        }
        DocumentLink::Account(target) => {
            sqlx::query!(
                "INSERT INTO document_links (document_id, account_id)
             SELECT ?1, ?2 WHERE EXISTS (SELECT 1 FROM accounts WHERE id = ?2)",
                id,
                target,
            )
            .execute(&mut *conn)
            .await?
        }
        DocumentLink::Payee(target) => {
            sqlx::query!(
                "INSERT INTO document_links (document_id, payee_id)
             SELECT ?1, ?2 WHERE EXISTS (SELECT 1 FROM payees WHERE id = ?2)",
                id,
                target,
            )
            .execute(&mut *conn)
            .await?
        }
        DocumentLink::BillPlan(target) => {
            sqlx::query!(
                "INSERT INTO document_links (document_id, bill_plan_id)
             SELECT ?1, ?2 WHERE EXISTS (SELECT 1 FROM bill_plans WHERE id = ?2)",
                id,
                target,
            )
            .execute(&mut *conn)
            .await?
        }
        DocumentLink::InventoryItem(target) => {
            sqlx::query!(
                "INSERT INTO document_links (document_id, item_id)
             SELECT ?1, ?2 WHERE EXISTS (SELECT 1 FROM items WHERE id = ?2)",
                id,
                target,
            )
            .execute(&mut *conn)
            .await?
        }
    };
    Ok(())
}

async fn read_types(conn: &mut SqliteConnection) -> Result<Vec<DocumentTypeRow>> {
    let mut types = Vec::new();
    for row in sqlx::query!(
        r#"SELECT
               id             AS "id!: DocumentTypeId",
               name,
               tracks_date,
               remind_unit,
               remind_count,
               financial_year AS "financial_year!: bool",
               files          AS "files!: i64",
               is_default     AS "is_default!: bool"
           FROM document_types
           ORDER BY position, rowid"#
    )
    .fetch_all(&mut *conn)
    .await?
    {
        types.push(DocumentTypeRow {
            id: row.id,
            name: row.name,
            tracks_date: row.tracks_date.as_deref().map(tracks_of).transpose()?,
            remind: remind_of(row.remind_unit.as_deref(), row.remind_count)?,
            financial_year: row.financial_year,
            files: u32::try_from(row.files)
                .map_err(|_| Error::Validation(format!("a type of {} files", row.files)))?,
            is_default: row.is_default,
        });
    }
    Ok(types)
}

async fn read_documents(conn: &mut SqliteConnection) -> Result<Vec<Document>> {
    let mut documents = Vec::new();
    for row in sqlx::query!(
        r#"SELECT
               id                    AS "id!: DocumentId",
               path,
               kind,
               pages                 AS "pages!: i64",
               bytes                 AS "bytes!: i64",
               title,
               document_type_id      AS "document_type_id!: DocumentTypeId",
               date                  AS "date!: NaiveDate",
               key_date_kind,
               key_date              AS "key_date?: NaiveDate",
               key_date_reminder     AS "key_date_reminder?: bool",
               extracted_text,
               filing,
               source,
               received_on           AS "received_on?: NaiveDate",
               fact_merchant,
               fact_date             AS "fact_date?: NaiveDate",
               fact_total            AS "fact_total?: Money",
               fact_document_type_id AS "fact_document_type_id?: DocumentTypeId"
           FROM documents
           ORDER BY rowid"#
    )
    .fetch_all(&mut *conn)
    .await?
    {
        let key_date = match (
            row.key_date_kind.as_deref(),
            row.key_date,
            row.key_date_reminder,
        ) {
            (Some(kind), Some(date), Some(reminder)) => Some(KeyDate {
                kind: key_kind_of(kind)?,
                date,
                reminder,
            }),
            (None, None, None) => None,
            (kind, date, _) => {
                return Err(Error::Validation(format!(
                    "a Key Date of {kind:?} on {date:?}"
                )));
            }
        };
        let intake = match (row.source.as_deref(), row.received_on) {
            (Some(source), Some(received_at)) => Some(Intake {
                source: source_of(source)?,
                received_at,
                facts: ExtractedFacts {
                    merchant: row.fact_merchant,
                    date: row.fact_date,
                    total: row.fact_total,
                    doc_type: row.fact_document_type_id.map(DocumentType),
                },
            }),
            (None, None) => None,
            (source, received) => {
                return Err(Error::Validation(format!(
                    "an Intake from {source:?} received {received:?}"
                )));
            }
        };
        documents.push(Document {
            id: row.id,
            path: row.path.into(),
            kind: kind_of(&row.kind)?,
            pages: u32::try_from(row.pages)
                .map_err(|_| Error::Validation(format!("a file of {} pages", row.pages)))?,
            bytes: u64::try_from(row.bytes)
                .map_err(|_| Error::Validation(format!("a file of {} bytes", row.bytes)))?,
            title: row.title,
            doc_type: DocumentType(row.document_type_id),
            date: row.date,
            key_date,
            links: Vec::new(),
            extracted_text: row.extracted_text,
            filing: filing_of(&row.filing)?,
            intake,
        });
    }
    for link in sqlx::query!(
        r#"SELECT
               document_id    AS "document_id!: DocumentId",
               transaction_id AS "transaction_id?: TransactionId",
               account_id     AS "account_id?: AccountId",
               payee_id       AS "payee_id?: PayeeId",
               bill_plan_id   AS "bill_plan_id?: BillPlanId",
               item_id        AS "item_id?: ItemId"
           FROM document_links
           ORDER BY rowid"#
    )
    .fetch_all(&mut *conn)
    .await?
    {
        let target = match (
            link.transaction_id,
            link.account_id,
            link.payee_id,
            link.bill_plan_id,
            link.item_id,
        ) {
            (Some(id), None, None, None, None) => DocumentLink::Transaction(id),
            (None, Some(id), None, None, None) => DocumentLink::Account(id),
            (None, None, Some(id), None, None) => DocumentLink::Payee(id),
            (None, None, None, Some(id), None) => DocumentLink::BillPlan(id),
            (None, None, None, None, Some(id)) => DocumentLink::InventoryItem(id),
            _ => {
                return Err(Error::Validation(format!(
                    "a Link of Document {} names other than one record",
                    link.document_id
                )));
            }
        };
        let document = documents
            .iter_mut()
            .find(|document| document.id == link.document_id)
            .ok_or_else(|| {
                Error::Validation(format!(
                    "a Link names Document {}, which isn't stored",
                    link.document_id
                ))
            })?;
        document.links.push(target);
    }
    Ok(documents)
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;

    use lib_documents::LinkRecords;
    use lib_documents::contract::{self, PortsFactory};
    use lib_documents::types::default_types;
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

    use super::*;
    use crate::TransactionsStore as _;
    use crate::commit::Commit;
    use crate::writer::WriteFailure;

    /// The adapter on one test database, with its pending batch. The stub's every other domain is
    /// stored first, which a Link names. A reload commits, loads afresh, and fails the check if the
    /// writer reported a failure, so a write SQLite refuses can't pass unseen.
    struct AdapterFactory {
        writer: Writer,
        pending: PendingWrites,
        failures: mpsc::Receiver<WriteFailure>,
    }

    impl AdapterFactory {
        fn new(options: SqliteConnectOptions) -> Self {
            let (writer, failures) = Writer::spawn(options).expect("the test database opens");
            crate::test_support::store_sample_but_documents(&writer, contract::today());
            Self {
                pending: PendingWrites::new(writer.clone()),
                writer,
                failures,
            }
        }

        /// Settles the writes sent so far and returns the failures reported since the last call.
        fn failures(&self) -> Vec<WriteFailure> {
            self.writer.flush().expect("the writer is running");
            self.failures.try_iter().collect()
        }

        /// Runs `sql`, returning whether the file took it.
        fn execute(&self, sql: &'static str) -> bool {
            self.writer
                .run(move |pool| async move {
                    sqlx::query(sql).execute(&pool).await?;
                    Ok(())
                })
                .is_ok()
        }
    }

    impl PortsFactory for AdapterFactory {
        type Ports = DocumentsAdapter;

        /// The stored stub's records, read as the stores answer them.
        fn records(&self) -> LinkRecords {
            crate::test_support::stored_link_records(&self.pending)
        }

        fn create(
            &mut self,
            types: Vec<DocumentTypeRow>,
            documents: Vec<Document>,
        ) -> DocumentsAdapter {
            DocumentsAdapter::seed(&self.writer, types, documents).expect("the seed is written");
            DocumentsAdapter::load(self.pending.clone()).expect("the seed loads")
        }

        fn reload(&mut self, _ports: DocumentsAdapter) -> DocumentsAdapter {
            self.pending.commit();
            let fresh = DocumentsAdapter::load(self.pending.clone()).expect("the rows load");
            if let Some(failure) = self.failures().first() {
                panic!("a write failed: {failure:?}");
            }
            fresh
        }

        fn commit(&mut self, _ports: &mut DocumentsAdapter) {
            self.pending.commit();
        }

        fn break_writes(&mut self, _ports: &mut DocumentsAdapter) {
            assert!(self.execute(
                "CREATE TRIGGER refuse_documents BEFORE INSERT ON documents
                 BEGIN SELECT RAISE(ABORT, 'writes are refused'); END"
            ));
        }

        fn recover(&mut self, mut ports: DocumentsAdapter) -> (DocumentsAdapter, usize) {
            let failures = self
                .failures()
                .iter()
                .filter(|failure| failure.domains == [Domain::Documents])
                .count();
            ports.reload().expect("the rows load");
            (ports, failures)
        }

        /// The stub's last Transaction and last Item, which only Links name, and a new Account and
        /// Payee, since the stub's are each named by a Transaction or a Bill Plan.
        fn deletable(&mut self) -> Vec<DocumentLink> {
            let records = self.records();
            let account = AccountId::new();
            let payee = PayeeId::new();
            self.writer
                .run(move |pool| async move {
                    sqlx::query(
                        "INSERT INTO accounts (id, name, institution_id, account_type, unit_id, balance, opened_at)
                         VALUES (?, 'Spare account', ?, 'bank', ?, '0', '2026-01-01')",
                    )
                    .bind(account)
                    .bind(lib_core::InstitutionId::SEED_PERSONAL)
                    .bind(lib_core::UnitId::SEED_AUD)
                    .execute(&pool)
                    .await?;
                    sqlx::query("INSERT INTO payees (id, name) VALUES (?, 'Spare payee')")
                        .bind(payee)
                        .execute(&pool)
                        .await?;
                    Ok(())
                })
                .expect("the spare records are written");
            vec![
                DocumentLink::Transaction(records.transactions.last().expect("a Transaction").id),
                DocumentLink::Account(account),
                DocumentLink::Payee(payee),
                DocumentLink::InventoryItem(records.items.last().expect("an Item").id),
            ]
        }

        /// Deletes through the record's own adapter, so the commit's report of the cascade is
        /// what the reload follows.
        fn delete_record(
            &mut self,
            mut ports: DocumentsAdapter,
            record: DocumentLink,
        ) -> DocumentsAdapter {
            let pending = self.pending.clone();
            match record {
                DocumentLink::Transaction(id) => crate::TransactionsAdapter::load(pending)
                    .expect("the Transactions load")
                    .transactions_mut()
                    .delete_transaction(id),
                DocumentLink::Account(id) => crate::AccountsAdapter::load(pending)
                    .expect("the Accounts load")
                    .accounts_mut()
                    .delete_account(id),
                DocumentLink::Payee(id) => crate::PayeesAdapter::load(pending)
                    .expect("the Payees load")
                    .payees_mut()
                    .delete_payee(id),
                DocumentLink::InventoryItem(id) => crate::InventoryAdapter::load(pending)
                    .expect("the Inventory loads")
                    .inventory_mut()
                    .delete_item(id),
                DocumentLink::BillPlan(_) => panic!("a Bill Plan has no delete"),
            }
            let cascades = self.pending.commit();
            assert_eq!(
                cascades,
                [Domain::Documents],
                "the commit asks for the reload"
            );
            if let Some(failure) = self.failures().first() {
                panic!("the delete failed: {failure:?}");
            }
            ports.reload().expect("the rows load");
            ports
        }
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn a_document_with_links_and_intake_reloads_the_same(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        contract::a_document_with_links_and_intake_reloads_the_same(&mut AdapterFactory::new(
            options,
        ));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn the_document_types_keep_their_order(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        contract::the_document_types_keep_their_order(&mut AdapterFactory::new(options));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn a_type_removal_moves_its_documents_in_one_batch(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        contract::a_type_removal_moves_its_documents_in_one_batch(&mut AdapterFactory::new(
            options,
        ));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn deleting_a_linked_record_drops_its_links_and_keeps_the_document(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        contract::deleting_a_linked_record_drops_its_links_and_keeps_the_document(
            &mut AdapterFactory::new(options),
        );
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn deleting_a_document_removes_it_and_its_links_and_keeps_the_records(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        contract::deleting_a_document_removes_it_and_its_links_and_keeps_the_records(
            &mut AdapterFactory::new(options),
        );
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn a_failed_write_reloads_and_reports(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        contract::a_failed_write_reloads_and_reports(&mut AdapterFactory::new(options));
    }

    /// The key has no cascade: a type a Document still names can't be deleted, and the Documents
    /// reload as they were.
    #[sqlx::test(migrations = "migrations/client")]
    async fn the_file_refuses_deleting_a_type_a_document_names(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        let mut factory = AdapterFactory::new(options);
        let mut documents = factory.create(default_types(), vec![tax_return()]);

        documents.delete_document_type(DocumentTypeId::seed(3));
        factory.pending.commit();
        let failures = factory.failures();
        assert_eq!(failures.len(), 1, "the delete is refused");
        assert_eq!(failures[0].domains, [Domain::Documents]);
        documents.reload().expect("the rows load");
        assert_eq!(documents.document_types(), default_types());
        assert_eq!(documents.documents(), [tax_return()]);
    }

    /// A Link whose record was deleted cascades away in the file; a Document still holding it in
    /// memory, as one does until it reloads, writes without it rather than failing.
    #[sqlx::test(migrations = "migrations/client")]
    async fn a_document_holding_a_deleted_records_link_still_writes(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        let mut factory = AdapterFactory::new(options);
        let item = factory.records().items[0].id;
        let mut held = tax_return();
        held.links = vec![DocumentLink::InventoryItem(item)];
        let mut documents = factory.create(default_types(), vec![held.clone()]);
        assert!(factory.execute("DELETE FROM items WHERE rowid = 1"));

        held.title = "Return 2025".into();
        documents.upsert_document(&held);
        let documents = factory.reload(documents);

        let stored = &documents.documents()[0];
        assert_eq!(stored.title, "Return 2025");
        assert!(stored.links.is_empty());
    }

    /// A Location's delete takes its Items, so their Links cascade too, and the commit asks for
    /// the Documents' reload.
    #[sqlx::test(migrations = "migrations/client")]
    async fn deleting_a_location_drops_the_links_to_its_items(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        let mut factory = AdapterFactory::new(options);
        let item = factory.records().items[0].id;
        let mut held = tax_return();
        held.links = vec![DocumentLink::InventoryItem(item)];
        let mut documents = factory.create(default_types(), vec![held.clone()]);
        let mut inventory =
            crate::InventoryAdapter::load(factory.pending.clone()).expect("the Inventory loads");
        let location = inventory
            .inventory()
            .inventory()
            .item(item)
            .expect("the Item is stored")
            .location;

        inventory.inventory_mut().delete_location(location);
        let cascades = factory.pending.commit();
        documents.reload().expect("the rows load");

        assert_eq!(cascades, [Domain::Documents]);
        assert!(factory.failures().is_empty());
        held.links.clear();
        assert_eq!(documents.documents(), [held]);
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn an_empty_database_loads_no_documents(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        let (writer, _failures) = Writer::spawn(options).expect("the test database opens");
        assert!(DocumentsAdapter::has_no_types(&writer).expect("it counts"));
        let adapter = DocumentsAdapter::load(PendingWrites::new(writer)).expect("it loads");
        assert!(adapter.documents().is_empty());
        assert!(adapter.document_types().is_empty());
    }

    fn tax_return() -> Document {
        Document {
            id: DocumentId::seed(1),
            path: "tax/return-2025.pdf".into(),
            kind: FileKind::Pdf,
            pages: 12,
            bytes: 204_800,
            title: "Tax return".into(),
            doc_type: DocumentType(DocumentTypeId::seed(3)),
            date: contract::today(),
            key_date: None,
            links: Vec::new(),
            extracted_text: None,
            filing: Filing::Filed,
            intake: None,
        }
    }
}
