//! # Push/Pull Sync Demo (FC-SYNC-003)
//!
//! Proves the Sync Server's basic push/pull mechanism end-to-end, through real gRPC
//! calls over a real socket and real SQLite files -- not mocks -- mirroring how
//! [Scaffold bin-sync-server](https://github.com/IanTeda/Personal-Ledger/issues/41)
//! was verified with `grpcurl`.
//!
//! Two independent local SQLite Clients ("A" and "B") each hold their own full copy of
//! the `categories` table, the same "Client holds its own local Ledger copy" pattern
//! `bin-desktop`'s embedded-SQLite demo established. Client A makes local edits and
//! pushes them as Change Sets while Client B is offline; Client B then connects for the
//! first time and pulls everything queued in one batch -- proving offline catch-up, not
//! just online-online sync -- and applies the Change Sets to its own local table.
//! Finally, the Sync Server's Change Set log is reopened from a fresh connection to
//! prove it is a durable store, not in-memory state (ADR-0009).
//!
//! Scoped to this ticket's Non-goals: Change Set construction/application here is a
//! demo-only helper limited to the `categories` table's known columns, not a generic
//! change-data-capture or arbitrary-table sync engine. Auth (ADR-0010) is out of scope
//! -- these gRPC calls are unauthenticated, per issue #46/#50's ticket boundary.

use std::sync::Arc;

use lib_database::{ChangeSet, DatabaseConfig, DatabaseConnection};
use lib_domain::{HlcClock, RowID};
use lib_rpc::{
    ChangeSet as ProtoChangeSet, PullRequest, PushRequest, SyncService, SyncServiceClient,
    SyncServiceServer, UtilitiesService, UtilitiesServiceServer,
};
use sqlx::SqlitePool;
use tonic::transport::Server;

/// Start the Sync Server (both `SyncService` and `UtilitiesService`, matching
/// `main.rs`) bound to an OS-assigned ephemeral port, backed by a fresh SQLite file at
/// `db_path`. Returns the bound address; the server runs in a background task for the
/// lifetime of the test process.
async fn spawn_sync_server(db_path: &std::path::Path) -> std::net::SocketAddr {
    let connection = DatabaseConnection::new(DatabaseConfig {
        url: format!("sqlite://{}?mode=rwc", db_path.display()),
        ..DatabaseConfig::default()
    })
    .await
    .expect("Sync Server database connection should establish");
    let pool = connection.into_pool();
    sqlx::migrate!("../../libs/lib-database/migrations")
        .run(&pool)
        .await
        .expect("Sync Server migrations should apply");

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("should bind an ephemeral port");
    let addr = listener
        .local_addr()
        .expect("bound listener has a local address");
    let incoming = tokio_stream::wrappers::TcpListenerStream::new(listener);

    let sync_service = SyncService::new(Arc::new(pool));
    tokio::spawn(async move {
        Server::builder()
            .add_service(UtilitiesServiceServer::new(UtilitiesService::default()))
            .add_service(SyncServiceServer::new(sync_service))
            .serve_with_incoming(incoming)
            .await
            .expect("Sync Server should serve without error");
    });

    addr
}

/// Set up a Client's own local SQLite Ledger copy -- migrated with `lib-database`'s
/// full schema, same as `bin-desktop`'s embedded-SQLite demo.
async fn client_pool(db_path: &std::path::Path) -> SqlitePool {
    let connection = DatabaseConnection::new(DatabaseConfig {
        url: format!("sqlite://{}?mode=rwc", db_path.display()),
        ..DatabaseConfig::default()
    })
    .await
    .expect("Client database connection should establish");
    let pool = connection.into_pool();
    sqlx::migrate!("../../libs/lib-database/migrations")
        .run(&pool)
        .await
        .expect("Client migrations should apply");
    pool
}

/// Decompose a `Categories` row into its field-level Change Sets -- the demo-only stand-in
/// for a future Client write-path's change capture (out of scope here; see the ticket's
/// Non-goals). Every field is stringified explicitly rather than attempting a generic
/// to-string conversion, since the receiving side's `apply_category_field_change` must
/// parse each one back with the matching logic.
fn categories_row_to_change_sets(
    category: &lib_database::Categories,
    client_id: RowID,
    clock: &mut HlcClock,
) -> Vec<ChangeSet> {
    let fields: Vec<(&str, Option<String>)> = vec![
        ("code", Some(category.code.clone())),
        ("name", Some(category.name.clone())),
        ("description", category.description.clone()),
        (
            "url_slug",
            category.url_slug.as_ref().map(|s| s.to_string()),
        ),
        (
            "category_type",
            Some(category.category_type.as_str().to_string()),
        ),
        ("color", category.color.as_ref().map(|c| c.to_string())),
        ("icon", category.icon.clone()),
        ("is_active", Some(category.is_active.to_string())),
    ];

    fields
        .into_iter()
        .map(|(field_name, value)| {
            lib_database::change_sets::ChangeSetBuilder::new()
                .with_id(RowID::new())
                .with_table_name("categories")
                .with_row_id(category.id)
                .with_field_name(field_name)
                .with_value_opt(value)
                .with_hlc(clock.tick())
                .with_client_id(client_id)
                .build()
                .expect("demo Change Set should always build")
        })
        .collect()
}

/// Apply a pulled Change Set to a Client's local `categories` table. Allowlisted to
/// `categories`' known columns only -- not a generic arbitrary-table sync engine (see
/// the ticket's Non-goals) -- using runtime-bound queries (not `sqlx::query!`) so this
/// test crate doesn't need its own `.sqlx` offline cache.
async fn apply_category_field_change(
    pool: &SqlitePool,
    row_id: RowID,
    field_name: &str,
    value: Option<&str>,
) {
    let row_id_str = row_id.to_string();
    let now = chrono::Utc::now().to_rfc3339();

    // Materialise a placeholder row on first sight of this row_id, satisfying the
    // `categories` table's NOT NULL/UNIQUE constraints with values unique-by-construction
    // (the row_id itself) -- the same problem a real Client applying a brand-new row's
    // Change Sets one field at a time would need to solve.
    sqlx::query(
        r#"
            INSERT INTO categories (id, code, name, category_type, is_active, created_on, updated_on)
            VALUES (?, ?, ?, 'expense', TRUE, ?, ?)
            ON CONFLICT(id) DO NOTHING
        "#,
    )
    .bind(&row_id_str)
    .bind(&row_id_str)
    .bind(&row_id_str)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await
    .expect("placeholder row insert should succeed");

    let column = match field_name {
        "code" | "name" | "description" | "url_slug" | "icon" | "color" | "category_type" => {
            field_name
        }
        "is_active" => {
            let is_active = value == Some("true");
            sqlx::query("UPDATE categories SET is_active = ? WHERE id = ?")
                .bind(is_active)
                .bind(&row_id_str)
                .execute(pool)
                .await
                .expect("is_active update should succeed");
            return;
        }
        other => panic!(
            "apply_category_field_change: unsupported field '{other}' -- this demo is scoped to categories' known columns"
        ),
    };

    let sql = format!("UPDATE categories SET {column} = ? WHERE id = ?");
    sqlx::query(&sql)
        .bind(value)
        .bind(&row_id_str)
        .execute(pool)
        .await
        .expect("field update should succeed");
}

/// Build a `Categories` row directly via struct literal -- `CategoriesBuilder` isn't
/// exposed outside `lib_database`, so this mirrors how `bin-desktop`'s own embedded-SQLite
/// demo constructs one.
fn new_demo_category(code: &str, name: &str) -> lib_database::Categories {
    let now = chrono::Utc::now();
    lib_database::Categories {
        id: RowID::new(),
        code: code.to_string(),
        name: name.to_string(),
        description: None,
        url_slug: None,
        category_type: lib_domain::CategoryTypes::Expense,
        color: None,
        icon: None,
        is_active: true,
        created_on: now,
        updated_on: now,
    }
}

#[tokio::test]
async fn push_pull_sync_with_offline_catch_up() {
    let temp_dir = tempfile::tempdir().expect("should create a scratch tempdir");
    let server_addr = spawn_sync_server(&temp_dir.path().join("sync-server.db")).await;
    let client_a_pool = client_pool(&temp_dir.path().join("client-a.db")).await;
    let client_b_pool = client_pool(&temp_dir.path().join("client-b.db")).await;

    let mut client = SyncServiceClient::connect(format!("http://{server_addr}"))
        .await
        .expect("should connect to the running Sync Server");

    let client_a_id = RowID::new();
    let mut client_a_clock = HlcClock::new();

    // -- Client A: create a category locally, and push its Change Sets --
    let category_one = new_demo_category("FOO.BAR.001", "Groceries");
    category_one.insert(&client_a_pool).await.unwrap();

    let change_sets_one =
        categories_row_to_change_sets(&category_one, client_a_id, &mut client_a_clock);
    let push_response = client
        .push(PushRequest {
            change_sets: change_sets_one
                .iter()
                .cloned()
                .map(ProtoChangeSet::from)
                .collect(),
        })
        .await
        .expect("first push should succeed")
        .into_inner();
    assert_eq!(push_response.accepted_count as usize, change_sets_one.len());

    // -- Offline catch-up: Client A creates a second category and pushes more Change
    // Sets while Client B does nothing -- simulating changes queued while B was down --
    let category_two = new_demo_category("FOO.BAR.002", "Rent");
    category_two.insert(&client_a_pool).await.unwrap();

    let change_sets_two =
        categories_row_to_change_sets(&category_two, client_a_id, &mut client_a_clock);
    client
        .push(PushRequest {
            change_sets: change_sets_two
                .iter()
                .cloned()
                .map(ProtoChangeSet::from)
                .collect(),
        })
        .await
        .expect("second push should succeed");

    // -- Client B connects for the first time and pulls everything in one call --
    let pull_response = client
        .pull(PullRequest {
            since_id: None,
            limit: 100,
        })
        .await
        .expect("pull should succeed")
        .into_inner();

    let expected_change_set_count = change_sets_one.len() + change_sets_two.len();
    assert_eq!(
        pull_response.change_sets.len(),
        expected_change_set_count,
        "Client B should catch up on every Change Set queued while it was offline, in one batch"
    );

    // -- Client B applies every pulled Change Set to its own local table --
    for change_set in &pull_response.change_sets {
        let row_id: RowID = change_set.row_id.parse().unwrap();
        apply_category_field_change(
            &client_b_pool,
            row_id,
            &change_set.field_name,
            change_set.value.as_deref(),
        )
        .await;
    }

    // -- Assert: Client B now has both categories, matching Client A's field values --
    let synced_one = lib_database::Categories::find_by_id(category_one.id, &client_b_pool)
        .await
        .unwrap()
        .expect("category one should have synced to Client B");
    assert_eq!(synced_one.code, category_one.code);
    assert_eq!(synced_one.name, category_one.name);
    assert_eq!(synced_one.category_type, category_one.category_type);
    assert!(synced_one.is_active);

    let synced_two = lib_database::Categories::find_by_id(category_two.id, &client_b_pool)
        .await
        .unwrap()
        .expect("category two should have synced to Client B");
    assert_eq!(synced_two.code, category_two.code);
    assert_eq!(synced_two.name, category_two.name);

    // -- Durability: reopen the Sync Server's Change Set log from a fresh connection
    // (not the still-running server's own pool) and confirm the log survived --
    // proving it's a durable store, not in-memory state (ADR-0009) --
    let reopened = DatabaseConnection::new(DatabaseConfig {
        url: format!(
            "sqlite://{}?mode=rwc",
            temp_dir.path().join("sync-server.db").display()
        ),
        ..DatabaseConfig::default()
    })
    .await
    .expect("should reopen the Sync Server's database file");
    let durable_change_sets = ChangeSet::find_since(None, 100, reopened.pool())
        .await
        .unwrap();
    assert_eq!(durable_change_sets.len(), expected_change_set_count);
}
