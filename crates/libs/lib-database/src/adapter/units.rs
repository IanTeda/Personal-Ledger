//! The Units adapter: `lib-units`' Units, Price Sources and base Unit ports over the `units`,
//! `price_sources` and `ledger` tables (ADR-0046).
//!
//! It loads every row into memory, blocking, and serves reads from there. A port write changes
//! memory at once and joins the [`PendingWrites`] every adapter shares, which the bin commits
//! after each Interactor call, so one call is one transaction whatever domains it writes.
//!
//! A built-in Price Source has a row only once a person overrides it, and Manual entry never has
//! one (ADR-0043), so `units.price_source_id` is not a foreign key.

use chrono::{DateTime, Utc};
use lib_core::{Money, PriceSourceId, UnitId, UnitKind};
use lib_units::{
    BaseUnitRepository, BuiltInOverride, CustomSource, Endpoint, KeyPlacement, LastPrice, PortsMut,
    PriceLink, PriceSourceAuth, PriceSourceRecord, PriceSourceRepository, UnitRepository, UnitRow,
    UnitUsage, Updates,
};
use sqlx::SqliteConnection;

use crate::commit::PendingWrites;
use crate::writer::{BoxFuture, Change, Domain, Writer};
use crate::{Error, Result};

/// One port write, saved in the order it was made.
#[derive(Debug, Clone)]
enum UnitsChange {
    UpsertUnit(UnitRow),
    DeleteUnit(UnitId),
    UpsertPriceSource(PriceSourceRecord),
    DeletePriceSource(PriceSourceId),
    SetBaseUnit(UnitId),
}

/// The Units domain, held in memory and saved through the shared writer.
pub struct UnitsAdapter {
    units: StoredUnits,
    price_sources: StoredPriceSources,
    base_unit: StoredBaseUnit,
    pending: PendingWrites,
}

impl UnitsAdapter {
    /// Loads every Unit, Price Source row and the base Unit through `pending`'s writer, blocking
    /// until they are read. The adapter's writes join `pending`.
    ///
    /// # Errors
    /// Returns [`Error::NotFound`] if the database has no base Unit yet, as an empty one doesn't:
    /// [`Self::seed`] it first. Otherwise the error of the read.
    #[tracing::instrument(name = "Load the Units", level = "debug", skip_all)]
    pub fn load(pending: PendingWrites) -> Result<Self> {
        let rows = pending.writer().run(|pool| async move {
            let mut conn = pool.acquire().await?;
            read(&mut conn).await
        })?;
        Ok(Self {
            units: StoredUnits {
                rows: rows.units,
                pending: pending.clone(),
            },
            price_sources: StoredPriceSources {
                rows: rows.records,
                pending: pending.clone(),
            },
            base_unit: StoredBaseUnit {
                id: rows.base_unit,
                pending: pending.clone(),
            },
            pending,
        })
    }

    /// Writes `units`, `records` and `base_unit` in one transaction, blocking until it lands. For
    /// an empty database, before the first [`Self::load`]. A Ledger that already has a base Unit
    /// keeps it, so seeding again never resets the person's choice.
    ///
    /// [`Self::seed`] with the rows of an in-memory set, the stub's or a test's.
    ///
    /// # Errors
    /// Returns the error of the write; nothing is written then.
    pub fn seed_from(writer: &Writer, ports: lib_units::fake::InMemoryPorts) -> Result<()> {
        Self::seed(
            writer,
            ports.units.units,
            ports.price_sources.records,
            ports.base_unit.0,
        )
    }

    /// # Errors
    /// Returns the error of the write; nothing is written then.
    #[tracing::instrument(name = "Seed the Units", level = "debug", skip_all, err)]
    pub fn seed(
        writer: &Writer,
        units: Vec<UnitRow>,
        records: Vec<PriceSourceRecord>,
        base_unit: UnitId,
    ) -> Result<()> {
        let changes: Vec<UnitsChange> = units
            .into_iter()
            .map(UnitsChange::UpsertUnit)
            .chain(records.into_iter().map(UnitsChange::UpsertPriceSource))
            .collect();
        writer.run(move |pool| async move {
            let mut transaction = pool.begin().await?;
            // A seed's Units may quote one listed after them.
            sqlx::query("PRAGMA defer_foreign_keys = ON")
                .execute(&mut *transaction)
                .await?;
            for change in &changes {
                save(&mut transaction, change).await?;
            }
            seed_base_unit(&mut transaction, base_unit).await?;
            transaction.commit().await?;
            Ok(())
        })
    }

    /// Drops the Units changes not yet committed and loads the rows again, blocking. For after a
    /// [`WriteFailure`](crate::writer::WriteFailure) names [`Domain::Units`], so memory matches
    /// the file again.
    ///
    /// # Errors
    /// Returns the error of the read; memory is left as it was then.
    #[tracing::instrument(name = "Reload the Units", level = "debug", skip_all)]
    pub fn reload(&mut self) -> Result<()> {
        let fresh = Self::load(self.pending.clone())?;
        self.units.rows = fresh.units.rows;
        self.price_sources.rows = fresh.price_sources.rows;
        self.base_unit.id = fresh.base_unit.id;
        self.pending.discard(Domain::Units);
        Ok(())
    }

    pub fn units(&self) -> &dyn UnitRepository {
        &self.units
    }

    pub fn price_sources(&self) -> &dyn PriceSourceRepository {
        &self.price_sources
    }

    pub fn base_unit(&self) -> &dyn BaseUnitRepository {
        &self.base_unit
    }

    /// The three ports, with `usage` from the domains that name Units, for one Interactor call.
    pub fn split_mut<'a>(&'a mut self, usage: &'a dyn UnitUsage) -> PortsMut<'a> {
        PortsMut {
            units: &mut self.units,
            price_sources: &mut self.price_sources,
            base_unit: &mut self.base_unit,
            usage,
        }
    }
}

struct StoredUnits {
    rows: Vec<UnitRow>,
    pending: PendingWrites,
}

impl UnitRepository for StoredUnits {
    fn units(&self) -> Vec<UnitRow> {
        self.rows.clone()
    }

    fn upsert_unit(&mut self, unit: UnitRow) {
        match self.rows.iter_mut().find(|row| row.id == unit.id) {
            Some(row) => *row = unit.clone(),
            None => self.rows.push(unit.clone()),
        }
        self.pending.push(UnitsChange::UpsertUnit(unit));
    }

    fn delete_unit(&mut self, id: UnitId) {
        self.rows.retain(|row| row.id != id);
        self.pending.push(UnitsChange::DeleteUnit(id));
    }
}

struct StoredPriceSources {
    rows: Vec<PriceSourceRecord>,
    pending: PendingWrites,
}

impl PriceSourceRepository for StoredPriceSources {
    fn price_source_records(&self) -> Vec<PriceSourceRecord> {
        self.rows.clone()
    }

    fn upsert_price_source(&mut self, record: PriceSourceRecord) {
        match self.rows.iter_mut().find(|row| row.id() == record.id()) {
            Some(row) => *row = record.clone(),
            None => self.rows.push(record.clone()),
        }
        self.pending.push(UnitsChange::UpsertPriceSource(record));
    }

    fn delete_price_source(&mut self, id: PriceSourceId) {
        self.rows.retain(|row| row.id() != id);
        self.pending.push(UnitsChange::DeletePriceSource(id));
    }
}

struct StoredBaseUnit {
    id: UnitId,
    pending: PendingWrites,
}

impl BaseUnitRepository for StoredBaseUnit {
    fn base_unit(&self) -> UnitId {
        self.id
    }

    fn set_base_unit(&mut self, id: UnitId) {
        self.id = id;
        self.pending.push(UnitsChange::SetBaseUnit(id));
    }
}

impl Change for UnitsChange {
    fn domain(&self) -> Domain {
        Domain::Units
    }

    fn apply<'c>(self: Box<Self>, conn: &'c mut SqliteConnection) -> BoxFuture<'c, Result<()>> {
        Box::pin(async move { save(conn, &self).await })
    }
}

async fn save(conn: &mut SqliteConnection, change: &UnitsChange) -> Result<()> {
    match change {
        UnitsChange::UpsertUnit(unit) => upsert_unit(conn, unit).await,
        UnitsChange::DeleteUnit(id) => {
            sqlx::query!("DELETE FROM units WHERE id = ?", id)
                .execute(conn)
                .await?;
            Ok(())
        }
        UnitsChange::UpsertPriceSource(record) => upsert_price_source(conn, record).await,
        UnitsChange::DeletePriceSource(id) => {
            sqlx::query!("DELETE FROM price_sources WHERE id = ?", id)
                .execute(conn)
                .await?;
            Ok(())
        }
        UnitsChange::SetBaseUnit(id) => {
            sqlx::query!(
                "INSERT INTO ledger (id, base_unit_id) VALUES (1, ?)
                 ON CONFLICT (id) DO UPDATE SET base_unit_id = excluded.base_unit_id",
                id
            )
            .execute(conn)
            .await?;
            Ok(())
        }
    }
}

/// Sets the base Unit only when the Ledger has none, so a re-seed keeps the person's choice.
async fn seed_base_unit(conn: &mut SqliteConnection, id: UnitId) -> Result<()> {
    sqlx::query!(
        "INSERT INTO ledger (id, base_unit_id) VALUES (1, ?) ON CONFLICT (id) DO NOTHING",
        id
    )
    .execute(conn)
    .await?;
    Ok(())
}

async fn upsert_unit(conn: &mut SqliteConnection, unit: &UnitRow) -> Result<()> {
    let price = unit.last_price.as_ref().map(|last| &last.price);
    let fetched_at = unit.last_price.as_ref().map(|last| last.fetched_at);
    sqlx::query!(
        "INSERT INTO units (id, code, name, unit_kind, price_source_id, price_symbol,
                            quote_unit_id, last_price, last_price_fetched_on, is_active)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT (id) DO UPDATE SET
             code = excluded.code,
             name = excluded.name,
             unit_kind = excluded.unit_kind,
             price_source_id = excluded.price_source_id,
             price_symbol = excluded.price_symbol,
             quote_unit_id = excluded.quote_unit_id,
             last_price = excluded.last_price,
             last_price_fetched_on = excluded.last_price_fetched_on,
             is_active = excluded.is_active",
        unit.id,
        unit.code,
        unit.name,
        unit.kind,
        unit.link.source_id,
        unit.link.symbol,
        unit.link.quote,
        price,
        fetched_at,
        unit.is_active,
    )
    .execute(conn)
    .await?;
    Ok(())
}

/// A Price Source row's columns. A built-in's endpoint is code, so only a Custom source fills
/// the endpoint columns.
struct SourceColumns<'a> {
    id: PriceSourceId,
    origin: &'static str,
    name: &'a str,
    updates: &'static str,
    endpoint: Option<&'a Endpoint>,
}

async fn upsert_price_source(
    conn: &mut SqliteConnection,
    record: &PriceSourceRecord,
) -> Result<()> {
    let row = match record {
        PriceSourceRecord::BuiltIn(source) => SourceColumns {
            id: source.id,
            origin: BUILT_IN,
            name: &source.name,
            updates: updates_str(source.updates),
            endpoint: None,
        },
        PriceSourceRecord::Custom(source) => SourceColumns {
            id: source.id,
            origin: CUSTOM,
            name: &source.name,
            updates: updates_str(source.updates),
            endpoint: Some(&source.endpoint),
        },
    };
    let (key_placement, key_name) = match row.endpoint.map(|endpoint| &endpoint.auth) {
        Some(PriceSourceAuth::ApiKey { placement, name }) => {
            (Some(placement_str(*placement)), Some(name.as_ref()))
        }
        Some(PriceSourceAuth::None) | None => (None, None),
    };
    let address = row.endpoint.map(|endpoint| endpoint.address.as_ref());
    let request = row.endpoint.map(|endpoint| endpoint.request.as_ref());
    let price_path = row.endpoint.map(|endpoint| endpoint.price_path.as_ref());
    sqlx::query!(
        "INSERT INTO price_sources (id, origin, name, updates, address, request, price_path,
                                    key_placement, key_name)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT (id) DO UPDATE SET
             origin = excluded.origin,
             name = excluded.name,
             updates = excluded.updates,
             address = excluded.address,
             request = excluded.request,
             price_path = excluded.price_path,
             key_placement = excluded.key_placement,
             key_name = excluded.key_name",
        row.id,
        row.origin,
        row.name,
        row.updates,
        address,
        request,
        price_path,
        key_placement,
        key_name,
    )
    .execute(conn)
    .await?;
    Ok(())
}

/// Every row the adapter holds, as read.
struct Rows {
    units: Vec<UnitRow>,
    records: Vec<PriceSourceRecord>,
    base_unit: UnitId,
}

async fn read(conn: &mut SqliteConnection) -> Result<Rows> {
    let units = sqlx::query!(
        r#"SELECT
               id                    AS "id!: UnitId",
               code,
               name,
               unit_kind             AS "unit_kind!: UnitKind",
               price_source_id       AS "price_source_id!: PriceSourceId",
               price_symbol,
               quote_unit_id         AS "quote_unit_id!: UnitId",
               last_price            AS "last_price?: Money",
               last_price_fetched_on AS "last_price_fetched_on?: DateTime<Utc>",
               is_active
           FROM units"#
    )
    .fetch_all(&mut *conn)
    .await?
    .into_iter()
    .map(|row| UnitRow {
        id: row.id,
        code: row.code,
        name: row.name,
        kind: row.unit_kind,
        link: PriceLink {
            source_id: row.price_source_id,
            symbol: row.price_symbol,
            quote: row.quote_unit_id,
        },
        last_price: row
            .last_price
            .zip(row.last_price_fetched_on)
            .map(|(price, fetched_at)| LastPrice { price, fetched_at }),
        is_active: row.is_active,
    })
    .collect();

    let records = sqlx::query!(
        r#"SELECT
               id AS "id!: PriceSourceId",
               origin,
               name,
               updates,
               address,
               request,
               price_path,
               key_placement,
               key_name
           FROM price_sources"#
    )
    .fetch_all(&mut *conn)
    .await?
    .into_iter()
    .map(|row| {
        let updates = parse_updates(&row.updates)?;
        if row.origin == BUILT_IN {
            return Ok(PriceSourceRecord::BuiltIn(BuiltInOverride {
                id: row.id,
                name: row.name,
                updates,
            }));
        }
        let auth = match (row.key_placement, row.key_name) {
            (Some(placement), Some(name)) => PriceSourceAuth::ApiKey {
                placement: parse_placement(&placement)?,
                name: name.into(),
            },
            _ => PriceSourceAuth::None,
        };
        let missing =
            || Error::Validation(format!("Custom Price Source {} has no endpoint", row.id));
        Ok(PriceSourceRecord::Custom(CustomSource {
            id: row.id,
            name: row.name,
            endpoint: Endpoint {
                address: row.address.ok_or_else(missing)?.into(),
                request: row.request.ok_or_else(missing)?.into(),
                price_path: row.price_path.ok_or_else(missing)?.into(),
                auth,
            },
            updates,
        }))
    })
    .collect::<Result<_>>()?;

    let base_unit = sqlx::query_scalar!(
        r#"SELECT base_unit_id AS "base_unit_id!: UnitId" FROM ledger WHERE id = 1"#
    )
    .fetch_optional(&mut *conn)
    .await?
    .ok_or_else(|| Error::NotFound("the Ledger has no base Unit; seed it first".into()))?;

    Ok(Rows {
        units,
        records,
        base_unit,
    })
}

const BUILT_IN: &str = "built_in";
const CUSTOM: &str = "custom";

fn updates_str(updates: Updates) -> &'static str {
    match updates {
        Updates::Every15Minutes => "every_15_minutes",
        Updates::Hourly => "hourly",
        Updates::Daily => "daily",
        Updates::ManualOnly => "manual_only",
    }
}

fn parse_updates(text: &str) -> Result<Updates> {
    match text {
        "every_15_minutes" => Ok(Updates::Every15Minutes),
        "hourly" => Ok(Updates::Hourly),
        "daily" => Ok(Updates::Daily),
        "manual_only" => Ok(Updates::ManualOnly),
        other => Err(Error::Validation(format!(
            "Unknown Price Source update interval {other}"
        ))),
    }
}

fn placement_str(placement: KeyPlacement) -> &'static str {
    match placement {
        KeyPlacement::Param => "param",
        KeyPlacement::Header => "header",
    }
}

fn parse_placement(text: &str) -> Result<KeyPlacement> {
    match text {
        "param" => Ok(KeyPlacement::Param),
        "header" => Ok(KeyPlacement::Header),
        other => Err(Error::Validation(format!(
            "Unknown API key placement {other}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;

    use chrono::Utc;
    use lib_units::contract::{self, PortsFactory, StoredPorts, snapshot};
    use lib_units::fake::InMemoryPorts;
    use lib_units::interactor::AddUnit;
    use sqlx::SqlitePool;
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

    use super::*;
    use crate::commit::Commit;
    use crate::writer::{Gate, WriteFailure};

    impl StoredPorts for UnitsAdapter {
        fn split_mut<'a>(&'a mut self, usage: &'a dyn UnitUsage) -> PortsMut<'a> {
            UnitsAdapter::split_mut(self, usage)
        }
    }

    /// Adapters on one test database, sharing one pending batch. A reload commits, loads afresh,
    /// and fails the check if the writer reported a failure, so a write SQLite refuses can't pass
    /// unseen.
    struct AdapterFactory {
        writer: Writer,
        pending: PendingWrites,
        failures: mpsc::Receiver<WriteFailure>,
    }

    impl AdapterFactory {
        fn new(options: SqliteConnectOptions) -> Self {
            let (writer, failures) = Writer::spawn(options).expect("the test database opens");
            Self {
                pending: PendingWrites::new(writer.clone()),
                writer,
                failures,
            }
        }
    }

    impl PortsFactory for AdapterFactory {
        type Ports = UnitsAdapter;

        fn create(&mut self, seed: InMemoryPorts) -> UnitsAdapter {
            UnitsAdapter::seed_from(&self.writer, seed).expect("the seed is written");
            UnitsAdapter::load(self.pending.clone()).expect("the seed loads")
        }

        fn reload(&mut self, _ports: UnitsAdapter) -> UnitsAdapter {
            self.pending.commit();
            let fresh = UnitsAdapter::load(self.pending.clone()).expect("the rows load");
            if let Ok(failure) = self.failures.try_recv() {
                panic!("a write failed: {failure:?}");
            }
            fresh
        }

        fn commit(&mut self, _ports: &mut UnitsAdapter) {
            self.pending.commit();
        }

        fn break_writes(&mut self, _ports: &mut UnitsAdapter) {
            self.writer
                .run(|pool| async move {
                    sqlx::query(
                        "CREATE TRIGGER refuse_units BEFORE INSERT ON units
                         BEGIN SELECT RAISE(ABORT, 'writes are refused'); END",
                    )
                    .execute(&pool)
                    .await?;
                    Ok(())
                })
                .expect("the trigger is created");
        }

        fn recover(&mut self, mut ports: UnitsAdapter) -> (UnitsAdapter, usize) {
            self.writer.flush().expect("the writer is running");
            let mut failures = 0;
            while let Ok(failure) = self.failures.try_recv() {
                if failure.domains == [Domain::Units] {
                    failures += 1;
                }
            }
            ports.reload().expect("the rows load");
            (ports, failures)
        }
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn written_rows_reload_the_same(_: SqlitePoolOptions, options: SqliteConnectOptions) {
        contract::written_rows_reload_the_same(&mut AdapterFactory::new(options));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn deleting_the_base_unit_with_a_move_lands_in_order(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        contract::deleting_the_base_unit_with_a_move_lands_in_order(&mut AdapterFactory::new(
            options,
        ));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn deleting_a_custom_source_moves_its_units_to_manual_entry(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        contract::deleting_a_custom_source_moves_its_units_to_manual_entry(
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

    /// The contract covers one refused write; what it can't reach is the adapter's writer
    /// dropping the writes queued behind the failed one, held back here by a gate.
    #[sqlx::test(migrations = "migrations/client")]
    async fn a_failed_write_drops_the_queued_writes_and_the_adapter_reloads(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        let mut factory = AdapterFactory::new(options);
        let mut adapter = factory.create(InMemoryPorts::stub(Utc::now()));
        let stored = snapshot(&mut adapter);
        let pending = factory.pending.clone();
        let add = |adapter: &mut UnitsAdapter, code: &str| {
            let usage = lib_units::fake::InMemoryUnitUsage::default();
            let ports = adapter.split_mut(&usage);
            AddUnit {
                code: code.into(),
                name: code.into(),
                kind: UnitKind::Currency,
                link: PriceLink::manual(UnitId::SEED_AUD),
            }
            .run(ports.units, ports.price_sources)
            .expect("Manual entry exists");
            pending.commit();
        };

        let (gate, release) = Gate::batch();
        factory.writer.write(gate);
        // A second aud breaks units.code's UNIQUE; eur queues behind it.
        add(&mut adapter, "aud");
        add(&mut adapter, "eur");
        assert_eq!(
            adapter.units().units().len(),
            5,
            "memory runs ahead of the file"
        );
        release.send(()).expect("the gate is waiting");

        let failure = factory.failures.recv().expect("the failure is reported");
        assert_eq!(failure.domains, [Domain::Units]);
        assert!(matches!(failure.error, Error::Sqlx(_)));
        adapter.reload().expect("the rows load");
        assert_eq!(snapshot(&mut adapter), stored, "both writes are gone");
        let mut fresh = UnitsAdapter::load(factory.pending.clone()).expect("the rows load");
        assert_eq!(snapshot(&mut fresh), stored, "the queued eur was dropped");
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn loading_an_empty_database_asks_for_a_seed(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        let (writer, _failures) = Writer::spawn(options).unwrap();
        assert!(matches!(
            UnitsAdapter::load(PendingWrites::new(writer)).map(|_| ()),
            Err(Error::NotFound(_))
        ));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn no_table_stores_an_api_key(pool: SqlitePool) {
        let columns: Vec<(String, String)> = sqlx::query_as(
            "SELECT m.name, p.name FROM sqlite_master m, pragma_table_info(m.name) p
             WHERE m.type = 'table'",
        )
        .fetch_all(&pool)
        .await
        .unwrap();
        for (table, column) in &columns {
            let column = column.to_lowercase();
            assert!(
                !(column.contains("api_key") || column.contains("secret") || column == "key"),
                "{table}.{column} looks like it holds a key"
            );
        }
        let price_sources: Vec<&str> = columns
            .iter()
            .filter(|(table, _)| table == "price_sources")
            .map(|(_, column)| column.as_str())
            .collect();
        assert_eq!(
            price_sources,
            [
                "id",
                "origin",
                "name",
                "updates",
                "address",
                "request",
                "price_path",
                "key_placement",
                "key_name",
                "created_on",
                "updated_on"
            ],
            "only where a key goes and what it is called"
        );
    }
}
