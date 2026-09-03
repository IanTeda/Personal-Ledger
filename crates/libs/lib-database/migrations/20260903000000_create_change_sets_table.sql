-- Migration: create change_sets table -- the Sync Server's own durable Change Set log
-- (ADR-0009: per-field last-write-wins over a SQLite Change Set log).
--
-- One row is one field-level edit: the target table/row/field, its new value, a Hybrid
-- Logical Clock timestamp (ADR-0009; see lib_domain::HybridLogicalClock) for last-write-wins
-- comparison, the originating Client's stable ID for tie-breaking, and a version/parent-version
-- placeholder held for a future CRDT or manual-merge upgrade path. `id` is a UUIDv7 RowID, so
-- it is already chronologically sortable and doubles as the Sync Server's pull cursor.

CREATE TABLE IF NOT EXISTS change_sets (
    id UUID PRIMARY KEY,
    table_name TEXT NOT NULL,
    row_id UUID NOT NULL,
    field_name TEXT NOT NULL,
    value TEXT,
    hlc TEXT NOT NULL,
    client_id UUID NOT NULL,
    version INTEGER NOT NULL DEFAULT 0,
    created_on TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);


-- Cover LWW-compare lookups (has this table/row/field been changed since X?) and
-- per-Client queries.
CREATE INDEX IF NOT EXISTS idx_change_sets_table_row_field ON change_sets(table_name, row_id, field_name);
CREATE INDEX IF NOT EXISTS idx_change_sets_client_id ON change_sets(client_id);
