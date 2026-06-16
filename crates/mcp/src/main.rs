mod server;

use std::sync::Arc;

use newt_todo_auth::KeyringTokenStore;
use newt_todo_core::Store;
use newt_todo_service::{resolve_db_url, TaskService};
use rmcp::transport::stdio;
use rmcp::ServiceExt;

use crate::server::TodoServer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let db_url = resolve_db_url()?;
    let store = Store::connect(&db_url).await?;
    let token_store = Arc::new(KeyringTokenStore::new());
    let server = TodoServer::new(TaskService::new(store), token_store);

    let running = server.serve(stdio()).await?;
    running.waiting().await?;
    Ok(())
}
