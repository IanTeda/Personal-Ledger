//! # Always-On Hub Demo (FC-SYNC-004)
//!
//! `tests/push_pull_sync.rs` (#46) proved two Clients can push/pull through the Sync
//! Server, including one that was offline during the push window. This test proves the
//! stronger "always-on hub" property FC-SYNC-004 asks for: **one** Sync Server process
//! stays running while three independent Clients -- shaped like a real Desktop Client
//! and two real TUI Clients (each its own local SQLite Ledger copy, the same
//! embedded-SQLite pattern `bin-desktop`/`bin-tui`'s own feasibility demos use) --
//! connect, sync, and disconnect in a staggered sequence, with **no two Clients ever
//! online at the same moment**. Every Client still ends up with every other Client's
//! changes, because the durable Change Set log -- not a live relay between
//! simultaneously-connected peers -- is what makes the Sync Server a hub.
//!
//! Scoped like #46/#50: no real system browser, OS keychain, or GUI/TUI binary is
//! touched -- see this ticket's scope discussion. All three Clients authenticate as
//! the same account (ADR-0010's single-account-per-cycle decision maps naturally onto
//! "one self-hoster's own multiple devices").

use std::sync::Arc;

use bin_sync_server::auth;
use lib_core::{HlcClock, RowID};
use lib_database::{DatabaseConfig, DatabaseConnection};
use lib_rpc::{
    ChangeSet as ProtoChangeSet, PullRequest, PushRequest, SyncService, SyncServiceClient,
    SyncServiceServer, UtilitiesService, UtilitiesServiceServer,
};
use secrecy::SecretString;
use sqlx::SqlitePool;
use tonic::transport::Server;

/// Start the Sync Server once, bound to an ephemeral port, and never touch it again
/// except through fresh Client connections -- the whole point being demonstrated.
async fn spawn_sync_server(db_path: &std::path::Path) -> (std::net::SocketAddr, SecretString) {
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

    let signing_key_material = "test-only-signing-key-not-for-production".to_string();
    let interceptor_key = SecretString::from(signing_key_material.clone());
    let test_key = SecretString::from(signing_key_material);

    let sync_service = SyncServiceServer::with_interceptor(
        SyncService::new(Arc::new(pool)),
        auth::interceptor::AuthInterceptor::new(interceptor_key),
    );
    tokio::spawn(async move {
        Server::builder()
            .add_service(UtilitiesServiceServer::new(UtilitiesService::default()))
            .add_service(sync_service)
            .serve_with_incoming(incoming)
            .await
            .expect("Sync Server should serve without error");
    });

    (addr, test_key)
}

fn bearer_request<T>(message: T, access_token: &str) -> tonic::Request<T> {
    let mut request = tonic::Request::new(message);
    let value = format!("Bearer {access_token}")
        .parse()
        .expect("bearer header value should be valid ASCII");
    request.metadata_mut().insert("authorization", value);
    request
}

/// Set up a Client's own local SQLite Ledger copy -- migrated with `lib-database`'s
/// full schema, the same pattern `bin-desktop`/`bin-tui`'s embedded-SQLite demos use.
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

fn new_demo_category(code: &str, name: &str) -> lib_database::Categories {
    let now = chrono::Utc::now();
    lib_database::Categories {
        id: RowID::new(),
        code: code.to_string(),
        name: name.to_string(),
        description: None,
        url_slug: None,
        category_type: lib_core::CategoryTypes::Expense,
        color: None,
        icon: None,
        is_active: true,
        created_on: now,
        updated_on: now,
    }
}

/// Decompose a `Categories` row into its field-level Change Sets -- demo-only stand-in
/// for a future Client write-path's change capture, same as #46's helper.
fn categories_row_to_change_sets(
    category: &lib_database::Categories,
    client_id: RowID,
    clock: &mut HlcClock,
) -> Vec<lib_database::ChangeSet> {
    let fields: Vec<(&str, Option<String>)> = vec![
        ("code", Some(category.code.clone())),
        ("name", Some(category.name.clone())),
        (
            "category_type",
            Some(category.category_type.as_str().to_string()),
        ),
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

/// Apply a pulled Change Set to a Client's local `categories` table -- allowlisted to
/// the columns this demo actually exercises, same shape as #46's helper.
async fn apply_category_field_change(
    pool: &SqlitePool,
    row_id: RowID,
    field_name: &str,
    value: Option<&str>,
) {
    let row_id_str = row_id.to_string();
    let now = chrono::Utc::now().to_rfc3339();

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
        "code" | "name" | "category_type" => field_name,
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
        other => panic!("apply_category_field_change: unsupported field '{other}' for this demo"),
    };

    let sql = format!("UPDATE categories SET {column} = ? WHERE id = ?");
    sqlx::query(&sql)
        .bind(value)
        .bind(&row_id_str)
        .execute(pool)
        .await
        .expect("field update should succeed");
}

/// Push every field of `category`, as `client_id`, over a freshly-connected gRPC
/// channel to `server_addr` -- simulating one app launch's worth of syncing.
async fn push_category_in_a_fresh_session(
    server_addr: std::net::SocketAddr,
    access_token: &str,
    category: &lib_database::Categories,
    client_id: RowID,
    clock: &mut HlcClock,
) {
    let mut client = SyncServiceClient::connect(format!("http://{server_addr}"))
        .await
        .expect("should connect to the running Sync Server");
    let change_sets = categories_row_to_change_sets(category, client_id, clock);
    client
        .push(bearer_request(
            PushRequest {
                change_sets: change_sets.into_iter().map(ProtoChangeSet::from).collect(),
            },
            access_token,
        ))
        .await
        .expect("push should succeed");
}

/// Pull everything since `since_id` over a freshly-connected gRPC channel, apply it to
/// `pool`, and return the highest Change Set `id` seen (the caller's new cursor) --
/// simulating one app launch's worth of syncing.
async fn pull_and_apply_in_a_fresh_session(
    server_addr: std::net::SocketAddr,
    access_token: &str,
    pool: &SqlitePool,
    since_id: Option<String>,
) -> Option<String> {
    let mut client = SyncServiceClient::connect(format!("http://{server_addr}"))
        .await
        .expect("should connect to the running Sync Server");
    let response = client
        .pull(bearer_request(
            PullRequest {
                since_id,
                limit: 100,
            },
            access_token,
        ))
        .await
        .expect("pull should succeed")
        .into_inner();

    let mut last_id = None;
    for change_set in &response.change_sets {
        let row_id: RowID = change_set.row_id.parse().unwrap();
        apply_category_field_change(
            pool,
            row_id,
            &change_set.field_name,
            change_set.value.as_deref(),
        )
        .await;
        last_id = Some(change_set.id.clone());
    }
    last_id
}

#[tokio::test]
async fn sync_server_acts_as_an_always_on_hub_for_staggered_clients() {
    let temp_dir = tempfile::tempdir().expect("should create a scratch tempdir");
    let (server_addr, signing_key) =
        spawn_sync_server(&temp_dir.path().join("sync-server.db")).await;
    let access_token = auth::jwt::issue_access_token(RowID::new(), &signing_key)
        .expect("should be able to issue a test access token");

    // Three Clients, one self-hoster's own devices (ADR-0010's single-account model),
    // each with its own local SQLite Ledger copy and its own stable Client ID.
    let desktop_pool = client_pool(&temp_dir.path().join("desktop.db")).await;
    let tui_a_pool = client_pool(&temp_dir.path().join("tui-a.db")).await;
    let tui_b_pool = client_pool(&temp_dir.path().join("tui-b.db")).await;
    let desktop_id = RowID::new();
    let tui_b_id = RowID::new();
    let mut desktop_clock = HlcClock::new();
    let mut tui_b_clock = HlcClock::new();

    // -- 1. Desktop Client launches, creates a category, pushes it, and quits --
    let groceries = new_demo_category("FOO.BAR.001", "Groceries");
    groceries.insert(&desktop_pool).await.unwrap();
    push_category_in_a_fresh_session(
        server_addr,
        &access_token,
        &groceries,
        desktop_id,
        &mut desktop_clock,
    )
    .await;
    // Desktop already has its own write locally; record where its own pull cursor
    // starts so its later relaunch doesn't re-fetch what it already knows.
    let desktop_cursor =
        pull_and_apply_in_a_fresh_session(server_addr, &access_token, &desktop_pool, None).await;

    // -- 2. TUI Client A launches for the very first time (never online before
    // Desktop's push) and catches up in one pull --
    let tui_a_cursor =
        pull_and_apply_in_a_fresh_session(server_addr, &access_token, &tui_a_pool, None).await;
    assert!(
        lib_database::Categories::find_by_id(groceries.id, &tui_a_pool)
            .await
            .unwrap()
            .is_some(),
        "TUI Client A should have Desktop's category despite never being online with it"
    );

    // -- 3. TUI Client B launches, also catches up on Desktop's category, then
    // creates its own category and pushes it -- then quits --
    pull_and_apply_in_a_fresh_session(server_addr, &access_token, &tui_b_pool, None).await;
    let rent = new_demo_category("FOO.BAR.002", "Rent");
    rent.insert(&tui_b_pool).await.unwrap();
    push_category_in_a_fresh_session(
        server_addr,
        &access_token,
        &rent,
        tui_b_id,
        &mut tui_b_clock,
    )
    .await;

    // -- 4. Desktop Client relaunches (a brand new gRPC connection) and pulls since
    // its own cursor -- Desktop and TUI B were never online at the same time, so this
    // is the durable log doing the relaying, not a live peer-to-peer hop --
    pull_and_apply_in_a_fresh_session(server_addr, &access_token, &desktop_pool, desktop_cursor)
        .await;
    assert!(
        lib_database::Categories::find_by_id(rent.id, &desktop_pool)
            .await
            .unwrap()
            .is_some(),
        "Desktop Client should catch up on TUI Client B's category on relaunch"
    );

    // -- 5. TUI Client A relaunches and pulls since its own cursor -- also never
    // online with TUI Client B --
    pull_and_apply_in_a_fresh_session(server_addr, &access_token, &tui_a_pool, tui_a_cursor).await;
    assert!(
        lib_database::Categories::find_by_id(rent.id, &tui_a_pool)
            .await
            .unwrap()
            .is_some(),
        "TUI Client A should catch up on TUI Client B's category on relaunch"
    );

    // -- Final state: all three Clients converged on both categories, via one
    // continuously-running Sync Server instance, with no two Clients ever connected
    // to it at the same moment --
    for (label, pool) in [
        ("Desktop", &desktop_pool),
        ("TUI A", &tui_a_pool),
        ("TUI B", &tui_b_pool),
    ] {
        let has_groceries = lib_database::Categories::find_by_id(groceries.id, pool)
            .await
            .unwrap()
            .is_some();
        let has_rent = lib_database::Categories::find_by_id(rent.id, pool)
            .await
            .unwrap()
            .is_some();
        assert!(has_groceries, "{label} Client should have Groceries");
        assert!(has_rent, "{label} Client should have Rent");
    }
}
