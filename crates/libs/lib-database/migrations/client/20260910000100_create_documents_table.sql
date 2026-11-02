-- Migration: create documents table -- a reference to one file beside the Ledger with its metadata
-- (GLOSSARY.md's Document, #963). Its Links are in document_links.
--
-- path is unique: a file is one Document. document_type_id references the type with no cascade, so
-- a type with Documents can't be deleted; removing a type moves its Documents in the same batch,
-- and the key is checked at commit (DEFERRABLE INITIALLY DEFERRED) so the batch's order doesn't
-- matter. The Key Date is three columns, set together or not at all. bytes and the Extracted
-- Facts' total are what was read from the file; the total is a decimal string, so no precision is
-- lost to a float.
--
-- The Intake (how an Unfiled Document came in) is nullable columns: source and received_on are set
-- together, and the Extracted Facts (merchant, date, total, fact_document_type_id) only with them.
-- A Document filed by hand has none. A fact's Document Type goes null when its type is deleted
-- (ON DELETE SET NULL). Skip is the session's and is not stored.

CREATE TABLE IF NOT EXISTS documents (
    id UUID PRIMARY KEY,
    path TEXT NOT NULL UNIQUE,
    kind TEXT NOT NULL CHECK (kind IN ('pdf', 'jpg', 'png')),
    pages INTEGER NOT NULL CHECK (pages >= 0),
    bytes INTEGER NOT NULL CHECK (bytes >= 0),
    title TEXT NOT NULL,
    document_type_id UUID NOT NULL
        REFERENCES document_types(id) DEFERRABLE INITIALLY DEFERRED,
    date TEXT NOT NULL,
    key_date_kind TEXT CHECK (key_date_kind IN ('renews', 'ends', 'expires', 'revalue')),
    key_date TEXT,
    key_date_reminder BOOLEAN,
    extracted_text TEXT,
    filing TEXT NOT NULL CHECK (filing IN ('unfiled', 'filed')),
    source TEXT CHECK (
        source IN ('scanned', 'emailed', 'downloads', 'watched_folder', 'dropped', 'import')
    ),
    received_on TEXT,
    fact_merchant TEXT,
    fact_date TEXT,
    fact_total TEXT,
    fact_document_type_id UUID REFERENCES document_types(id) ON DELETE SET NULL,
    created_on TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    updated_on TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    CHECK ((key_date_kind IS NULL) = (key_date IS NULL)),
    CHECK ((key_date IS NULL) = (key_date_reminder IS NULL)),
    CHECK ((source IS NULL) = (received_on IS NULL)),
    CHECK (
        source IS NOT NULL
        OR (fact_merchant IS NULL AND fact_date IS NULL AND fact_total IS NULL
            AND fact_document_type_id IS NULL)
    )
);

CREATE INDEX IF NOT EXISTS idx_documents_document_type_id ON documents(document_type_id);
CREATE INDEX IF NOT EXISTS idx_documents_fact_document_type_id
    ON documents(fact_document_type_id);

CREATE TRIGGER IF NOT EXISTS trg_documents_set_updated_on
AFTER UPDATE ON documents
FOR EACH ROW
WHEN NEW.updated_on = OLD.updated_on
BEGIN
    UPDATE documents
    SET updated_on = (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
    WHERE rowid = NEW.rowid;
END;
