use std::sync::Arc;

use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router, ErrorData};

use newt_todo_auth::TokenStore;
use newt_todo_service::{
    CreateTaskParams, ListTasksParams, SyncAccountParams, TaskIdParam, TaskService,
    UpdateTaskParams,
};

#[derive(Clone)]
pub struct TodoServer {
    service: Arc<TaskService>,
    token_store: Arc<dyn TokenStore>,
}

impl TodoServer {
    pub fn new(service: TaskService, token_store: Arc<dyn TokenStore>) -> Self {
        Self {
            service: Arc::new(service),
            token_store,
        }
    }

    fn ok<T: serde::Serialize>(value: T) -> Result<String, ErrorData> {
        serde_json::to_string(&value).map_err(|e| ErrorData::internal_error(e.to_string(), None))
    }

    fn fail(e: anyhow::Error) -> ErrorData {
        ErrorData::internal_error(e.to_string(), None)
    }
}

#[tool_router(server_handler)]
impl TodoServer {
    #[tool(description = "List todos. Optional filters: status (open|done), label, query.")]
    async fn list_tasks(
        &self,
        Parameters(p): Parameters<ListTasksParams>,
    ) -> Result<String, ErrorData> {
        Self::ok(self.service.list(p).await.map_err(Self::fail)?)
    }

    #[tool(description = "Get a single todo by id.")]
    async fn get_task(&self, Parameters(p): Parameters<TaskIdParam>) -> Result<String, ErrorData> {
        Self::ok(self.service.get(&p.id).await.map_err(Self::fail)?)
    }

    #[tool(
        description = "Create a new local todo. Fields: title (required), body, labels, due_at (RFC3339)."
    )]
    async fn create_task(
        &self,
        Parameters(p): Parameters<CreateTaskParams>,
    ) -> Result<String, ErrorData> {
        Self::ok(self.service.create(p).await.map_err(Self::fail)?)
    }

    #[tool(
        description = "Update a todo. Fields: id (required), title, body, status, labels, due_at, clear_due."
    )]
    async fn update_task(
        &self,
        Parameters(p): Parameters<UpdateTaskParams>,
    ) -> Result<String, ErrorData> {
        Self::ok(self.service.update(p).await.map_err(Self::fail)?)
    }

    #[tool(description = "Mark a todo done by id.")]
    async fn complete_task(
        &self,
        Parameters(p): Parameters<TaskIdParam>,
    ) -> Result<String, ErrorData> {
        Self::ok(self.service.complete(p).await.map_err(Self::fail)?)
    }

    #[tool(description = "Delete (tombstone) a todo by id.")]
    async fn delete_task(
        &self,
        Parameters(p): Parameters<TaskIdParam>,
    ) -> Result<String, ErrorData> {
        self.service.delete(p).await.map_err(Self::fail)?;
        Self::ok(serde_json::json!({ "deleted": true }))
    }

    #[tool(description = "List configured provider accounts (GitHub, Codeberg, ClickUp).")]
    async fn list_accounts(&self) -> Result<String, ErrorData> {
        Self::ok(self.service.list_accounts().await.map_err(Self::fail)?)
    }

    #[tool(
        description = "Sync an account with its remote provider by account id. Returns {pulled, pushed}."
    )]
    async fn sync(
        &self,
        Parameters(p): Parameters<SyncAccountParams>,
    ) -> Result<String, ErrorData> {
        Self::ok(
            self.service
                .sync(&*self.token_store, &p.account_id)
                .await
                .map_err(Self::fail)?,
        )
    }
}
