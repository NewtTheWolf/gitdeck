//! End-to-end tests for the Phase K1 server auth machinery.
//!
//! - register → login returns a JWT the codec accepts; duplicate username and
//!   wrong password are handled.
//! - In `multi` mode a data RPC (list_tasks) is rejected `unauthenticated`
//!   WITHOUT a bearer token, and passes WITH a valid one.
//! - In `single` mode the same RPC works with no token (backwards-compatible).
//! - set_github_token → get_me round-trips the per-user GitHub token flag.

use std::sync::Arc;

use newt_todo_api::proto::gitdeck_client::GitdeckClient;
use newt_todo_api::proto::{
    BoardIdRequest, CreateBoardRequest, Empty, GetMeRequest, ListTasksRequest, LoginRequest,
    RegisterRequest, SetGithubTokenRequest,
};
use newt_todo_api::{
    router_with_auth, AuthSetup, AuthStore, EnvTokenStore, JwtCodec,
};
use newt_todo_auth::TokenStore;
use newt_todo_core::Store;
use newt_todo_service::TaskService;
use tokio::net::TcpListener;
use tonic::metadata::MetadataValue;
use tonic::transport::Channel;
use tonic::Request;

/// Start a server. When `multi` is true, wire the auth store + JWT (auth
/// enforced); otherwise single-user (BC). Returns (url, shared JwtCodec, db_url).
async fn start(multi: bool) -> (String, JwtCodec) {
    // Shared in-memory sqlite: core Store + AuthStore both connect to it. Use a
    // named shared-cache memory db so both connections see the same schema.
    // (Each AuthStore/Store pins max_connections=1 for `:memory:`.)
    let store = Store::connect("sqlite::memory:").await.unwrap();
    let service = Arc::new(TaskService::new(store));
    let token_store: Arc<dyn TokenStore> = Arc::new(EnvTokenStore::new(None));

    let jwt = JwtCodec::new("test-secret");
    let setup = if multi {
        let auth_store = Arc::new(AuthStore::connect("sqlite::memory:").await.unwrap());
        AuthSetup::multi(auth_store, jwt.clone(), None)
    } else {
        AuthSetup::single(None)
    };

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let incoming = tokio_stream::wrappers::TcpListenerStream::new(listener);

    tokio::spawn(async move {
        router_with_auth(service, token_store, setup)
            .serve_with_incoming(incoming)
            .await
            .unwrap();
    });

    (format!("http://{addr}"), jwt)
}

fn with_bearer<T>(mut req: Request<T>, token: &str) -> Request<T> {
    let val: MetadataValue<_> = format!("Bearer {token}").parse().unwrap();
    req.metadata_mut().insert("authorization", val);
    req
}

#[tokio::test]
async fn register_login_returns_verifiable_token() {
    let (url, jwt) = start(true).await;
    let mut client = GitdeckClient::connect(url).await.unwrap();

    let user_id = client
        .register(RegisterRequest {
            username: "alice".into(),
            password: "hunter2".into(),
        })
        .await
        .unwrap()
        .into_inner()
        .user_id;
    assert!(!user_id.is_empty());

    let token = client
        .login(LoginRequest {
            username: "alice".into(),
            password: "hunter2".into(),
        })
        .await
        .unwrap()
        .into_inner()
        .token;
    // The issued session token verifies back to the same user id.
    assert_eq!(jwt.verify(&token).as_deref(), Some(user_id.as_str()));
}

#[tokio::test]
async fn duplicate_username_and_wrong_password() {
    let (url, _jwt) = start(true).await;
    let mut client = GitdeckClient::connect(url).await.unwrap();

    client
        .register(RegisterRequest {
            username: "bob".into(),
            password: "pw".into(),
        })
        .await
        .unwrap();

    // Duplicate username errors.
    let dup = client
        .register(RegisterRequest {
            username: "bob".into(),
            password: "other".into(),
        })
        .await;
    assert!(dup.is_err());

    // Wrong password → unauthenticated, no token.
    let bad = client
        .login(LoginRequest {
            username: "bob".into(),
            password: "nope".into(),
        })
        .await;
    let err = bad.unwrap_err();
    assert_eq!(err.code(), tonic::Code::Unauthenticated);
}

#[tokio::test]
async fn multi_mode_gates_data_rpc() {
    let (url, _jwt) = start(true).await;
    let channel = Channel::from_shared(url).unwrap().connect().await.unwrap();
    let mut client = GitdeckClient::new(channel);

    // No bearer token → rejected.
    let denied = client
        .list_tasks(ListTasksRequest {
            status: None,
            label: None,
            query: None,
        })
        .await;
    assert_eq!(
        denied.unwrap_err().code(),
        tonic::Code::Unauthenticated,
        "data RPC without a token must be unauthenticated in multi mode"
    );

    // Register + login → token → request passes the interceptor.
    client
        .register(RegisterRequest {
            username: "carol".into(),
            password: "pw".into(),
        })
        .await
        .unwrap();
    let token = client
        .login(LoginRequest {
            username: "carol".into(),
            password: "pw".into(),
        })
        .await
        .unwrap()
        .into_inner()
        .token;

    let listed = client
        .list_tasks(with_bearer(
            Request::new(ListTasksRequest {
                status: None,
                label: None,
                query: None,
            }),
            &token,
        ))
        .await
        .unwrap()
        .into_inner()
        .tasks;
    assert!(listed.is_empty());
}

#[tokio::test]
async fn single_mode_data_rpc_works_without_token() {
    let (url, _jwt) = start(false).await;
    let mut client = GitdeckClient::connect(url).await.unwrap();

    // BC: no token needed in single mode.
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
    assert!(listed.is_empty());
}

#[tokio::test]
async fn set_github_token_then_get_me() {
    let (url, _jwt) = start(true).await;
    let mut client = GitdeckClient::connect(url).await.unwrap();

    client
        .register(RegisterRequest {
            username: "dave".into(),
            password: "pw".into(),
        })
        .await
        .unwrap();
    let token = client
        .login(LoginRequest {
            username: "dave".into(),
            password: "pw".into(),
        })
        .await
        .unwrap()
        .into_inner()
        .token;

    // Before: no github token.
    let me = client
        .get_me(with_bearer(Request::new(GetMeRequest {}), &token))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(me.username, "dave");
    assert!(!me.has_github_token);

    // Set it (authenticated).
    client
        .set_github_token(with_bearer(
            Request::new(SetGithubTokenRequest {
                github_token: "gho_xyz".into(),
            }),
            &token,
        ))
        .await
        .unwrap();

    // After: flag flips.
    let me2 = client
        .get_me(with_bearer(Request::new(GetMeRequest {}), &token))
        .await
        .unwrap()
        .into_inner();
    assert!(me2.has_github_token);
}

/// Register `username` then log in, returning the issued session token.
async fn register_login(client: &mut GitdeckClient<Channel>, username: &str) -> String {
    client
        .register(RegisterRequest {
            username: username.into(),
            password: "pw".into(),
        })
        .await
        .unwrap();
    client
        .login(LoginRequest {
            username: username.into(),
            password: "pw".into(),
        })
        .await
        .unwrap()
        .into_inner()
        .token
}

/// Multi-mode user isolation over the wire: alice creates a board; bob's
/// `list_boards` must NOT include it, `get_board(alice's id)` must NOT find it,
/// while alice still sees her own. Exercises the K2b user-scoped data RPCs end
/// to end (local data only — no GitHub/network).
#[tokio::test]
async fn multi_mode_isolates_boards_per_user() {
    let (url, _jwt) = start(true).await;
    let channel = Channel::from_shared(url).unwrap().connect().await.unwrap();
    let mut client = GitdeckClient::new(channel);

    let alice = register_login(&mut client, "alice-iso").await;
    let bob = register_login(&mut client, "bob-iso").await;

    // Alice creates a board.
    let alice_board = client
        .create_board(with_bearer(
            Request::new(CreateBoardRequest {
                name: "Alice secret".into(),
            }),
            &alice,
        ))
        .await
        .unwrap()
        .into_inner()
        .board
        .expect("created board");

    // Alice sees her own board.
    let alice_list = client
        .list_boards(with_bearer(Request::new(Empty {}), &alice))
        .await
        .unwrap()
        .into_inner()
        .boards;
    assert!(
        alice_list.iter().any(|b| b.id == alice_board.id),
        "alice must see her own board"
    );

    // Bob's list does NOT include alice's board (in fact bob has none).
    let bob_list = client
        .list_boards(with_bearer(Request::new(Empty {}), &bob))
        .await
        .unwrap()
        .into_inner()
        .boards;
    assert!(
        !bob_list.iter().any(|b| b.id == alice_board.id),
        "bob must NOT see alice's board"
    );

    // Bob cannot fetch alice's board by id → NotFound (returns no board).
    let bob_get = client
        .get_board(with_bearer(
            Request::new(BoardIdRequest {
                id: alice_board.id.clone(),
            }),
            &bob,
        ))
        .await
        .unwrap()
        .into_inner();
    assert!(
        bob_get.board.is_none(),
        "bob must NOT be able to read alice's board"
    );

    // Alice can fetch her own board by id.
    let alice_get = client
        .get_board(with_bearer(
            Request::new(BoardIdRequest {
                id: alice_board.id.clone(),
            }),
            &alice,
        ))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(
        alice_get.board.map(|b| b.id),
        Some(alice_board.id),
        "alice must read her own board"
    );
}
