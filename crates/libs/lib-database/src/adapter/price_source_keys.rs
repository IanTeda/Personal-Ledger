//! The Price Source API keys (ADR-0043's amendment): each key sealed with XChaCha20-Poly1305 on the
//! `price_source_keys` table, which syncs with the Ledger, under a key derived with Argon2id from
//! the device's API key secret, which never does. A fresh nonce seals each write and the source's id
//! is the associated data, so a row moved onto another source won't open. `key_check`, an HMAC of
//! the derived key, says which secret sealed a row without saying the secret, so a device with
//! another secret refuses to write rather than leave a Ledger whose keys no one device can read.
//!
//! The rows are held in memory like every adapter's (ADR-0046), sealed: a key is opened only as
//! `lib-units` asks for it, into a `SecretString` the request exposes as it is sent. No trace here
//! records a key, the secret, a nonce or a ciphertext, and no `Debug` prints them.

use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

use base64::Engine as _;
use chacha20poly1305::aead::{Aead as _, Payload};
use chacha20poly1305::{KeyInit as _, XChaCha20Poly1305, XNonce};
use hmac::Mac as _;
use lib_core::PriceSourceId;
use rand::RngCore as _;
use secrecy::{ExposeSecret as _, SecretBox, SecretString};
use sqlx::SqliteConnection;

use crate::Result;
use crate::commit::PendingWrites;
use crate::writer::{BoxFuture, Change, Domain};

/// The Argon2id salt. Fixed, so every device given the same secret derives the same key; the
/// secret a Client generates has 256 bits of its own, so a salt per Ledger would add nothing.
const SALT: &[u8] = b"personal-ledger/price-source-keys/v1";

/// What `key_check` is the HMAC of, under the derived key.
const CHECK_LABEL: &[u8] = b"personal-ledger/price-source-keys/key-check/v1";

/// XChaCha20's nonce, in bytes.
const NONCE_LEN: usize = 24;

/// One source's key as the table holds it, sealed.
#[derive(Clone, PartialEq, Eq)]
pub struct SealedKey {
    pub(crate) source: PriceSourceId,
    pub(crate) nonce: Vec<u8>,
    pub(crate) ciphertext: Vec<u8>,
    pub(crate) key_check: Vec<u8>,
}

impl SealedKey {
    pub fn source(&self) -> PriceSourceId {
        self.source
    }
}

/// The source alone: the rest is ciphertext no log needs.
impl fmt::Debug for SealedKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SealedKey")
            .field("source", &self.source)
            .finish_non_exhaustive()
    }
}

/// The sealed keys, wherever they are kept: [`PriceSourceKeysAdapter`] or
/// [`InMemorySealedKeys`]. A write replaces or removes a source's one row.
pub trait PriceSourceKeysStore {
    fn sealed(&self) -> Vec<SealedKey>;

    fn seal(&mut self, key: SealedKey);

    fn delete(&mut self, source: PriceSourceId);

    /// Puts the rows back as they are stored, after a write failed (ADR-0046).
    ///
    /// # Errors
    /// Returns the error reading them back; the rows in memory are left as they were.
    fn reload(&mut self) -> Result<()>;
}

/// The sealed keys in memory, for the tests and a Client with no Ledger file.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InMemorySealedKeys {
    pub rows: Vec<SealedKey>,
}

/// Memory is all there is, so a reload has nothing to read.
impl PriceSourceKeysStore for InMemorySealedKeys {
    fn sealed(&self) -> Vec<SealedKey> {
        self.rows.clone()
    }

    fn seal(&mut self, key: SealedKey) {
        upsert(&mut self.rows, key);
    }

    fn delete(&mut self, source: PriceSourceId) {
        self.rows.retain(|row| row.source != source);
    }

    fn reload(&mut self) -> Result<()> {
        Ok(())
    }
}

fn upsert(rows: &mut Vec<SealedKey>, key: SealedKey) {
    match rows.iter_mut().find(|row| row.source == key.source) {
        Some(row) => *row = key,
        None => rows.push(key),
    }
}

/// The `price_source_keys` rows, held in memory and written through the shared writer.
pub struct PriceSourceKeysAdapter {
    rows: Vec<SealedKey>,
    pending: PendingWrites,
}

impl PriceSourceKeysAdapter {
    /// Loads every row through `pending`'s writer, blocking until they are read.
    ///
    /// # Errors
    /// Returns the error of the read.
    #[tracing::instrument(name = "Load the Price Source keys", level = "debug", skip_all, err)]
    pub fn load(pending: PendingWrites) -> Result<Self> {
        let rows = pending.writer().run(|pool| async move {
            let mut conn = pool.acquire().await?;
            read(&mut conn).await
        })?;
        tracing::debug!(rows = rows.len(), "Loaded the Price Source keys");
        Ok(Self { rows, pending })
    }
}

impl PriceSourceKeysStore for PriceSourceKeysAdapter {
    fn sealed(&self) -> Vec<SealedKey> {
        self.rows.clone()
    }

    fn seal(&mut self, key: SealedKey) {
        upsert(&mut self.rows, key.clone());
        self.pending.push(KeysChange::Seal(key));
    }

    fn delete(&mut self, source: PriceSourceId) {
        self.rows.retain(|row| row.source != source);
        self.pending.push(KeysChange::Delete(source));
    }

    /// Reloaded with the Units, whose pending writes the Units adapter's reload drops.
    #[tracing::instrument(name = "Reload the Price Source keys", level = "debug", skip_all, err)]
    fn reload(&mut self) -> Result<()> {
        let fresh = Self::load(self.pending.clone())?;
        self.rows = fresh.rows;
        self.pending.discard(Domain::Units);
        Ok(())
    }
}

/// A queued write of one source's row. A key is part of its source, so it is the Units' domain.
enum KeysChange {
    Seal(SealedKey),
    Delete(PriceSourceId),
}

impl Change for KeysChange {
    fn domain(&self) -> Domain {
        Domain::Units
    }

    fn apply<'c>(self: Box<Self>, conn: &'c mut SqliteConnection) -> BoxFuture<'c, Result<()>> {
        Box::pin(async move {
            match *self {
                Self::Seal(key) => {
                    sqlx::query!(
                        "INSERT INTO price_source_keys (price_source_id, nonce, ciphertext, key_check)
                         VALUES (?, ?, ?, ?)
                         ON CONFLICT(price_source_id) DO UPDATE SET
                             nonce = excluded.nonce,
                             ciphertext = excluded.ciphertext,
                             key_check = excluded.key_check",
                        key.source,
                        key.nonce,
                        key.ciphertext,
                        key.key_check,
                    )
                    .execute(conn)
                    .await?;
                }
                Self::Delete(source) => {
                    sqlx::query!(
                        "DELETE FROM price_source_keys WHERE price_source_id = ?",
                        source
                    )
                    .execute(conn)
                    .await?;
                }
            }
            Ok(())
        })
    }
}

async fn read(conn: &mut SqliteConnection) -> Result<Vec<SealedKey>> {
    let rows = sqlx::query!(
        r#"SELECT price_source_id AS "price_source_id!: PriceSourceId", nonce, ciphertext, key_check
           FROM price_source_keys
           ORDER BY price_source_id"#
    )
    .fetch_all(conn)
    .await?;
    Ok(rows
        .into_iter()
        .map(|row| SealedKey {
            source: row.price_source_id,
            nonce: row.nonce,
            ciphertext: row.ciphertext,
            key_check: row.key_check,
        })
        .collect())
}

/// The device's API key secret (`[Secrets] api_key_secret`), shared by every Ledger a Client opens,
/// and where a secret it generates is kept. Cloning shares it, so a secret generated on one Ledger
/// seals the next. The derived key is worked out once, on first use, since Argon2id is slow by
/// design.
#[derive(Clone)]
pub struct ApiKeySecret {
    inner: Rc<SecretCell>,
}

struct SecretCell {
    configured: RefCell<Option<SecretString>>,
    derived: RefCell<Option<Rc<Sealer>>>,
    /// Where a generated secret is written; `None` keeps one in memory only.
    writer: Option<lib_config::ConfigWriter>,
}

impl ApiKeySecret {
    /// The secret `configured` gives, or none yet, generated into `writer`'s file when a Ledger
    /// with no key saves its first ([`lib_config::ConfigWriter::set_api_key_secret`]).
    pub fn new(configured: Option<SecretString>, writer: lib_config::ConfigWriter) -> Self {
        Self::with(configured, Some(writer))
    }

    /// The secret `configured` gives, or none yet, a generated one kept in memory only: for a
    /// Client with no configuration file to keep it in, and the tests.
    pub fn in_memory(configured: Option<SecretString>) -> Self {
        Self::with(configured, None)
    }

    fn with(configured: Option<SecretString>, writer: Option<lib_config::ConfigWriter>) -> Self {
        let configured = configured.filter(|secret| !secret.expose_secret().trim().is_empty());
        Self {
            inner: Rc::new(SecretCell {
                configured: RefCell::new(configured),
                derived: RefCell::new(None),
                writer,
            }),
        }
    }

    /// Whether a secret is set, configured or generated.
    pub fn is_set(&self) -> bool {
        self.inner.configured.borrow().is_some()
    }

    /// The secret's fingerprint, as Settings › Data & backup shows it so two devices can be seen to
    /// share one (`3F9A-21C0`): the first four bytes of the derived key's `key_check`, which is
    /// not secret. `None` with no secret set.
    pub fn fingerprint(&self) -> Option<String> {
        let sealer = self.sealer()?;
        let [a, b, c, d, ..] = *sealer.check() else {
            return None;
        };
        Some(format!("{a:02X}{b:02X}-{c:02X}{d:02X}"))
    }

    /// The key derived from the secret, `None` with no secret set.
    pub(crate) fn sealer(&self) -> Option<Rc<Sealer>> {
        if let Some(sealer) = self.inner.derived.borrow().as_ref() {
            return Some(sealer.clone());
        }
        let sealer = Rc::new(Sealer::derive(self.inner.configured.borrow().as_ref()?));
        *self.inner.derived.borrow_mut() = Some(sealer.clone());
        Some(sealer)
    }

    /// Generates a 256-bit secret, keeps it in the configuration file and uses it from here on.
    ///
    /// # Errors
    /// Returns the error writing the file; no secret is set then.
    pub(crate) fn generate(&self) -> Result<Rc<Sealer>> {
        let mut bytes = [0_u8; 32];
        rand::rngs::OsRng.fill_bytes(&mut bytes);
        let secret =
            SecretString::from(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes));
        secrecy::zeroize::Zeroize::zeroize(&mut bytes);
        if let Some(writer) = &self.inner.writer {
            writer.set_api_key_secret(&secret)?;
        }
        tracing::info!("Generated this device's API key secret");
        let sealer = Rc::new(Sealer::derive(&secret));
        *self.inner.configured.borrow_mut() = Some(secret);
        *self.inner.derived.borrow_mut() = Some(sealer.clone());
        Ok(sealer)
    }
}

/// Whether a secret is set, never the secret.
impl fmt::Debug for ApiKeySecret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ApiKeySecret")
            .field("set", &self.is_set())
            .finish_non_exhaustive()
    }
}

/// The key derived from one secret: what seals and opens a key, and its fingerprint.
pub(crate) struct Sealer {
    key: SecretBox<[u8; 32]>,
    check: Vec<u8>,
}

impl Sealer {
    #[expect(
        clippy::expect_used,
        reason = "Argon2's default parameters, a 36-byte salt and a 32-byte output are all in range"
    )]
    fn derive(secret: &SecretString) -> Self {
        let mut key = SecretBox::new(Box::new([0_u8; 32]));
        argon2::Argon2::new(
            argon2::Algorithm::Argon2id,
            argon2::Version::V0x13,
            argon2::Params::DEFAULT,
        )
        .hash_password_into(
            secret.expose_secret().as_bytes(),
            SALT,
            secrecy::ExposeSecretMut::expose_secret_mut(&mut key),
        )
        .expect("default parameters, a 36-byte salt and a 32-byte output are in range");
        let mut mac = <hmac::Hmac<sha2::Sha256> as hmac::Mac>::new_from_slice(key.expose_secret())
            .expect("HMAC takes a key of any length");
        mac.update(CHECK_LABEL);
        let check = mac.finalize().into_bytes().to_vec();
        Self { key, check }
    }

    /// The fingerprint a row this key sealed carries.
    pub(crate) fn check(&self) -> &[u8] {
        &self.check
    }

    fn cipher(&self) -> XChaCha20Poly1305 {
        XChaCha20Poly1305::new(self.key.expose_secret().into())
    }

    /// `key` sealed for `source` under a fresh nonce.
    ///
    /// # Errors
    /// Returns [`crate::Error::Generic`] if the cipher refuses, which it does only past its length
    /// limit, far beyond any API key.
    pub(crate) fn seal(&self, source: PriceSourceId, key: &SecretString) -> Result<SealedKey> {
        let mut nonce = [0_u8; NONCE_LEN];
        rand::rngs::OsRng.fill_bytes(&mut nonce);
        let aad = source.to_string();
        let ciphertext = self
            .cipher()
            .encrypt(
                XNonce::from_slice(&nonce),
                Payload {
                    msg: key.expose_secret().as_bytes(),
                    aad: aad.as_bytes(),
                },
            )
            .map_err(|_| crate::Error::Generic("an API key couldn't be sealed".into()))?;
        Ok(SealedKey {
            source,
            nonce: nonce.to_vec(),
            ciphertext,
            key_check: self.check.clone(),
        })
    }

    /// `sealed`'s key, `None` if another secret sealed it or it was sealed for another source.
    pub(crate) fn open(&self, sealed: &SealedKey) -> Option<SecretString> {
        if sealed.nonce.len() != NONCE_LEN {
            return None;
        }
        let aad = sealed.source.to_string();
        let plain = self
            .cipher()
            .decrypt(
                XNonce::from_slice(&sealed.nonce),
                Payload {
                    msg: &sealed.ciphertext,
                    aad: aad.as_bytes(),
                },
            )
            .ok()?;
        let key = String::from_utf8(plain).ok()?;
        Some(SecretString::from(key))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use lib_units::{KeyState, PriceSourceKeys as _};
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

    use super::*;
    use crate::adapter::store::SharedPriceSourceKeysStore;
    use crate::ports::StoredPriceSourceKeys;
    use crate::{Commit as _, Writer};

    const KEY: &str = "k3y-0f-a-pr1ce-s0urce";

    fn secret(text: &str) -> ApiKeySecret {
        ApiKeySecret::in_memory(Some(text.into()))
    }

    fn exposed(key: Option<SecretString>) -> Option<String> {
        key.map(|key| key.expose_secret().to_string())
    }

    /// The keys table of a migrated test database, loaded as start-up loads it.
    struct OnFile {
        writer: Writer,
        pending: PendingWrites,
        rows: SharedPriceSourceKeysStore,
    }

    impl OnFile {
        fn new(options: SqliteConnectOptions) -> Self {
            let (writer, _failures) = Writer::spawn(options).expect("the test database opens");
            let pending = PendingWrites::new(writer.clone());
            let adapter = PriceSourceKeysAdapter::load(pending.clone()).expect("the rows load");
            Self {
                writer,
                pending,
                rows: Rc::new(RefCell::new(adapter)),
            }
        }

        fn port(&self, secret: &ApiKeySecret) -> StoredPriceSourceKeys {
            StoredPriceSourceKeys::new(self.rows.clone(), secret.clone())
        }

        /// Commits the pending writes, waits for them to land and reads the rows back.
        fn commit_and_reload(&self) {
            self.pending.commit();
            self.writer.flush().expect("the writer is running");
            self.rows.borrow_mut().reload().expect("the rows reload");
        }

        fn stored(&self) -> Vec<(Vec<u8>, Vec<u8>)> {
            self.writer
                .run(|pool| async move {
                    Ok(sqlx::query_as::<_, (Vec<u8>, Vec<u8>)>(
                        "SELECT nonce, ciphertext FROM price_source_keys",
                    )
                    .fetch_all(&pool)
                    .await?)
                })
                .expect("the rows read")
        }
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn a_saved_key_round_trips_through_the_file_sealed_under_a_fresh_nonce_each_write(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        let file = OnFile::new(options);
        let source = PriceSourceId::new();
        let mut keys = file.port(&secret("one device's secret"));

        keys.store_key(source, KEY.into())
            .expect("the key is sealed");
        file.commit_and_reload();
        let first = file.stored();
        keys.store_key(source, KEY.into())
            .expect("the key is sealed again");
        file.commit_and_reload();
        let second = file.stored();

        assert_eq!(exposed(keys.key(source)), Some(KEY.to_string()));
        assert_eq!(keys.key_state(source), KeyState::Saved);
        assert_eq!((first.len(), second.len()), (1, 1), "one row per source");
        let (nonce, ciphertext) = &second[0];
        assert_eq!(nonce.len(), 24);
        assert_ne!(first[0].0, *nonce, "each write takes a fresh nonce");
        assert!(
            !ciphertext
                .windows(KEY.len())
                .any(|window| window == KEY.as_bytes()),
            "the file holds no key in the clear"
        );
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn a_row_moved_onto_another_source_wont_open(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        let file = OnFile::new(options);
        let mut keys = file.port(&secret("one device's secret"));
        keys.store_key(lib_units::COINGECKO, KEY.into())
            .expect("the key is sealed");
        file.commit_and_reload();

        let mut moved = file.rows.borrow().sealed()[0].clone();
        moved.source = PriceSourceId::new();
        file.rows.borrow_mut().delete(lib_units::COINGECKO);
        file.rows.borrow_mut().seal(moved.clone());
        file.commit_and_reload();
        let moved = moved.source;

        assert_eq!(keys.key_state(lib_units::COINGECKO), KeyState::NotSet);
        assert_eq!(
            keys.key(moved).map(|_| ()),
            None,
            "sealed for CoinGecko alone"
        );
        assert_eq!(keys.key_state(moved), KeyState::Unreadable);
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn another_devices_secret_reads_none_and_may_not_write(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        let file = OnFile::new(options);
        let source = PriceSourceId::new();
        file.port(&secret("the first device's secret"))
            .store_key(source, KEY.into())
            .expect("the key is sealed");
        file.commit_and_reload();

        let none = ApiKeySecret::in_memory(None);
        for other in [secret("the second device's secret"), none.clone()] {
            let mut keys = file.port(&other);

            assert!(keys.key(source).is_none());
            assert_eq!(keys.key_state(source), KeyState::Unreadable);
            assert!(!keys.can_write(), "its key_check doesn't match the rows'");
            assert_eq!(
                keys.store_key(PriceSourceId::new(), "another".into()),
                Err(lib_units::Error::KeysLocked)
            );
        }
        assert_eq!(file.rows.borrow().sealed().len(), 1, "nothing was written");
        assert!(!none.is_set(), "a refused write generates no secret");
    }

    /// A configuration file holding `text`, and a secret with none configured that keeps one there.
    fn configured(dir: &std::path::Path, text: &str) -> (std::path::PathBuf, ApiKeySecret) {
        let path = dir.join("personal-ledger.conf");
        std::fs::write(&path, text).expect("the file is written");
        let writer =
            lib_config::ConfigWriter::new(Some(path.clone()), lib_config::Overrides::default());
        (path, ApiKeySecret::new(None, writer))
    }

    #[test]
    fn a_secret_is_generated_into_the_configuration_on_a_ledgers_first_key_and_never_after() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let (path, secret) = configured(dir.path(), "[General]\ntoasts = false\n");
        let rows: SharedPriceSourceKeysStore = Rc::new(RefCell::new(InMemorySealedKeys::default()));
        let mut keys = StoredPriceSourceKeys::new(rows.clone(), secret.clone());
        assert!(
            keys.can_write(),
            "a Ledger with no keys takes this device's"
        );

        keys.store_key(lib_units::COINGECKO, KEY.into())
            .expect("the key is sealed");
        let written = std::fs::read_to_string(&path).expect("the file reads");
        keys.store_key(PriceSourceId::new(), "another".into())
            .expect("the key is sealed");

        assert!(secret.is_set());
        let line = written
            .lines()
            .find(|line| line.starts_with("api_key_secret = "))
            .expect("the secret is kept under [Secrets]");
        assert!(written.contains("\n[Secrets]\n"), "{written}");
        let generated = line
            .trim_start_matches("api_key_secret = ")
            .trim_matches('"');
        assert_eq!(generated.len(), 43, "256 bits, base64url with no padding");
        assert!(
            generated
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        );
        assert_eq!(
            std::fs::read_to_string(&path).expect("the file reads"),
            written,
            "a second key reuses the secret"
        );
        let reread = ApiKeySecret::in_memory(Some(generated.into()));
        assert_eq!(
            exposed(keys.key(lib_units::COINGECKO)),
            exposed(StoredPriceSourceKeys::new(rows, reread).key(lib_units::COINGECKO)),
            "the kept secret opens the key"
        );
    }

    #[test]
    fn no_secret_is_generated_with_one_configured_or_on_a_ledger_whose_keys_another_sealed() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let text = "[General]\ntoasts = false\n";
        let (path, unset) = configured(dir.path(), text);
        let sealed: SharedPriceSourceKeysStore =
            Rc::new(RefCell::new(InMemorySealedKeys::default()));
        StoredPriceSourceKeys::new(sealed.clone(), secret("elsewhere"))
            .store_key(PriceSourceId::new(), KEY.into())
            .expect("the key is sealed");

        let refused = StoredPriceSourceKeys::new(sealed, unset.clone())
            .store_key(PriceSourceId::new(), KEY.into());
        let writer =
            lib_config::ConfigWriter::new(Some(path.clone()), lib_config::Overrides::default());
        let set = ApiKeySecret::new(Some("mine".into()), writer);
        StoredPriceSourceKeys::new(Rc::new(RefCell::new(InMemorySealedKeys::default())), set)
            .store_key(PriceSourceId::new(), KEY.into())
            .expect("the key is sealed");

        assert_eq!(refused, Err(lib_units::Error::KeysLocked));
        assert!(!unset.is_set());
        assert_eq!(
            std::fs::read_to_string(&path).expect("the file reads"),
            text
        );
    }

    #[test]
    fn a_secret_that_cant_be_kept_saves_no_key() {
        let unkept = ApiKeySecret::new(
            None,
            lib_config::ConfigWriter::new(None, lib_config::Overrides::default()),
        );
        let rows: SharedPriceSourceKeysStore = Rc::new(RefCell::new(InMemorySealedKeys::default()));
        let mut keys = StoredPriceSourceKeys::new(rows.clone(), unkept.clone());

        assert_eq!(
            keys.store_key(PriceSourceId::new(), KEY.into()),
            Err(lib_units::Error::KeyNotSaved)
        );
        assert!(!unkept.is_set());
        assert!(rows.borrow().sealed().is_empty());
    }

    /// Every event and span field recorded on this thread, as text.
    #[derive(Clone, Default)]
    struct Captured(Arc<Mutex<String>>);

    impl tracing::field::Visit for Captured {
        fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn fmt::Debug) {
            let line = format!("{}={value:?}\n", field.name());
            self.0.lock().expect("one test holds it").push_str(&line);
        }
    }

    impl tracing::Subscriber for Captured {
        fn enabled(&self, _: &tracing::Metadata<'_>) -> bool {
            true
        }

        fn new_span(&self, span: &tracing::span::Attributes<'_>) -> tracing::span::Id {
            span.record(&mut self.clone());
            tracing::span::Id::from_u64(1)
        }

        fn record(&self, _: &tracing::span::Id, values: &tracing::span::Record<'_>) {
            values.record(&mut self.clone());
        }

        fn record_follows_from(&self, _: &tracing::span::Id, _: &tracing::span::Id) {}

        fn event(&self, event: &tracing::Event<'_>) {
            event.record(&mut self.clone());
        }

        fn enter(&self, _: &tracing::span::Id) {}

        fn exit(&self, _: &tracing::span::Id) {}
    }

    #[test]
    fn no_trace_or_debug_output_holds_a_key_the_secret_a_nonce_or_a_ciphertext() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let (path, secret) = configured(dir.path(), "");
        let rows: SharedPriceSourceKeysStore = Rc::new(RefCell::new(InMemorySealedKeys::default()));
        let captured = Captured::default();

        // Reaches the callsites on this thread first, so each is registered before the rebuild
        // below: one another test thread registered at the same moment, with no subscriber, could
        // otherwise cache that nothing listens.
        let _ = StoredPriceSourceKeys::new(
            Rc::new(RefCell::new(InMemorySealedKeys::default())),
            ApiKeySecret::in_memory(None),
        )
        .store_key(PriceSourceId::new(), KEY.into());
        tracing::subscriber::with_default(captured.clone(), || {
            tracing::callsite::rebuild_interest_cache();
            let mut keys = StoredPriceSourceKeys::new(rows.clone(), secret.clone());
            keys.store_key(lib_units::COINGECKO, KEY.into())
                .expect("the key is sealed");
            let _ = keys.key(lib_units::COINGECKO);
            let _ = StoredPriceSourceKeys::new(rows.clone(), self::secret("another"))
                .store_key(PriceSourceId::new(), KEY.into());
        });

        let generated = std::fs::read_to_string(&path).expect("the file reads");
        let generated = generated
            .lines()
            .find_map(|line| line.strip_prefix("api_key_secret = "))
            .expect("a secret was generated")
            .trim_matches('"')
            .to_string();
        let sealed = rows.borrow().sealed()[0].clone();
        let output = format!(
            "{}\n{sealed:?}\n{secret:?}\n{:?}",
            captured.0.lock().expect("the test is done"),
            rows.borrow().sealed()
        );
        assert!(
            output.contains("Generated this device's API key secret"),
            "{output}"
        );
        for leaked in [
            KEY.to_string(),
            generated,
            format!("{:?}", sealed.nonce),
            format!("{:?}", sealed.ciphertext),
        ] {
            assert!(!output.contains(&leaked), "{leaked} in {output}");
        }
    }

    #[test]
    fn an_api_key_secret_left_blank_is_none() {
        assert!(!ApiKeySecret::in_memory(Some("  ".into())).is_set());
        assert!(ApiKeySecret::in_memory(None).sealer().is_none());
    }

    #[test]
    fn the_fingerprint_is_none_without_a_secret_and_the_same_on_two_devices_with_one() {
        let fingerprint = |secret: ApiKeySecret| secret.fingerprint();

        assert_eq!(fingerprint(ApiKeySecret::in_memory(None)), None);
        let first = fingerprint(secret("one secret")).expect("a secret has a fingerprint");
        assert_eq!(first.len(), 9);
        assert_eq!(first.as_bytes()[4], b'-');
        assert!(
            first
                .chars()
                .all(|c| c == '-' || c.is_ascii_digit() || c.is_ascii_uppercase()),
            "{first}"
        );
        assert_eq!(fingerprint(secret("one secret")), Some(first.clone()));
        assert_ne!(fingerprint(secret("another secret")), Some(first));
    }

    #[test]
    fn remove_all_deletes_every_key_row_and_the_sources_read_as_not_set() {
        use lib_settings::ApiKeys as _;

        let rows: SharedPriceSourceKeysStore = Rc::new(RefCell::new(InMemorySealedKeys::default()));
        let secret = secret("one device's secret");
        let mut keys = StoredPriceSourceKeys::new(rows.clone(), secret.clone());
        let broker = PriceSourceId::new();
        keys.store_key(lib_units::COINGECKO, KEY.into())
            .expect("the key is sealed");
        keys.store_key(broker, KEY.into())
            .expect("the key is sealed");
        assert_eq!(rows.borrow().sealed().len(), 2);

        keys.remove_all();

        assert!(rows.borrow().sealed().is_empty());
        assert_eq!(keys.key_state(lib_units::COINGECKO), KeyState::NotSet);
        assert_eq!(keys.key_state(broker), KeyState::NotSet);
        assert!(secret.is_set(), "the secret stays on the device");
    }
}
