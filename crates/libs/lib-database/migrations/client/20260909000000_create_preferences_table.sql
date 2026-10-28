-- Migration: create preferences table -- Ledger-scoped settings a user edits from inside a
-- running Client (ADR-0014). It has no Preference columns today: the date style, Colour Theme
-- and Colour Appearance belong to the device and are kept by each Client (ADR-0051), and the
-- base Unit lives in the ledger table (ADR-0044). The row stays for the Ledger-scoped
-- Preferences to come, such as the Ledger's name and financial year.
--
-- Singleton by convention, not a database constraint -- mirrors sync_users: a normal RowID
-- primary key, no CHECK (id = ...). See lib_database::Preferences::find_only/
-- get_or_create_default.
--
-- No seed row here: get_or_create_default inserts it at first use, because an id must be a
-- genuine UUIDv7 (see lib_core::RowID), which plain SQL cannot generate. The base Unit lives
-- in the ledger table, not here (ADR-0044).

CREATE TABLE IF NOT EXISTS preferences (
    id UUID PRIMARY KEY,
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
