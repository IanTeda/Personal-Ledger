-- Migration: create price_source_keys table -- each Price Source's API key, encrypted (ADR-0043's
-- amendment "API keys sync, encrypted with the API key secret"). One row per source, built-ins
-- included, keyed by the source's id with no foreign key, as units.price_source_id has none: a
-- built-in has a price_sources row only once a person overrides it.
--
-- The key is XChaCha20-Poly1305 ciphertext under a key derived with Argon2id from the device's API
-- key secret, with a fresh 24-byte nonce on each write and the source id as associated data, so a
-- row moved onto another source won't decrypt. key_check is a keyed fingerprint of the derived key,
-- not secret: a Client writes a key only when its own matches the rows', or there are none. The
-- rows sync with the Ledger; the secret never does.

CREATE TABLE IF NOT EXISTS price_source_keys (
    price_source_id UUID PRIMARY KEY,
    nonce BLOB NOT NULL CHECK (length(nonce) = 24),
    ciphertext BLOB NOT NULL,
    key_check BLOB NOT NULL,
    created_on TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    updated_on TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);


-- Trigger to update updated_on on every key row change
CREATE TRIGGER IF NOT EXISTS trg_price_source_keys_set_updated_on
AFTER UPDATE ON price_source_keys
FOR EACH ROW
WHEN NEW.updated_on = OLD.updated_on
BEGIN
    UPDATE price_source_keys
    SET updated_on = (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
    WHERE rowid = NEW.rowid;
END;
