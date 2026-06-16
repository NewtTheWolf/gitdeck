//! End-to-end test of the proto ↔ service ↔ store stack for the LOCAL RPCs
//! (todos + boards), which hit only the sqlite Store — no network needed.
//!
//! Spins up the real tonic server on an ephemeral TCP port and drives it via the
//! generated client over a channel. Proves: request decode → TaskService call →
//! store roundtrip → response encode, across the wire.

use std::sync::Arc;

use newt_todo_api::gitdeck_server;
use newt_todo_api::proto::gitdeck_client::GitdeckClient;
use newt_todo_api::proto::{
    BoardIdRequest, CreateBoardRequest, CreateColumnRequest, CreateTaskRequest, ListTasksRequest,
};
use newt_todo_auth::{MemoryTokenStore, TokenStore};
use newt_todo_core::Store;
use newt_todo_service::TaskService;
use tokio::net::TcpListener;
use tonic::transport::Server;

async fn start_server() -> String {
    let store = Store::connect("sqlite::memory:").await.unwrap();
    let service = Arc::new(TaskService::new(store));
    let token_store: Arc<dyn TokenStore> = Arc::new(MemoryTokenStore::new());

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let incoming = tokio_stream::wrappers::TcpListenerStream::new(listener);

    tokio::spawn(async move {
        Server::builder()
            .add_service(gitdeck_server(service, token_store, None))
            .serve_with_incoming(incoming)
            .await
            .unwrap();
    });

    format!("http://{addr}")
}

#[tokio::test]
async fn create_then_list_todo_over_grpc() {
    let url = start_server().await;
    let mut client = GitdeckClient::connect(url).await.unwrap();

    let created = client
        .create_task(CreateTaskRequest {
            title: "write grpc test".into(),
            body: Some("body".into()),
            labels: vec!["dev".into()],
            due_at: None,
        })
        .await
        .unwrap()
        .into_inner()
        .task
        .unwrap();
    assert_eq!(created.title, "write grpc test");
    assert_eq!(created.status, "open");
    assert_eq!(created.labels, vec!["dev".to_string()]);

    let listed = client
        .list_tasks(ListTasksRequest {
            status: None,
            label: None,
            query: None,
        })
        .await
        .unwrap()
        .into_inner()
        .tasks;
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].id, created.id);
}

#[tokio::test]
async fn create_board_with_column_then_get_over_grpc() {
    let url = start_server().await;
    let mut client = GitdeckClient::connect(url).await.unwrap();

    let board = client
        .create_board(CreateBoardRequest { name: "PM".into() })
        .await
        .unwrap()
        .into_inner()
        .board
        .unwrap();
    assert_eq!(board.name, "PM");

    client
        .create_column(CreateColumnRequest {
            board_id: board.id.clone(),
            name: "Todo".into(),
            filter_json: Some(r#"{"state":"open"}"#.into()),
        })
        .await
        .unwrap();

    let fetched = client
        .get_board(BoardIdRequest {
            id: board.id.clone(),
        })
        .await
        .unwrap()
        .into_inner()
        .board
        .unwrap();
    assert_eq!(fetched.columns.len(), 1);
    assert_eq!(fetched.columns[0].name, "Todo");
    assert_eq!(fetched.columns[0].filter_json, r#"{"state":"open"}"#);
}
