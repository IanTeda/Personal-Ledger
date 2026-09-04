-- Migration: create accounts table -- the Sync Server's own auth user store (ADR-0010:
-- OAuth2 Authorization Code + PKCE, Sync Server as its own authorization server).
--
-- Single-account this cycle (ADR-0010): the PRD's deployment profile is one self-hoster,
-- not multiple distinct human users of one Sync Server. `refresh_token_hash` is nullable
-- (no active session yet) and rotates on every token refresh -- the previous value stops
-- being valid the moment a new one is written.

CREATE TABLE IF NOT EXISTS accounts (
    id UUID PRIMARY KEY,
    username TEXT UNIQUE NOT NULL,
    password_hash TEXT NOT NULL,
    refresh_token_hash TEXT,
    created_on TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    updated_on TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);


-- Trigger to update updated_on on every account row change (refresh-token rotation, etc.)
CREATE TRIGGER IF NOT EXISTS trg_accounts_set_updated_on
AFTER UPDATE ON accounts
FOR EACH ROW
WHEN NEW.updated_on = OLD.updated_on
BEGIN
    UPDATE accounts
    SET updated_on = (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
    WHERE rowid = NEW.rowid;
END;
