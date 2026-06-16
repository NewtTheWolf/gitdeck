//! Standalone gRPC + gRPC-Web server hosting a headless `TaskService`.
//!
//! Env:
//!   GITDECK_BIND        bind addr           (default 127.0.0.1:50061)
//!   GITDECK_DB          sqlite path OR DSN  (default ./gitdeck.db; a `postgres://`
//!                       or `sqlite://` URL is used verbatim, otherwise treated as
//!                       a SQLite file path)
//!   GITHUB_TOKEN        headless GitHub PAT (optional; single-token fallback)
//!   GITDECK_API_KEY     bearer key          (optional; protects the server)
//!   GITDECK_AUTH        `single` (DEFAULT, BC) or `multi` (per-user accounts)
//!   GITDECK_JWT_SECRET  HS256 secret        (required when GITDECK_AUTH=multi)

use std::sync::Arc;

use newt_todo_api::{
    db_url_from_setting, router, router_with_auth, AuthMode, AuthSetup, AuthStore, EnvTokenStore,
    JwtCodec,
};
use newt_todo_auth::TokenStore;
use newt_todo_core::Store;
use newt_todo_service::TaskService;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let bind: std::net::SocketAddr = std::env::var("GITDECK_BIND")
        .unwrap_or_else(|_| "127.0.0.1:50061".into())
        .parse()?;
    let db_setting = std::env::var("GITDECK_DB").unwrap_or_else(|_| "./gitdeck.db".into());

    // Headless TaskService — mirrors the desktop construction (Store + sqlite),
    // but with an env-backed token store and no Tauri/managed state.
    // `GITDECK_DB` may be a full DSN (`postgres://...` or `sqlite://...`) — used
    // verbatim — or a bare filesystem path, which is treated as a SQLite file.
    let db_url = db_url_from_setting(&db_setting);
    let store = Store::connect(&db_url).await?;
    let service = Arc::new(TaskService::new(store));

    let token_store = EnvTokenStore::from_env();
    let api_key = std::env::var("GITDECK_API_KEY")
        .ok()
        .filter(|s| !s.is_empty());
    let token_on = token_store.has_token();
    let token_store: Arc<dyn TokenStore> = Arc::new(token_store);

    let mode = AuthMode::parse(&std::env::var("GITDECK_AUTH").unwrap_or_default());

    match mode {
        AuthMode::Multi => {
            let secret = std::env::var("GITDECK_JWT_SECRET")
                .ok()
                .filter(|s| !s.is_empty())
                .ok_or_else(|| {
                    anyhow::anyhow!("GITDECK_JWT_SECRET is required when GITDECK_AUTH=multi")
                })?;
            // The auth store opens its OWN connection to the SAME db as core's Store.
            let auth_store = Arc::new(AuthStore::connect(&db_url).await?);
            let jwt = JwtCodec::new(&secret);

            tracing::info!(
                bind = %bind,
                db = %db_url,
                auth = "multi",
                api_key = api_key.is_some(),
                github_token_fallback = token_on,
                "starting gitdeck-server (gRPC + gRPC-Web, multi-user)"
            );

            router_with_auth(
                service,
                token_store,
                AuthSetup::multi(auth_store, jwt, api_key),
            )
            .serve(bind)
            .await?;
        }
        AuthMode::Single => {
            tracing::info!(
                bind = %bind,
                db = %db_url,
                auth = "single",
                api_key = api_key.is_some(),
                github_token = token_on,
                "starting gitdeck-server (gRPC + gRPC-Web)"
            );

            // Keep the original single-user path (BC).
            router(service, token_store, api_key).serve(bind).await?;
        }
    }

    Ok(())
}
