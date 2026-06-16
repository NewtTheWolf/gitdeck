//! Over-the-wire tests for the Phase K3 cross-device sync RPCs
//! (`PushChanges` / `PullChanges` / `WatchChanges`).
//!
//! - Push a board change from one client, pull it from another, assert the wire
//!   round-trip and that the cursor advances.
//! - Pulling again with that cursor returns nothing (no duplicate).
//! - Watch: subscribe, push from another call, receive the streamed change
//!   (bounded by a timeout). Smoke-level — asserts at least one delivery.

use std::sync::Arc;
use std::time::Duration;

use newt_todo_api::proto::gitdeck_client::GitdeckClient;
use newt_todo_api::proto::{
    Change, PullChangesRequest, PushChangesRequest, WatchChangesRequest,
};
use newt_todo_api::{router_with_auth, AuthSetup, EnvTokenStore};
use newt_todo_auth::TokenStore;
use newt_todo_core::Store;
use newt_todo_service::TaskService;
use tokio::net::TcpListener;

/// Start a single-mode (BC, no auth) server over a shared in-memory store and
/// return its URL. All clients hit the same `LOCAL_USER` scope.
async fn start() -> String {
    let store = Store::connect("sqlite::memory:").await.unwrap();
    let service = Arc::new(TaskService::new(store));
    let token_store: Arc<dyn TokenStore> = Arc::new(EnvTokenStore::new(None));
    let setup = AuthSetup::single(None);

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let incoming = tokio_stream::wrappers::TcpListenerStream::new(listener);

    tokio::spawn(async move {
        router_with_auth(service, token_store, setup)
            .serve_with_incoming(incoming)
            .await
            .unwrap();
    });

    format!("http://{addr}")
}

/// A board change with a fixed id/timestamp.
fn board_change(id: &str, name: &str, updated: &str) -> Change {
    let data = serde_json::json!({
        "name": name,
        "position": 0,
        "created_at": updated,
    })
    .to_string();
    Change {
        kind: "board".into(),
        id: id.into(),
        updated_at: updated.into(),
        deleted: false,
        data_json: data,
        board_id: String::new(),
        column_id: String::new(),
    }
}

#[tokio::test]
async fn push_then_pull_roundtrips_and_cursor_advances() {
    let url = start().await;
    let mut a = GitdeckClient::connect(url.clone()).await.unwrap();
    let mut b = GitdeckClient::connect(url).await.unwrap();

    let id = uuid::Uuid::new_v4().to_string();
    let ts = "2026-06-16T10:00:00Z";

    // Client A pushes a created board.
    let push = a
        .push_changes(PushChangesRequest {
            changes: vec![board_change(&id, "Shared", ts)],
        })
        .await
        .unwrap()
        .into_inner();
    assert_eq!(push.applied, 1);
    assert_eq!(push.server_cursor, ts, "server cursor = applied updated_at");

    // Client B pulls from the beginning and sees it.
    let pull = b
        .pull_changes(PullChangesRequest {
            since_cursor: String::new(),
        })
        .await
        .unwrap()
        .into_inner();
    let board = pull
        .changes
        .iter()
        .find(|c| c.id == id)
        .expect("pulled the pushed board");
    assert_eq!(board.kind, "board");
    assert!(!board.deleted);
    assert_eq!(pull.cursor, ts, "cursor advances to max updated_at");

    // Pulling again from that cursor returns nothing new.
    let pull2 = b
        .pull_changes(PullChangesRequest {
            since_cursor: pull.cursor.clone(),
        })
        .await
        .unwrap()
        .into_inner();
    assert!(
        pull2.changes.iter().all(|c| c.id != id),
        "cursor excludes already-pulled rows"
    );
    // Echoes the cursor when nothing newer.
    assert_eq!(pull2.cursor, ts);
}

#[tokio::test]
async fn push_older_change_is_not_applied() {
    let url = start().await;
    let mut a = GitdeckClient::connect(url).await.unwrap();

    let id = uuid::Uuid::new_v4().to_string();
    // First, newer.
    a.push_changes(PushChangesRequest {
        changes: vec![board_change(&id, "v2", "2026-06-16T12:00:00Z")],
    })
    .await
    .unwrap();
    // Then an OLDER change for the same row → not applied (LWW).
    let push = a
        .push_changes(PushChangesRequest {
            changes: vec![board_change(&id, "v1", "2026-06-16T09:00:00Z")],
        })
        .await
        .unwrap()
        .into_inner();
    assert_eq!(push.applied, 0, "older change ignored by LWW");
}

#[tokio::test]
async fn watch_receives_change_from_another_push() {
    let url = start().await;
    let mut watcher = GitdeckClient::connect(url.clone()).await.unwrap();
    let mut pusher = GitdeckClient::connect(url).await.unwrap();

    // Subscribe first so the broadcast sender exists before the push.
    let mut stream = watcher
        .watch_changes(WatchChangesRequest {})
        .await
        .unwrap()
        .into_inner();

    // Give the subscription a moment to register, then push from another client.
    let id = uuid::Uuid::new_v4().to_string();
    tokio::time::sleep(Duration::from_millis(100)).await;
    pusher
        .push_changes(PushChangesRequest {
            changes: vec![board_change(&id, "Watched", "2026-06-16T10:00:00Z")],
        })
        .await
        .unwrap();

    // Expect the applied change to arrive on the stream within a timeout.
    let received = tokio::time::timeout(Duration::from_secs(3), stream.message())
        .await
        .expect("watch stream did not deliver in time")
        .expect("stream error")
        .expect("stream closed without a message");
    assert_eq!(received.id, id);
    assert_eq!(received.kind, "board");
}
