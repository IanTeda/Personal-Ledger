//! The Inventory adapter: `lib-inventory`'s repository port over the `locations`, `areas` and
//! `items` tables, held as one aggregate (ADR-0046, #866).
//!
//! It loads every Location with its cover and its Areas in tab order, and every Item, into
//! memory, blocking, and serves the rows from there in the order they were added (rowid order,
//! which an upsert keeps). A port write changes memory at once and joins the [`PendingWrites`]
//! every adapter shares, which the bin commits after each input. Writing a Location rewrites its
//! Areas with their positions, deleting any it no longer lists, so the Items in one must be moved
//! first: Remove Area writes them before the Location, and `items`' keys refuse it otherwise.
//!
//! The port has no Location delete: removing one also drops the Document Links to its Items, so
//! it stays a `lib-ledger` command (Q18, until `lib-reassign`, #813). [`InventoryAdapter::
//! delete_location`] is the write the Desktop's forwarding shim sends it through, taking the
//! Location's Items and Areas with it.

use chrono::NaiveDate;
use lib_core::{AreaId, ItemId, LocationId, Money, UnitId};
use lib_inventory::{Area, Cover, Inventory, InventoryRepository, Item, Location};
use sqlx::SqliteConnection;

use crate::commit::PendingWrites;
use crate::writer::{BoxFuture, Change, Domain, Writer};
use crate::{Error, Result};

/// One write, saved in the order it was made.
#[derive(Debug, Clone)]
enum InventoryChange {
    UpsertLocation(Location),
    DeleteLocation(LocationId),
    UpsertItem(Item),
    DeleteItem(ItemId),
}

/// The Inventory domain, held in memory and saved through the shared writer.
pub struct InventoryAdapter {
    inventory: StoredInventory,
}

/// The rows, with the batch their writes join.
struct StoredInventory {
    rows: Inventory,
    pending: PendingWrites,
}

impl InventoryAdapter {
    /// Loads every Location, Area and Item through `pending`'s writer, blocking until they are
    /// read. The adapter's writes join `pending`. An empty database loads no rows.
    ///
    /// # Errors
    /// Returns the error of the read, or a validation error for a row the schema should have
    /// refused.
    #[tracing::instrument(name = "Load the Inventory", level = "debug", skip_all, err)]
    pub fn load(pending: PendingWrites) -> Result<Self> {
        let rows = pending.writer().run(|pool| async move {
            let mut conn = pool.acquire().await?;
            read(&mut conn).await
        })?;
        Ok(Self {
            inventory: StoredInventory { rows, pending },
        })
    }

    /// Writes `inventory` in one transaction, blocking until it lands: each Location with its
    /// Areas, then each Item. Each is an upsert, so seeding again leaves the same rows. For after
    /// the Units the Locations name.
    ///
    /// # Errors
    /// Returns the error of the write; nothing is written then.
    #[tracing::instrument(name = "Seed the Inventory", level = "debug", skip_all, err)]
    pub fn seed(writer: &Writer, inventory: Inventory) -> Result<()> {
        writer.run(move |pool| async move {
            let mut conn = pool.begin().await?;
            Self::seed_in(&mut conn, &inventory).await?;
            conn.commit().await?;
            Ok(())
        })
    }

    /// [`Self::seed`]'s writes on `conn`, so a seed of several domains commits them in one
    /// transaction (#886).
    ///
    /// # Errors
    /// Returns the error of a write.
    pub(crate) async fn seed_in(conn: &mut SqliteConnection, inventory: &Inventory) -> Result<()> {
        for location in &inventory.locations {
            upsert_location(conn, location).await?;
        }
        for item in &inventory.items {
            upsert_item(conn, item).await?;
        }
        Ok(())
    }

    /// Drops the Inventory changes not yet committed and loads the rows again, blocking. For after
    /// a [`WriteFailure`](crate::writer::WriteFailure) names [`Domain::Inventory`], so memory
    /// matches the file again.
    ///
    /// # Errors
    /// Returns the error of the read; memory is left as it was then.
    #[tracing::instrument(name = "Reload the Inventory", level = "debug", skip_all, err)]
    pub fn reload(&mut self) -> Result<()> {
        let fresh = Self::load(self.inventory.pending.clone())?;
        self.inventory.rows = fresh.inventory.rows;
        self.inventory.pending.discard(Domain::Inventory);
        Ok(())
    }

    /// Removes Location `id` with its Areas and Items: Remove Location's delete, which the
    /// Desktop's forwarding shim sends once the Document Links to its Items are dropped (#813
    /// moves it to `lib-reassign`).
    pub fn delete_location(&mut self, id: LocationId) {
        let rows = &mut self.inventory.rows;
        rows.locations.retain(|row| row.id != id);
        rows.items.retain(|row| row.location != id);
        self.inventory
            .pending
            .push(InventoryChange::DeleteLocation(id));
    }

    pub fn inventory(&self) -> &dyn InventoryRepository {
        &self.inventory
    }

    pub fn inventory_mut(&mut self) -> &mut dyn InventoryRepository {
        &mut self.inventory
    }
}

impl InventoryRepository for StoredInventory {
    fn locations(&self) -> Vec<Location> {
        self.rows.locations.clone()
    }

    fn items(&self) -> Vec<Item> {
        self.rows.items.clone()
    }

    fn upsert_location(&mut self, location: Location) {
        let rows = &mut self.rows.locations;
        match rows.iter_mut().find(|row| row.id == location.id) {
            Some(row) => *row = location.clone(),
            None => rows.push(location.clone()),
        }
        self.pending.push(InventoryChange::UpsertLocation(location));
    }

    fn upsert_item(&mut self, item: Item) {
        let rows = &mut self.rows.items;
        match rows.iter_mut().find(|row| row.id == item.id) {
            Some(row) => *row = item.clone(),
            None => rows.push(item.clone()),
        }
        self.pending.push(InventoryChange::UpsertItem(item));
    }

    fn delete_item(&mut self, id: ItemId) {
        self.rows.items.retain(|row| row.id != id);
        self.pending.push(InventoryChange::DeleteItem(id));
    }
}

impl Change for InventoryChange {
    fn domain(&self) -> Domain {
        Domain::Inventory
    }

    fn apply<'c>(self: Box<Self>, conn: &'c mut SqliteConnection) -> BoxFuture<'c, Result<()>> {
        Box::pin(async move {
            match *self {
                Self::UpsertLocation(location) => upsert_location(conn, &location).await,
                Self::DeleteLocation(id) => delete_location(conn, id).await,
                Self::UpsertItem(item) => upsert_item(conn, &item).await,
                Self::DeleteItem(id) => delete_item(conn, id).await,
            }
        })
    }
}

/// Writes the Location's row with its cover, deletes the Areas it no longer lists, then writes
/// each Area it lists at its position.
async fn upsert_location(conn: &mut SqliteConnection, location: &Location) -> Result<()> {
    let cover = location.cover.as_ref();
    let insurer = cover.map(|c| c.insurer.as_str());
    let policy_no = cover.and_then(|c| c.policy_no.as_deref());
    let renews_on = cover.and_then(|c| c.renews_on);
    let sum_insured = cover.map(|c| &c.sum_insured);
    let item_limit = cover.and_then(|c| c.item_limit.as_ref());
    sqlx::query!(
        "INSERT INTO locations (id, name, address, unit_id, cover_insurer, cover_policy_no,
                                cover_renews_on, cover_sum_insured, cover_item_limit)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT (id) DO UPDATE SET
             name = excluded.name,
             address = excluded.address,
             unit_id = excluded.unit_id,
             cover_insurer = excluded.cover_insurer,
             cover_policy_no = excluded.cover_policy_no,
             cover_renews_on = excluded.cover_renews_on,
             cover_sum_insured = excluded.cover_sum_insured,
             cover_item_limit = excluded.cover_item_limit",
        location.id,
        location.name,
        location.address,
        location.unit,
        insurer,
        policy_no,
        renews_on,
        sum_insured,
        item_limit,
    )
    .execute(&mut *conn)
    .await?;

    let stored = sqlx::query_scalar!(
        r#"SELECT id AS "id!: AreaId" FROM areas WHERE location_id = ?"#,
        location.id,
    )
    .fetch_all(&mut *conn)
    .await?;
    for gone in stored
        .into_iter()
        .filter(|id| !location.areas.iter().any(|area| area.id == *id))
    {
        sqlx::query!("DELETE FROM areas WHERE id = ?", gone)
            .execute(&mut *conn)
            .await?;
    }
    for (position, area) in location.areas.iter().enumerate() {
        let position = i64::try_from(position)
            .map_err(|_| Error::Validation(format!("Area {} is at {position}", area.id)))?;
        sqlx::query!(
            "INSERT INTO areas (id, location_id, name, position)
             VALUES (?, ?, ?, ?)
             ON CONFLICT (id) DO UPDATE SET
                 name = excluded.name,
                 position = excluded.position",
            area.id,
            location.id,
            area.name,
            position,
        )
        .execute(&mut *conn)
        .await?;
    }
    Ok(())
}

/// Deletes the Location's Items, then its Areas, then the Location: the keys refuse any other
/// order.
async fn delete_location(conn: &mut SqliteConnection, id: LocationId) -> Result<()> {
    sqlx::query!("DELETE FROM items WHERE location_id = ?", id)
        .execute(&mut *conn)
        .await?;
    sqlx::query!("DELETE FROM areas WHERE location_id = ?", id)
        .execute(&mut *conn)
        .await?;
    sqlx::query!("DELETE FROM locations WHERE id = ?", id)
        .execute(&mut *conn)
        .await?;
    Ok(())
}

async fn upsert_item(conn: &mut SqliteConnection, item: &Item) -> Result<()> {
    sqlx::query!(
        "INSERT INTO items (id, location_id, area_id, name, replacement)
         VALUES (?, ?, ?, ?, ?)
         ON CONFLICT (id) DO UPDATE SET
             location_id = excluded.location_id,
             area_id = excluded.area_id,
             name = excluded.name,
             replacement = excluded.replacement",
        item.id,
        item.location,
        item.area,
        item.name,
        item.replacement,
    )
    .execute(&mut *conn)
    .await?;
    Ok(())
}

async fn delete_item(conn: &mut SqliteConnection, id: ItemId) -> Result<()> {
    sqlx::query!("DELETE FROM items WHERE id = ?", id)
        .execute(&mut *conn)
        .await?;
    Ok(())
}

async fn read(conn: &mut SqliteConnection) -> Result<Inventory> {
    let mut locations = sqlx::query!(
        r#"SELECT
               id                AS "id!: LocationId",
               name,
               address,
               unit_id           AS "unit_id!: UnitId",
               cover_insurer,
               cover_policy_no,
               cover_renews_on   AS "cover_renews_on?: NaiveDate",
               cover_sum_insured AS "cover_sum_insured?: Money",
               cover_item_limit  AS "cover_item_limit?: Money"
           FROM locations
           ORDER BY rowid"#
    )
    .fetch_all(&mut *conn)
    .await?
    .into_iter()
    .map(|row| {
        let cover = match (row.cover_insurer, row.cover_sum_insured) {
            (None, None) => None,
            (Some(insurer), Some(sum_insured)) => Some(Cover {
                insurer,
                policy_no: row.cover_policy_no,
                renews_on: row.cover_renews_on,
                sum_insured,
                item_limit: row.cover_item_limit,
            }),
            _ => {
                return Err(Error::Validation(format!(
                    "Location {} has an insurer without a sum insured, or one without the other",
                    row.id
                )));
            }
        };
        Ok(Location {
            id: row.id,
            name: row.name,
            address: row.address,
            unit: row.unit_id,
            cover,
            areas: Vec::new(),
        })
    })
    .collect::<Result<Vec<_>>>()?;

    let areas = sqlx::query!(
        r#"SELECT id AS "id!: AreaId", location_id AS "location_id!: LocationId", name
           FROM areas
           ORDER BY position, rowid"#
    )
    .fetch_all(&mut *conn)
    .await?;
    for area in areas {
        let location = locations
            .iter_mut()
            .find(|location| location.id == area.location_id)
            .ok_or_else(|| {
                Error::Validation(format!(
                    "Area {} names Location {}, which isn't stored",
                    area.id, area.location_id
                ))
            })?;
        location.areas.push(Area {
            id: area.id,
            name: area.name,
        });
    }

    let items = sqlx::query!(
        r#"SELECT
               id          AS "id!: ItemId",
               location_id AS "location_id!: LocationId",
               area_id     AS "area_id!: AreaId",
               name,
               replacement AS "replacement!: Money"
           FROM items
           ORDER BY rowid"#
    )
    .fetch_all(&mut *conn)
    .await?
    .into_iter()
    .map(|row| Item {
        id: row.id,
        location: row.location_id,
        area: row.area_id,
        name: row.name,
        replacement: row.replacement,
    })
    .collect();

    Ok(Inventory { locations, items })
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;

    use lib_inventory::contract::{self, PortsFactory};
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

    use super::*;
    use crate::commit::Commit;
    use crate::writer::WriteFailure;

    /// The contract runs on the adapter as its port.
    impl InventoryRepository for InventoryAdapter {
        fn locations(&self) -> Vec<Location> {
            self.inventory.locations()
        }

        fn items(&self) -> Vec<Item> {
            self.inventory.items()
        }

        fn upsert_location(&mut self, location: Location) {
            self.inventory.upsert_location(location);
        }

        fn upsert_item(&mut self, item: Item) {
            self.inventory.upsert_item(item);
        }

        fn delete_item(&mut self, id: ItemId) {
            self.inventory.delete_item(id);
        }
    }

    /// The adapter on one test database, with its pending batch. The stub's Units are stored
    /// first, which a Location names. A reload commits, loads afresh, and fails the check if the
    /// writer reported a failure, so a write SQLite refuses can't pass unseen.
    struct AdapterFactory {
        writer: Writer,
        pending: PendingWrites,
        failures: mpsc::Receiver<WriteFailure>,
    }

    impl AdapterFactory {
        fn new(options: SqliteConnectOptions) -> Self {
            let (writer, failures) = Writer::spawn(options).expect("the test database opens");
            crate::test_support::store_stub_references(&writer);
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

        /// Commits the writes made so far and returns how many failed, each naming the Inventory
        /// alone.
        fn commit_and_count_failures(&self) -> usize {
            self.pending.commit();
            let failures = self.failures();
            assert!(
                failures
                    .iter()
                    .all(|failure| failure.domains == [Domain::Inventory])
            );
            failures.len()
        }
    }

    impl PortsFactory for AdapterFactory {
        type Port = InventoryAdapter;

        fn create(&mut self, seed: Inventory) -> InventoryAdapter {
            InventoryAdapter::seed(&self.writer, seed).expect("the seed is written");
            InventoryAdapter::load(self.pending.clone()).expect("the seed loads")
        }

        fn reload(&mut self, _port: InventoryAdapter) -> InventoryAdapter {
            self.pending.commit();
            let fresh = InventoryAdapter::load(self.pending.clone()).expect("the rows load");
            if let Some(failure) = self.failures().first() {
                panic!("a write failed: {failure:?}");
            }
            fresh
        }

        fn commit(&mut self, _port: &mut InventoryAdapter) {
            self.pending.commit();
        }

        fn break_writes(&mut self, _port: &mut InventoryAdapter) {
            self.writer
                .run(|pool| async move {
                    sqlx::query(
                        "CREATE TRIGGER refuse_locations BEFORE INSERT ON locations
                         BEGIN SELECT RAISE(ABORT, 'writes are refused'); END",
                    )
                    .execute(&pool)
                    .await?;
                    Ok(())
                })
                .expect("the trigger is created");
        }

        fn recover(&mut self, mut port: InventoryAdapter) -> (InventoryAdapter, usize) {
            let failures = self
                .failures()
                .iter()
                .filter(|failure| failure.domains == [Domain::Inventory])
                .count();
            port.reload().expect("the rows load");
            (port, failures)
        }
    }

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, 2).expect("a real date")
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn a_location_with_cover_reloads_the_same(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        contract::a_location_with_cover_reloads_the_same(&mut AdapterFactory::new(options));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn an_edited_location_reloads_the_same(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        contract::an_edited_location_reloads_the_same(&mut AdapterFactory::new(options));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn areas_reload_in_order(_: SqlitePoolOptions, options: SqliteConnectOptions) {
        contract::areas_reload_in_order(&mut AdapterFactory::new(options));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn items_reload_the_same(_: SqlitePoolOptions, options: SqliteConnectOptions) {
        contract::items_reload_the_same(&mut AdapterFactory::new(options));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn remove_area_moves_its_items(_: SqlitePoolOptions, options: SqliteConnectOptions) {
        contract::remove_area_moves_its_items(&mut AdapterFactory::new(options));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn an_empty_area_is_removed_alone(_: SqlitePoolOptions, options: SqliteConnectOptions) {
        contract::an_empty_area_is_removed_alone(&mut AdapterFactory::new(options));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn a_failed_write_is_reported_and_rolls_back(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        contract::a_failed_write_is_reported_and_rolls_back(&mut AdapterFactory::new(options));
    }

    /// The file refuses a Location on a Unit that isn't stored, an Item in an Area of another
    /// Location, and dropping an Area that still holds Items, which backstop the Interactors' own
    /// refusals.
    #[sqlx::test(migrations = "migrations/client")]
    async fn the_file_refuses_rows_naming_what_isnt_there_or_isnt_theirs(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        let mut factory = AdapterFactory::new(options);
        let mut port = factory.create(lib_inventory::default_inventory(today()));
        let stub = port.inventory().inventory();
        let (elm, storage) = (stub.locations[0].clone(), stub.locations[1].clone());

        port.upsert_location(Location {
            id: LocationId::new(),
            name: "Nowhere".into(),
            unit: UnitId::new(),
            ..storage.clone()
        });
        assert_eq!(factory.commit_and_count_failures(), 1, "no such Unit");
        port.reload().expect("the rows load");

        let item = stub.items.iter().find(|i| i.location == elm.id).cloned();
        let item = item.expect("the stub's Elm St holds Items");
        port.upsert_item(Item {
            area: storage.areas[0].id,
            ..item.clone()
        });
        assert_eq!(
            factory.commit_and_count_failures(),
            1,
            "the Area is another Location's"
        );
        port.reload().expect("the rows load");

        let holding = elm
            .areas
            .iter()
            .find(|area| area.id == item.area)
            .expect("the Item's Area is Elm St's");
        port.upsert_location(Location {
            areas: elm
                .areas
                .iter()
                .filter(|area| area.id != holding.id)
                .cloned()
                .collect(),
            ..elm.clone()
        });
        assert_eq!(
            factory.commit_and_count_failures(),
            1,
            "an Area holding Items can't go"
        );
        port.reload().expect("the rows load");
        assert_eq!(
            port.inventory().inventory(),
            stub,
            "every refusal rolled back"
        );
    }

    /// Removing a Location takes its Items and Areas with it and leaves the other Locations as
    /// they were.
    #[sqlx::test(migrations = "migrations/client")]
    async fn deleting_a_location_takes_its_areas_and_items(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        let mut factory = AdapterFactory::new(options);
        let mut port = factory.create(lib_inventory::default_inventory(today()));
        let stub = port.inventory().inventory();
        let elm = stub.locations[0].id;
        assert!(stub.location_items(elm) > 0);

        port.delete_location(elm);
        assert_eq!(factory.commit_and_count_failures(), 0);
        let port = factory.reload(port);

        let after = port.inventory().inventory();
        assert!(after.location(elm).is_none());
        assert_eq!(after.location_items(elm), 0);
        assert_eq!(after.locations, stub.locations[1..]);
        assert_eq!(
            after.items,
            stub.items
                .into_iter()
                .filter(|item| item.location != elm)
                .collect::<Vec<_>>()
        );
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn an_empty_database_loads_no_inventory(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        let (writer, _failures) = Writer::spawn(options).expect("the test database opens");
        let adapter = InventoryAdapter::load(PendingWrites::new(writer)).expect("it loads");
        assert_eq!(adapter.inventory().inventory(), Inventory::default());
    }
}
