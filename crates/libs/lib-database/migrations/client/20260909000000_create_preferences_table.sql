-- Migration: create preferences table -- Ledger-scoped settings a user edits from inside a
-- running Client (ADR-0014): default Unit for new Accounts, colour theme, date format,
-- decimal/thousands separator. Client-scoped (local-only, unsynced) Preferences remain a
-- valid category per CONTEXT.md, but nothing needs one yet, so no such table is built here.
--
-- Singleton by convention, not a database constraint -- mirrors sync_users: a normal RowID
-- primary key, no CHECK (id = ...). See lib_database::Preferences::find_only/
-- get_or_create_default.
--
-- No seed row here: get_or_create_default also seeds a default "USD" units row the first
-- time this table is used. A Unit's id must be a genuine UUIDv7 (see lib_core::RowID and
-- the payee_aliases migration's comment on the same constraint), which plain SQL cannot
-- generate -- so the seed happens in Rust at first use, not as an INSERT here.

CREATE TABLE IF NOT EXISTS preferences (
    id UUID PRIMARY KEY,
    default_unit_id UUID REFERENCES units(id) ON DELETE SET NULL,
    colour_theme TEXT NOT NULL,
    date_format TEXT NOT NULL,
    number_format TEXT NOT NULL,
    created_on TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    updated_on TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

CREATE TRIGGER IF NOT EXISTS trg_preferences_set_updated_on
AFTER UPDATE ON preferences
FOR EACH ROW
WHEN NEW.updated_on = OLD.updated_on
BEGIN
    UPDATE preferences
    SET updated_on = (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
    WHERE rowid = NEW.rowid;
END;
