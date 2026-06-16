use sea_orm::sea_query::OnConflict;
use sea_orm::{
    ActiveValue::Set, ColumnTrait, ConnectOptions, Database, DatabaseConnection,
    EntityTrait, QueryFilter, QueryOrder, QuerySelect,
};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::account::{Account, AccountDraft, ProviderKind};
use crate::board::{Board, BoardCard, BoardColumn};
use crate::domain::{SourceRef, Task, TaskDraft, TaskFilter, TaskPatch, TaskStatus};
use crate::entities::{
    accounts, board_cards, board_columns, boards, repo_snapshots, tasks,
};
use crate::migrator::Migrator;
use crate::provider::RemoteTask;
use crate::CoreError;

use sea_orm_migration::MigratorTrait;

pub struct Store {
    conn: DatabaseConnection,
}

/// The user_id under which all desktop (offline, single-tenant) data lives.
/// Every existing public Store method operates as this user so the desktop
/// behaves exactly as before multi-user scoping was introduced. The server
/// (crates/api) passes real user ids to the `*_for` variants instead.
pub const LOCAL_USER: &str = "local";

/// A point-in-time capture of a repository's headline metrics for one day.
/// One row per (account, repo, day); re-capturing the same day overwrites it.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct RepoSnapshot {
    pub id: String,
    pub account_id: String,
    pub repo_full_name: String,
    /// Calendar day in `YYYY-MM-DD` (UTC).
    pub day: String,
    pub stars: i64,
    pub forks: i64,
    pub open_issues: i64,
    /// RFC3339 timestamp of when this row was written.
    pub captured_at: String,
}

/// The current UTC calendar date as `YYYY-MM-DD`, using the same `time` crate the
/// rest of the store uses for timestamps.
pub fn today_ymd() -> String {
    let fmt = time::macros::format_description!("[year]-[month]-[day]");
    OffsetDateTime::now_utc()
        .date()
        .format(&fmt)
        .unwrap_or_default()
}

impl Store {
    /// Connect to a database by URL/DSN. Both `sqlite:...` (incl. `sqlite::memory:`)
    /// and `postgres://...` are accepted — SeaORM picks the backend from the scheme.
    /// The schema migrator runs on connect for both backends.
    pub async fn connect(url: &str) -> Result<Self, CoreError> {
        let mut options = ConnectOptions::new(url.to_owned());
        // In-memory SQLite gives each connection its OWN database, so a multi-connection
        // pool would hand out empty/un-migrated databases. Pin in-memory to one connection.
        if url.contains(":memory:") {
            options.max_connections(1);
        }
        let conn = Database::connect(options).await.map_err(CoreError::Db)?;
        Migrator::up(&conn, None).await.map_err(CoreError::Db)?;
        Ok(Self { conn })
    }

    /// The underlying SeaORM connection, for the in-crate sync module.
    pub(crate) fn conn(&self) -> &DatabaseConnection {
        &self.conn
    }

    pub async fn create_task(
        &self,
        draft: TaskDraft,
        now: OffsetDateTime,
    ) -> Result<Task, CoreError> {
        self.create_task_for(LOCAL_USER, draft, now).await
    }

    /// User-scoped task create. The new row is owned by `user_id`.
    pub async fn create_task_for(
        &self,
        user_id: &str,
        draft: TaskDraft,
        now: OffsetDateTime,
    ) -> Result<Task, CoreError> {
        let id = Uuid::new_v4();
        let labels_json = serde_json::to_string(&draft.labels)?;
        let due = draft
            .due_at
            .map(|d| d.format(&Rfc3339))
            .transpose()
            .map_err(|_| CoreError::DataFormat("due_at"))?;
        let updated = now
            .format(&Rfc3339)
            .map_err(|_| CoreError::DataFormat("now"))?;

        let model = tasks::ActiveModel {
            id: Set(id.to_string()),
            account_id: Set(None),
            remote_id: Set(None),
            html_url: Set(None),
            title: Set(draft.title),
            body: Set(draft.body),
            status: Set("open".to_string()),
            labels: Set(labels_json),
            due_at: Set(due),
            project_id: Set(None),
            remote_updated_at: Set(None),
            local_updated_at: Set(updated),
            dirty: Set(0),
            deleted: Set(0),
            user_id: Set(user_id.to_string()),
        };
        tasks::Entity::insert(model)
            .exec(&self.conn)
            .await
            .map_err(CoreError::Db)?;

        self.get_task_for(user_id, id)
            .await?
            .ok_or(CoreError::NotFound(id))
    }

    pub async fn get_task(&self, id: Uuid) -> Result<Option<Task>, CoreError> {
        self.get_task_for(LOCAL_USER, id).await
    }

    /// User-scoped get: returns the task only if it is owned by `user_id`.
    pub async fn get_task_for(
        &self,
        user_id: &str,
        id: Uuid,
    ) -> Result<Option<Task>, CoreError> {
        let row = tasks::Entity::find_by_id(id.to_string())
            .filter(tasks::Column::UserId.eq(user_id))
            .one(&self.conn)
            .await
            .map_err(CoreError::Db)?;
        row.map(map_task).transpose()
    }

    pub async fn list_tasks(&self, filter: TaskFilter) -> Result<Vec<Task>, CoreError> {
        self.list_tasks_for(LOCAL_USER, filter).await
    }

    /// User-scoped list: only `user_id`'s tasks (delete/status/label filters as usual).
    pub async fn list_tasks_for(
        &self,
        user_id: &str,
        filter: TaskFilter,
    ) -> Result<Vec<Task>, CoreError> {
        let mut q = tasks::Entity::find().filter(tasks::Column::UserId.eq(user_id));
        if !filter.include_deleted {
            q = q.filter(tasks::Column::Deleted.eq(0));
        }
        if let Some(status) = filter.status {
            q = q.filter(tasks::Column::Status.eq(status.as_str()));
        }
        if let Some(label) = &filter.label {
            // Approximate JSON-array membership: matches the quoted label as a substring of
            // the labels JSON. Good enough for local todos; does not escape LIKE wildcards
            // (% _) or quotes in the label value. Exact membership would use json_each().
            q = q.filter(tasks::Column::Labels.like(format!("%\"{}\"%", label)));
        }
        if let Some(query) = &filter.query {
            let like = format!("%{}%", query);
            q = q.filter(
                sea_orm::Condition::any()
                    .add(tasks::Column::Title.like(like.clone()))
                    .add(tasks::Column::Body.like(like)),
            );
        }
        let rows = q
            .order_by_desc(tasks::Column::LocalUpdatedAt)
            .all(&self.conn)
            .await
            .map_err(CoreError::Db)?;
        rows.into_iter().map(map_task).collect()
    }

    pub async fn update_task(
        &self,
        id: Uuid,
        patch: TaskPatch,
        now: OffsetDateTime,
    ) -> Result<Task, CoreError> {
        self.update_task_for(LOCAL_USER, id, patch, now).await
    }

    /// User-scoped update: errors with `NotFound` if the task is not owned by `user_id`.
    pub async fn update_task_for(
        &self,
        user_id: &str,
        id: Uuid,
        patch: TaskPatch,
        now: OffsetDateTime,
    ) -> Result<Task, CoreError> {
        let mut current = self
            .get_task_for(user_id, id)
            .await?
            .ok_or(CoreError::NotFound(id))?;

        if let Some(title) = patch.title {
            current.title = title;
        }
        if let Some(body) = patch.body {
            current.body = body;
        }
        if let Some(status) = patch.status {
            current.status = status;
        }
        if let Some(labels) = patch.labels {
            current.labels = labels;
        }
        if let Some(due) = patch.due_at {
            current.due_at = due;
        }

        let labels_json = serde_json::to_string(&current.labels)?;
        let due = current
            .due_at
            .map(|d| d.format(&Rfc3339))
            .transpose()
            .map_err(|_| CoreError::DataFormat("due_at"))?;
        let updated = now
            .format(&Rfc3339)
            .map_err(|_| CoreError::DataFormat("now"))?;

        let model = tasks::ActiveModel {
            id: Set(id.to_string()),
            title: Set(current.title.clone()),
            body: Set(current.body.clone()),
            status: Set(current.status.as_str().to_string()),
            labels: Set(labels_json),
            due_at: Set(due),
            local_updated_at: Set(updated),
            dirty: Set(1),
            ..Default::default()
        };
        tasks::Entity::update(model)
            .filter(tasks::Column::Id.eq(id.to_string()))
            .filter(tasks::Column::UserId.eq(user_id))
            .exec(&self.conn)
            .await
            .map_err(CoreError::Db)?;

        self.get_task_for(user_id, id)
            .await?
            .ok_or(CoreError::NotFound(id))
    }

    /// Insert or update a task mirrored from a provider, keyed by (account_id, remote_id).
    /// If the local row is `dirty` (unpushed local edits), the incoming remote state is
    /// NOT applied (local wins until the next push resolves it). Otherwise remote wins.
    pub async fn upsert_remote_task(
        &self,
        account_id: Uuid,
        remote: RemoteTask,
        now: OffsetDateTime,
    ) -> Result<Task, CoreError> {
        let existing = tasks::Entity::find()
            .filter(tasks::Column::AccountId.eq(account_id.to_string()))
            .filter(tasks::Column::RemoteId.eq(remote.remote_id.clone()))
            .one(&self.conn)
            .await
            .map_err(CoreError::Db)?;

        let labels_json = serde_json::to_string(&remote.labels)?;
        let r_updated = remote
            .remote_updated_at
            .format(&Rfc3339)
            .map_err(|_| CoreError::DataFormat("remote_updated_at"))?;
        let now_s = now
            .format(&Rfc3339)
            .map_err(|_| CoreError::DataFormat("now"))?;

        match existing {
            None => {
                let id = Uuid::new_v4();
                let model = tasks::ActiveModel {
                    id: Set(id.to_string()),
                    account_id: Set(Some(account_id.to_string())),
                    remote_id: Set(Some(remote.remote_id)),
                    html_url: Set(remote.html_url),
                    title: Set(remote.title),
                    body: Set(remote.body),
                    status: Set(remote.status.as_str().to_string()),
                    labels: Set(labels_json),
                    due_at: Set(None),
                    project_id: Set(None),
                    remote_updated_at: Set(Some(r_updated)),
                    local_updated_at: Set(now_s),
                    dirty: Set(0),
                    deleted: Set(0),
                    user_id: Set(LOCAL_USER.to_string()),
                };
                tasks::Entity::insert(model)
                    .exec(&self.conn)
                    .await
                    .map_err(CoreError::Db)?;
                self.get_task(id).await?.ok_or(CoreError::NotFound(id))
            }
            Some(row) => {
                let id = Uuid::parse_str(&row.id).map_err(|_| CoreError::DataFormat("id"))?;
                if row.dirty != 0 {
                    // local edits pending → don't overwrite; just return current row
                    return self.get_task(id).await?.ok_or(CoreError::NotFound(id));
                }
                let model = tasks::ActiveModel {
                    id: Set(id.to_string()),
                    html_url: Set(remote.html_url),
                    title: Set(remote.title),
                    body: Set(remote.body),
                    status: Set(remote.status.as_str().to_string()),
                    labels: Set(labels_json),
                    remote_updated_at: Set(Some(r_updated)),
                    local_updated_at: Set(now_s),
                    ..Default::default()
                };
                tasks::Entity::update(model)
                    .filter(tasks::Column::Id.eq(id.to_string()))
                    .exec(&self.conn)
                    .await
                    .map_err(CoreError::Db)?;
                self.get_task(id).await?.ok_or(CoreError::NotFound(id))
            }
        }
    }

    /// Tasks with unpushed local edits for an account (have a source on that account).
    pub async fn list_dirty(&self, account_id: Uuid) -> Result<Vec<Task>, CoreError> {
        let rows = tasks::Entity::find()
            .filter(tasks::Column::AccountId.eq(account_id.to_string()))
            .filter(tasks::Column::Dirty.eq(1))
            .all(&self.conn)
            .await
            .map_err(CoreError::Db)?;
        rows.into_iter().map(map_task).collect()
    }

    /// Clear the dirty flag and record the provider's updated timestamp after a successful push.
    pub async fn mark_synced(
        &self,
        id: Uuid,
        remote_updated_at: OffsetDateTime,
        now: OffsetDateTime,
    ) -> Result<(), CoreError> {
        let r = remote_updated_at
            .format(&Rfc3339)
            .map_err(|_| CoreError::DataFormat("remote_updated_at"))?;
        let n = now
            .format(&Rfc3339)
            .map_err(|_| CoreError::DataFormat("now"))?;
        let model = tasks::ActiveModel {
            id: Set(id.to_string()),
            dirty: Set(0),
            remote_updated_at: Set(Some(r)),
            local_updated_at: Set(n),
            ..Default::default()
        };
        let res = tasks::Entity::update_many()
            .set(model)
            .filter(tasks::Column::Id.eq(id.to_string()))
            .exec(&self.conn)
            .await
            .map_err(CoreError::Db)?;
        if res.rows_affected == 0 {
            return Err(CoreError::NotFound(id));
        }
        Ok(())
    }

    pub async fn delete_task(&self, id: Uuid, now: OffsetDateTime) -> Result<(), CoreError> {
        self.delete_task_for(LOCAL_USER, id, now).await
    }

    /// User-scoped soft-delete: errors with `NotFound` if not owned by `user_id`.
    pub async fn delete_task_for(
        &self,
        user_id: &str,
        id: Uuid,
        now: OffsetDateTime,
    ) -> Result<(), CoreError> {
        let updated = now
            .format(&Rfc3339)
            .map_err(|_| CoreError::DataFormat("now"))?;
        let model = tasks::ActiveModel {
            id: Set(id.to_string()),
            deleted: Set(1),
            dirty: Set(1),
            local_updated_at: Set(updated),
            ..Default::default()
        };
        let res = tasks::Entity::update_many()
            .set(model)
            .filter(tasks::Column::Id.eq(id.to_string()))
            .filter(tasks::Column::UserId.eq(user_id))
            .exec(&self.conn)
            .await
            .map_err(CoreError::Db)?;
        if res.rows_affected == 0 {
            return Err(CoreError::NotFound(id));
        }
        Ok(())
    }

    pub async fn create_account(
        &self,
        draft: AccountDraft,
        now: OffsetDateTime,
    ) -> Result<Account, CoreError> {
        let id = Uuid::new_v4();
        let config_json = serde_json::to_string(&draft.config)?;
        let created = now
            .format(&Rfc3339)
            .map_err(|_| CoreError::DataFormat("now"))?;

        let model = accounts::ActiveModel {
            id: Set(id.to_string()),
            provider: Set(draft.provider.as_str().to_string()),
            display_name: Set(draft.display_name),
            base_url: Set(draft.base_url),
            config: Set(config_json),
            created_at: Set(created),
        };
        accounts::Entity::insert(model)
            .exec(&self.conn)
            .await
            .map_err(CoreError::Db)?;

        self.get_account(id).await?.ok_or(CoreError::NotFound(id))
    }

    pub async fn get_account(&self, id: Uuid) -> Result<Option<Account>, CoreError> {
        let row = accounts::Entity::find_by_id(id.to_string())
            .one(&self.conn)
            .await
            .map_err(CoreError::Db)?;
        row.map(map_account).transpose()
    }

    pub async fn list_accounts(&self) -> Result<Vec<Account>, CoreError> {
        let rows = accounts::Entity::find()
            .order_by_desc(accounts::Column::CreatedAt)
            .all(&self.conn)
            .await
            .map_err(CoreError::Db)?;
        rows.into_iter().map(map_account).collect()
    }

    pub async fn delete_account(&self, id: Uuid) -> Result<(), CoreError> {
        let res = accounts::Entity::delete_by_id(id.to_string())
            .exec(&self.conn)
            .await
            .map_err(CoreError::Db)?;
        if res.rows_affected == 0 {
            return Err(CoreError::NotFound(id));
        }
        Ok(())
    }

    // --- boards ---------------------------------------------------------------
    //
    // The original sqlx sqlite pool did NOT enable `PRAGMA foreign_keys=ON`, so the
    // `ON DELETE CASCADE` declarations in the schema were inert. Cascades are
    // therefore replicated manually in `delete_board` / `delete_column` (children
    // are deleted first, which is also correct on Postgres where FKs are enforced).

    pub async fn create_board(
        &self,
        name: &str,
        now: OffsetDateTime,
    ) -> Result<Board, CoreError> {
        self.create_board_for(LOCAL_USER, name, now).await
    }

    /// User-scoped board create. The new board is owned by `user_id` and its
    /// position is the max among that user's non-deleted boards + 1.
    pub async fn create_board_for(
        &self,
        user_id: &str,
        name: &str,
        now: OffsetDateTime,
    ) -> Result<Board, CoreError> {
        let id = Uuid::new_v4();
        let now_s = now
            .format(&Rfc3339)
            .map_err(|_| CoreError::DataFormat("now"))?;
        let max: Option<i64> = boards::Entity::find()
            .select_only()
            .column_as(boards::Column::Position.max(), "max_pos")
            .filter(boards::Column::UserId.eq(user_id))
            .filter(boards::Column::Deleted.eq(0))
            .into_tuple()
            .one(&self.conn)
            .await
            .map_err(CoreError::Db)?
            .flatten();
        let position = max.map(|m| m + 1).unwrap_or(0);

        let model = boards::ActiveModel {
            id: Set(id.to_string()),
            name: Set(name.to_string()),
            position: Set(position),
            created_at: Set(now_s.clone()),
            updated_at: Set(now_s),
            user_id: Set(user_id.to_string()),
            dirty: Set(1),
            deleted: Set(0),
        };
        boards::Entity::insert(model)
            .exec(&self.conn)
            .await
            .map_err(CoreError::Db)?;

        self.get_board_for(user_id, id)
            .await?
            .ok_or(CoreError::NotFound(id))
    }

    pub async fn get_board(&self, id: Uuid) -> Result<Option<Board>, CoreError> {
        self.get_board_for(LOCAL_USER, id).await
    }

    /// User-scoped get: returns the board only if owned by `user_id` and not soft-deleted.
    pub async fn get_board_for(
        &self,
        user_id: &str,
        id: Uuid,
    ) -> Result<Option<Board>, CoreError> {
        let row = boards::Entity::find_by_id(id.to_string())
            .filter(boards::Column::UserId.eq(user_id))
            .filter(boards::Column::Deleted.eq(0))
            .one(&self.conn)
            .await
            .map_err(CoreError::Db)?;
        row.map(map_board).transpose()
    }

    pub async fn list_boards(&self) -> Result<Vec<Board>, CoreError> {
        self.list_boards_for(LOCAL_USER).await
    }

    /// User-scoped list: only `user_id`'s non-deleted boards.
    pub async fn list_boards_for(&self, user_id: &str) -> Result<Vec<Board>, CoreError> {
        let rows = boards::Entity::find()
            .filter(boards::Column::UserId.eq(user_id))
            .filter(boards::Column::Deleted.eq(0))
            .order_by_asc(boards::Column::Position)
            .order_by_asc(boards::Column::CreatedAt)
            .all(&self.conn)
            .await
            .map_err(CoreError::Db)?;
        rows.into_iter().map(map_board).collect()
    }

    pub async fn rename_board(
        &self,
        id: Uuid,
        name: &str,
        now: OffsetDateTime,
    ) -> Result<Board, CoreError> {
        self.rename_board_for(LOCAL_USER, id, name, now).await
    }

    /// User-scoped rename: errors with `NotFound` if not owned by `user_id` (or deleted).
    pub async fn rename_board_for(
        &self,
        user_id: &str,
        id: Uuid,
        name: &str,
        now: OffsetDateTime,
    ) -> Result<Board, CoreError> {
        let now_s = now
            .format(&Rfc3339)
            .map_err(|_| CoreError::DataFormat("now"))?;
        let model = boards::ActiveModel {
            name: Set(name.to_string()),
            updated_at: Set(now_s),
            dirty: Set(1),
            ..Default::default()
        };
        let res = boards::Entity::update_many()
            .set(model)
            .filter(boards::Column::Id.eq(id.to_string()))
            .filter(boards::Column::UserId.eq(user_id))
            .filter(boards::Column::Deleted.eq(0))
            .exec(&self.conn)
            .await
            .map_err(CoreError::Db)?;
        if res.rows_affected == 0 {
            return Err(CoreError::NotFound(id));
        }
        self.get_board_for(user_id, id)
            .await?
            .ok_or(CoreError::NotFound(id))
    }

    pub async fn delete_board(&self, id: Uuid) -> Result<(), CoreError> {
        self.delete_board_for(LOCAL_USER, id, OffsetDateTime::now_utc())
            .await
    }

    /// User-scoped soft-delete. Tombstones the board AND (manually cascaded) its
    /// columns + cards by setting `deleted=1`, `dirty=1`, and bumping `updated_at`.
    /// Errors with `NotFound` if the board is not owned by `user_id` (or already deleted).
    pub async fn delete_board_for(
        &self,
        user_id: &str,
        id: Uuid,
        now: OffsetDateTime,
    ) -> Result<(), CoreError> {
        // Ownership check (also rejects already-deleted boards).
        if self.get_board_for(user_id, id).await?.is_none() {
            return Err(CoreError::NotFound(id));
        }
        let now_s = now
            .format(&Rfc3339)
            .map_err(|_| CoreError::DataFormat("now"))?;

        // Manual cascade as SOFT-deletes: cards, then columns, then the board.
        let card_model = board_cards::ActiveModel {
            deleted: Set(1),
            dirty: Set(1),
            updated_at: Set(now_s.clone()),
            ..Default::default()
        };
        board_cards::Entity::update_many()
            .set(card_model)
            .filter(board_cards::Column::BoardId.eq(id.to_string()))
            .exec(&self.conn)
            .await
            .map_err(CoreError::Db)?;

        let col_model = board_columns::ActiveModel {
            deleted: Set(1),
            dirty: Set(1),
            updated_at: Set(now_s.clone()),
            ..Default::default()
        };
        board_columns::Entity::update_many()
            .set(col_model)
            .filter(board_columns::Column::BoardId.eq(id.to_string()))
            .exec(&self.conn)
            .await
            .map_err(CoreError::Db)?;

        let board_model = boards::ActiveModel {
            deleted: Set(1),
            dirty: Set(1),
            updated_at: Set(now_s),
            ..Default::default()
        };
        boards::Entity::update_many()
            .set(board_model)
            .filter(boards::Column::Id.eq(id.to_string()))
            .filter(boards::Column::UserId.eq(user_id))
            .exec(&self.conn)
            .await
            .map_err(CoreError::Db)?;
        Ok(())
    }

    /// Reorder boards so their `position` matches the order of `ordered_ids`.
    /// Ids not present keep their existing position; unknown ids are ignored.
    pub async fn reorder_boards(
        &self,
        ordered_ids: &[Uuid],
        now: OffsetDateTime,
    ) -> Result<(), CoreError> {
        self.reorder_boards_for(LOCAL_USER, ordered_ids, now).await
    }

    /// User-scoped reorder: only affects boards owned by `user_id`.
    pub async fn reorder_boards_for(
        &self,
        user_id: &str,
        ordered_ids: &[Uuid],
        now: OffsetDateTime,
    ) -> Result<(), CoreError> {
        let now_s = now
            .format(&Rfc3339)
            .map_err(|_| CoreError::DataFormat("now"))?;
        for (idx, id) in ordered_ids.iter().enumerate() {
            let model = boards::ActiveModel {
                position: Set(idx as i64),
                updated_at: Set(now_s.clone()),
                dirty: Set(1),
                ..Default::default()
            };
            boards::Entity::update_many()
                .set(model)
                .filter(boards::Column::Id.eq(id.to_string()))
                .filter(boards::Column::UserId.eq(user_id))
                .exec(&self.conn)
                .await
                .map_err(CoreError::Db)?;
        }
        Ok(())
    }

    // --- columns --------------------------------------------------------------

    pub async fn create_column(
        &self,
        board_id: Uuid,
        name: &str,
        filter: &serde_json::Value,
        now: OffsetDateTime,
    ) -> Result<BoardColumn, CoreError> {
        self.create_column_for(LOCAL_USER, board_id, name, filter, now)
            .await
    }

    /// User-scoped column create. Verifies the parent board belongs to `user_id`.
    pub async fn create_column_for(
        &self,
        user_id: &str,
        board_id: Uuid,
        name: &str,
        filter: &serde_json::Value,
        now: OffsetDateTime,
    ) -> Result<BoardColumn, CoreError> {
        self.ensure_board_owned(user_id, board_id).await?;
        let id = Uuid::new_v4();
        let now_s = now
            .format(&Rfc3339)
            .map_err(|_| CoreError::DataFormat("now"))?;
        let filter_json = serde_json::to_string(filter)?;
        let max: Option<i64> = board_columns::Entity::find()
            .select_only()
            .column_as(board_columns::Column::Position.max(), "max_pos")
            .filter(board_columns::Column::BoardId.eq(board_id.to_string()))
            .filter(board_columns::Column::Deleted.eq(0))
            .into_tuple()
            .one(&self.conn)
            .await
            .map_err(CoreError::Db)?
            .flatten();
        let position = max.map(|m| m + 1).unwrap_or(0);

        let model = board_columns::ActiveModel {
            id: Set(id.to_string()),
            board_id: Set(board_id.to_string()),
            name: Set(name.to_string()),
            position: Set(position),
            filter: Set(filter_json),
            created_at: Set(now_s.clone()),
            updated_at: Set(now_s),
            dirty: Set(1),
            deleted: Set(0),
        };
        board_columns::Entity::insert(model)
            .exec(&self.conn)
            .await
            .map_err(CoreError::Db)?;

        self.touch_board(board_id, now).await?;
        self.get_column(id).await?.ok_or(CoreError::NotFound(id))
    }

    pub async fn get_column(&self, id: Uuid) -> Result<Option<BoardColumn>, CoreError> {
        let row = board_columns::Entity::find_by_id(id.to_string())
            .filter(board_columns::Column::Deleted.eq(0))
            .one(&self.conn)
            .await
            .map_err(CoreError::Db)?;
        row.map(map_column).transpose()
    }

    pub async fn list_columns(&self, board_id: Uuid) -> Result<Vec<BoardColumn>, CoreError> {
        let rows = board_columns::Entity::find()
            .filter(board_columns::Column::BoardId.eq(board_id.to_string()))
            .filter(board_columns::Column::Deleted.eq(0))
            .order_by_asc(board_columns::Column::Position)
            .order_by_asc(board_columns::Column::CreatedAt)
            .all(&self.conn)
            .await
            .map_err(CoreError::Db)?;
        rows.into_iter().map(map_column).collect()
    }

    pub async fn update_column(
        &self,
        id: Uuid,
        name: &str,
        filter: &serde_json::Value,
        now: OffsetDateTime,
    ) -> Result<BoardColumn, CoreError> {
        self.update_column_for(LOCAL_USER, id, name, filter, now)
            .await
    }

    /// User-scoped column update. Verifies the column's board belongs to `user_id`.
    pub async fn update_column_for(
        &self,
        user_id: &str,
        id: Uuid,
        name: &str,
        filter: &serde_json::Value,
        now: OffsetDateTime,
    ) -> Result<BoardColumn, CoreError> {
        let existing = self.get_column(id).await?.ok_or(CoreError::NotFound(id))?;
        self.ensure_board_owned(user_id, existing.board_id).await?;
        let now_s = now
            .format(&Rfc3339)
            .map_err(|_| CoreError::DataFormat("now"))?;
        let filter_json = serde_json::to_string(filter)?;
        let model = board_columns::ActiveModel {
            name: Set(name.to_string()),
            filter: Set(filter_json),
            updated_at: Set(now_s),
            dirty: Set(1),
            ..Default::default()
        };
        let res = board_columns::Entity::update_many()
            .set(model)
            .filter(board_columns::Column::Id.eq(id.to_string()))
            .filter(board_columns::Column::Deleted.eq(0))
            .exec(&self.conn)
            .await
            .map_err(CoreError::Db)?;
        if res.rows_affected == 0 {
            return Err(CoreError::NotFound(id));
        }
        let column = self.get_column(id).await?.ok_or(CoreError::NotFound(id))?;
        self.touch_board(column.board_id, now).await?;
        Ok(column)
    }

    pub async fn delete_column(&self, id: Uuid, now: OffsetDateTime) -> Result<(), CoreError> {
        self.delete_column_for(LOCAL_USER, id, now).await
    }

    /// User-scoped column soft-delete. Verifies the column's board belongs to
    /// `user_id`, then tombstones the column and (manual cascade) its cards.
    pub async fn delete_column_for(
        &self,
        user_id: &str,
        id: Uuid,
        now: OffsetDateTime,
    ) -> Result<(), CoreError> {
        // Resolve the owning board first (also rejects already-deleted columns).
        let column = self.get_column(id).await?.ok_or(CoreError::NotFound(id))?;
        self.ensure_board_owned(user_id, column.board_id).await?;
        let now_s = now
            .format(&Rfc3339)
            .map_err(|_| CoreError::DataFormat("now"))?;

        // Manual cascade as SOFT-deletes: the column's cards first.
        let card_model = board_cards::ActiveModel {
            deleted: Set(1),
            dirty: Set(1),
            updated_at: Set(now_s.clone()),
            ..Default::default()
        };
        board_cards::Entity::update_many()
            .set(card_model)
            .filter(board_cards::Column::ColumnId.eq(id.to_string()))
            .exec(&self.conn)
            .await
            .map_err(CoreError::Db)?;

        let col_model = board_columns::ActiveModel {
            deleted: Set(1),
            dirty: Set(1),
            updated_at: Set(now_s),
            ..Default::default()
        };
        board_columns::Entity::update_many()
            .set(col_model)
            .filter(board_columns::Column::Id.eq(id.to_string()))
            .exec(&self.conn)
            .await
            .map_err(CoreError::Db)?;

        self.touch_board(column.board_id, now).await?;
        Ok(())
    }

    /// Reorder columns within a board so their `position` matches `ordered_ids`.
    pub async fn reorder_columns(
        &self,
        board_id: Uuid,
        ordered_ids: &[Uuid],
        now: OffsetDateTime,
    ) -> Result<(), CoreError> {
        self.reorder_columns_for(LOCAL_USER, board_id, ordered_ids, now)
            .await
    }

    /// User-scoped column reorder. Verifies the board belongs to `user_id`.
    pub async fn reorder_columns_for(
        &self,
        user_id: &str,
        board_id: Uuid,
        ordered_ids: &[Uuid],
        now: OffsetDateTime,
    ) -> Result<(), CoreError> {
        self.ensure_board_owned(user_id, board_id).await?;
        let now_s = now
            .format(&Rfc3339)
            .map_err(|_| CoreError::DataFormat("now"))?;
        for (idx, id) in ordered_ids.iter().enumerate() {
            let model = board_columns::ActiveModel {
                position: Set(idx as i64),
                updated_at: Set(now_s.clone()),
                dirty: Set(1),
                ..Default::default()
            };
            board_columns::Entity::update_many()
                .set(model)
                .filter(board_columns::Column::Id.eq(id.to_string()))
                .filter(board_columns::Column::BoardId.eq(board_id.to_string()))
                .exec(&self.conn)
                .await
                .map_err(CoreError::Db)?;
        }
        self.touch_board(board_id, now).await?;
        Ok(())
    }

    // --- cards ----------------------------------------------------------------

    /// Manually place a card. Each `item_key` is unique per board
    /// (`UNIQUE(board_id, item_key)`), so placing an already-placed item moves it
    /// to the new column instead of erroring.
    pub async fn place_card(
        &self,
        board_id: Uuid,
        column_id: Uuid,
        item_key: &str,
        now: OffsetDateTime,
    ) -> Result<BoardCard, CoreError> {
        self.place_card_for(LOCAL_USER, board_id, column_id, item_key, now)
            .await
    }

    /// User-scoped card placement. Verifies the board belongs to `user_id`.
    pub async fn place_card_for(
        &self,
        user_id: &str,
        board_id: Uuid,
        column_id: Uuid,
        item_key: &str,
        now: OffsetDateTime,
    ) -> Result<BoardCard, CoreError> {
        self.ensure_board_owned(user_id, board_id).await?;
        let now_s = now
            .format(&Rfc3339)
            .map_err(|_| CoreError::DataFormat("now"))?;

        // Match any existing card for this item (incl. a tombstoned one) so the
        // UNIQUE(board_id, item_key) constraint can't be violated by re-placing.
        let existing = board_cards::Entity::find()
            .filter(board_cards::Column::BoardId.eq(board_id.to_string()))
            .filter(board_cards::Column::ItemKey.eq(item_key))
            .one(&self.conn)
            .await
            .map_err(CoreError::Db)?;

        let id = match existing {
            Some(existing_row) => {
                // Already placed on this board → move it (un-tombstoning if needed).
                let position = self.next_card_position(column_id).await?;
                let model = board_cards::ActiveModel {
                    id: Set(existing_row.id.clone()),
                    column_id: Set(column_id.to_string()),
                    position: Set(position),
                    updated_at: Set(now_s),
                    dirty: Set(1),
                    deleted: Set(0),
                    ..Default::default()
                };
                board_cards::Entity::update_many()
                    .set(model)
                    .filter(board_cards::Column::Id.eq(existing_row.id.clone()))
                    .exec(&self.conn)
                    .await
                    .map_err(CoreError::Db)?;
                Uuid::parse_str(&existing_row.id).map_err(|_| CoreError::DataFormat("id"))?
            }
            None => {
                let new_id = Uuid::new_v4();
                let position = self.next_card_position(column_id).await?;
                let model = board_cards::ActiveModel {
                    id: Set(new_id.to_string()),
                    board_id: Set(board_id.to_string()),
                    column_id: Set(column_id.to_string()),
                    item_key: Set(item_key.to_string()),
                    position: Set(position),
                    created_at: Set(now_s.clone()),
                    updated_at: Set(now_s),
                    dirty: Set(1),
                    deleted: Set(0),
                };
                board_cards::Entity::insert(model)
                    .exec(&self.conn)
                    .await
                    .map_err(CoreError::Db)?;
                new_id
            }
        };

        self.touch_board(board_id, now).await?;
        self.get_card(id).await?.ok_or(CoreError::NotFound(id))
    }

    async fn next_card_position(&self, column_id: Uuid) -> Result<i64, CoreError> {
        let max: Option<i64> = board_cards::Entity::find()
            .select_only()
            .column_as(board_cards::Column::Position.max(), "max_pos")
            .filter(board_cards::Column::ColumnId.eq(column_id.to_string()))
            .filter(board_cards::Column::Deleted.eq(0))
            .into_tuple()
            .one(&self.conn)
            .await
            .map_err(CoreError::Db)?
            .flatten();
        Ok(max.map(|m| m + 1).unwrap_or(0))
    }

    pub async fn get_card(&self, id: Uuid) -> Result<Option<BoardCard>, CoreError> {
        let row = board_cards::Entity::find_by_id(id.to_string())
            .filter(board_cards::Column::Deleted.eq(0))
            .one(&self.conn)
            .await
            .map_err(CoreError::Db)?;
        row.map(map_card).transpose()
    }

    pub async fn list_cards(&self, board_id: Uuid) -> Result<Vec<BoardCard>, CoreError> {
        let rows = board_cards::Entity::find()
            .filter(board_cards::Column::BoardId.eq(board_id.to_string()))
            .filter(board_cards::Column::Deleted.eq(0))
            .order_by_asc(board_cards::Column::Position)
            .order_by_asc(board_cards::Column::CreatedAt)
            .all(&self.conn)
            .await
            .map_err(CoreError::Db)?;
        rows.into_iter().map(map_card).collect()
    }

    /// Remove a manually placed card. Idempotent: removing an absent item is a no-op.
    pub async fn remove_card(
        &self,
        board_id: Uuid,
        item_key: &str,
        now: OffsetDateTime,
    ) -> Result<(), CoreError> {
        self.remove_card_for(LOCAL_USER, board_id, item_key, now)
            .await
    }

    /// User-scoped card removal (soft-delete). Verifies the board belongs to `user_id`.
    /// Idempotent: removing an absent/already-deleted item is a no-op.
    pub async fn remove_card_for(
        &self,
        user_id: &str,
        board_id: Uuid,
        item_key: &str,
        now: OffsetDateTime,
    ) -> Result<(), CoreError> {
        self.ensure_board_owned(user_id, board_id).await?;
        let now_s = now
            .format(&Rfc3339)
            .map_err(|_| CoreError::DataFormat("now"))?;
        let model = board_cards::ActiveModel {
            deleted: Set(1),
            dirty: Set(1),
            updated_at: Set(now_s),
            ..Default::default()
        };
        let res = board_cards::Entity::update_many()
            .set(model)
            .filter(board_cards::Column::BoardId.eq(board_id.to_string()))
            .filter(board_cards::Column::ItemKey.eq(item_key))
            .filter(board_cards::Column::Deleted.eq(0))
            .exec(&self.conn)
            .await
            .map_err(CoreError::Db)?;
        if res.rows_affected > 0 {
            self.touch_board(board_id, now).await?;
        }
        Ok(())
    }

    /// Verify a board exists, is not soft-deleted, and is owned by `user_id`.
    /// Returns `NotFound` otherwise — the ownership gate for columns/cards, which
    /// carry no `user_id` of their own and are reached only through their board.
    async fn ensure_board_owned(&self, user_id: &str, board_id: Uuid) -> Result<(), CoreError> {
        if self.get_board_for(user_id, board_id).await?.is_none() {
            return Err(CoreError::NotFound(board_id));
        }
        Ok(())
    }

    /// Best-effort bump of a board's `updated_at` (+ `dirty`) when its
    /// columns/cards/name change.
    async fn touch_board(&self, board_id: Uuid, now: OffsetDateTime) -> Result<(), CoreError> {
        let now_s = now
            .format(&Rfc3339)
            .map_err(|_| CoreError::DataFormat("now"))?;
        let model = boards::ActiveModel {
            updated_at: Set(now_s),
            dirty: Set(1),
            ..Default::default()
        };
        boards::Entity::update_many()
            .set(model)
            .filter(boards::Column::Id.eq(board_id.to_string()))
            .exec(&self.conn)
            .await
            .map_err(CoreError::Db)?;
        Ok(())
    }

    // --- repo snapshots -------------------------------------------------------

    /// Insert or replace the snapshot for `(account_id, repo_full_name, day)`.
    ///
    /// Re-capturing the same day overwrites the row with the latest numbers (so
    /// the last capture of the day wins). The `id` and `captured_at` are
    /// regenerated on every write, matching how other rows produce ids/timestamps.
    pub async fn upsert_snapshot(
        &self,
        account_id: &str,
        repo_full_name: &str,
        day: &str,
        stars: i64,
        forks: i64,
        open_issues: i64,
    ) -> Result<RepoSnapshot, CoreError> {
        let id = Uuid::new_v4().to_string();
        let captured_at = OffsetDateTime::now_utc()
            .format(&Rfc3339)
            .map_err(|_| CoreError::DataFormat("captured_at"))?;

        let model = repo_snapshots::ActiveModel {
            id: Set(id),
            account_id: Set(account_id.to_string()),
            repo_full_name: Set(repo_full_name.to_string()),
            day: Set(day.to_string()),
            stars: Set(stars),
            forks: Set(forks),
            open_issues: Set(open_issues),
            captured_at: Set(captured_at),
        };

        // Portable upsert on the unique (account_id, repo_full_name, day) index —
        // works on both SQLite and Postgres via `INSERT ... ON CONFLICT DO UPDATE`.
        repo_snapshots::Entity::insert(model)
            .on_conflict(
                OnConflict::columns([
                    repo_snapshots::Column::AccountId,
                    repo_snapshots::Column::RepoFullName,
                    repo_snapshots::Column::Day,
                ])
                .update_columns([
                    repo_snapshots::Column::Id,
                    repo_snapshots::Column::Stars,
                    repo_snapshots::Column::Forks,
                    repo_snapshots::Column::OpenIssues,
                    repo_snapshots::Column::CapturedAt,
                ])
                .to_owned(),
            )
            .exec(&self.conn)
            .await
            .map_err(CoreError::Db)?;

        let row = repo_snapshots::Entity::find()
            .filter(repo_snapshots::Column::AccountId.eq(account_id))
            .filter(repo_snapshots::Column::RepoFullName.eq(repo_full_name))
            .filter(repo_snapshots::Column::Day.eq(day))
            .one(&self.conn)
            .await
            .map_err(CoreError::Db)?
            .ok_or(CoreError::DataFormat("snapshot"))?;
        Ok(map_snapshot(row))
    }

    /// All snapshots for an account, optionally filtered to `day >= since_day`,
    /// ordered by day ascending then repo_full_name ascending.
    pub async fn list_snapshots(
        &self,
        account_id: &str,
        since_day: Option<&str>,
    ) -> Result<Vec<RepoSnapshot>, CoreError> {
        let mut q = repo_snapshots::Entity::find()
            .filter(repo_snapshots::Column::AccountId.eq(account_id));
        if let Some(since) = since_day {
            q = q.filter(repo_snapshots::Column::Day.gte(since));
        }
        let rows = q
            .order_by_asc(repo_snapshots::Column::Day)
            .order_by_asc(repo_snapshots::Column::RepoFullName)
            .all(&self.conn)
            .await
            .map_err(CoreError::Db)?;
        Ok(rows.into_iter().map(map_snapshot).collect())
    }

    /// The two most recent distinct `day` values for an account, most-recent first.
    /// Returns 0, 1, or 2 entries. Used to diff today's capture against the previous one.
    pub async fn latest_two_days(&self, account_id: &str) -> Result<Vec<String>, CoreError> {
        let days: Vec<String> = repo_snapshots::Entity::find()
            .select_only()
            .column(repo_snapshots::Column::Day)
            .filter(repo_snapshots::Column::AccountId.eq(account_id))
            .distinct()
            .order_by_desc(repo_snapshots::Column::Day)
            .limit(2)
            .into_tuple()
            .all(&self.conn)
            .await
            .map_err(CoreError::Db)?;
        Ok(days)
    }
}

fn map_snapshot(m: repo_snapshots::Model) -> RepoSnapshot {
    RepoSnapshot {
        id: m.id,
        account_id: m.account_id,
        repo_full_name: m.repo_full_name,
        day: m.day,
        stars: m.stars,
        forks: m.forks,
        open_issues: m.open_issues,
        captured_at: m.captured_at,
    }
}

fn map_account(m: accounts::Model) -> Result<Account, CoreError> {
    Ok(Account {
        id: Uuid::parse_str(&m.id).map_err(|_| CoreError::DataFormat("id"))?,
        provider: ProviderKind::parse(&m.provider).ok_or(CoreError::DataFormat("provider"))?,
        display_name: m.display_name,
        base_url: m.base_url,
        config: serde_json::from_str(&m.config)?,
        created_at: OffsetDateTime::parse(&m.created_at, &Rfc3339)
            .map_err(|_| CoreError::DataFormat("created_at"))?,
    })
}

fn map_task(m: tasks::Model) -> Result<Task, CoreError> {
    let source = match (m.account_id, m.remote_id) {
        (Some(account_id), Some(remote_id)) => Some(SourceRef {
            account_id: Uuid::parse_str(&account_id)
                .map_err(|_| CoreError::DataFormat("account_id"))?,
            remote_id,
            html_url: m.html_url,
        }),
        _ => None,
    };

    let parse_dt =
        |s: &str| OffsetDateTime::parse(s, &Rfc3339).map_err(|_| CoreError::DataFormat("datetime"));

    Ok(Task {
        id: Uuid::parse_str(&m.id).map_err(|_| CoreError::DataFormat("id"))?,
        source,
        title: m.title,
        body: m.body,
        status: TaskStatus::from_str(&m.status).ok_or(CoreError::DataFormat("status"))?,
        labels: serde_json::from_str(&m.labels)?,
        due_at: m.due_at.as_deref().map(parse_dt).transpose()?,
        local_updated_at: parse_dt(&m.local_updated_at)?,
        dirty: m.dirty != 0,
        deleted: m.deleted != 0,
    })
}

fn map_board(m: boards::Model) -> Result<Board, CoreError> {
    Ok(Board {
        id: Uuid::parse_str(&m.id).map_err(|_| CoreError::DataFormat("id"))?,
        name: m.name,
        position: m.position,
        created_at: OffsetDateTime::parse(&m.created_at, &Rfc3339)
            .map_err(|_| CoreError::DataFormat("created_at"))?,
        updated_at: OffsetDateTime::parse(&m.updated_at, &Rfc3339)
            .map_err(|_| CoreError::DataFormat("updated_at"))?,
    })
}

fn map_column(m: board_columns::Model) -> Result<BoardColumn, CoreError> {
    Ok(BoardColumn {
        id: Uuid::parse_str(&m.id).map_err(|_| CoreError::DataFormat("id"))?,
        board_id: Uuid::parse_str(&m.board_id).map_err(|_| CoreError::DataFormat("board_id"))?,
        name: m.name,
        position: m.position,
        filter: serde_json::from_str(&m.filter)?,
        created_at: OffsetDateTime::parse(&m.created_at, &Rfc3339)
            .map_err(|_| CoreError::DataFormat("created_at"))?,
    })
}

fn map_card(m: board_cards::Model) -> Result<BoardCard, CoreError> {
    Ok(BoardCard {
        id: Uuid::parse_str(&m.id).map_err(|_| CoreError::DataFormat("id"))?,
        board_id: Uuid::parse_str(&m.board_id).map_err(|_| CoreError::DataFormat("board_id"))?,
        column_id: Uuid::parse_str(&m.column_id).map_err(|_| CoreError::DataFormat("column_id"))?,
        item_key: m.item_key,
        position: m.position,
        created_at: OffsetDateTime::parse(&m.created_at, &Rfc3339)
            .map_err(|_| CoreError::DataFormat("created_at"))?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::{ConnectionTrait, Statement};

    use crate::account::{AccountDraft, ProviderKind};
    use crate::domain::{TaskDraft, TaskFilter, TaskPatch, TaskStatus};
    use crate::provider::RemoteTask;
    use time::macros::datetime;

    async fn mem_store() -> Store {
        Store::connect("sqlite::memory:").await.unwrap()
    }

    /// Run a `SELECT COUNT(*)` against the store's connection (test-only helper that
    /// replaces the old raw-sqlx scalar queries).
    async fn count(store: &Store, table: &str) -> i64 {
        let backend = store.conn.get_database_backend();
        let stmt = Statement::from_string(backend, format!("SELECT COUNT(*) AS c FROM {table}"));
        let row = store.conn.query_one(stmt).await.unwrap().unwrap();
        row.try_get::<i64>("", "c").unwrap()
    }

    fn remote(
        id: &str,
        title: &str,
        status: TaskStatus,
        updated: time::OffsetDateTime,
    ) -> RemoteTask {
        RemoteTask {
            remote_id: id.into(),
            title: title.into(),
            body: String::new(),
            status,
            labels: vec![],
            html_url: Some(format!("https://x/{id}")),
            remote_updated_at: updated,
        }
    }

    #[tokio::test]
    async fn upsert_remote_inserts_then_updates_when_not_dirty() {
        let store = mem_store().await;
        let acct = uuid::Uuid::new_v4();
        let t0 = datetime!(2026-06-15 10:00:00 UTC);
        let t1 = datetime!(2026-06-15 11:00:00 UTC);

        let created = store
            .upsert_remote_task(acct, remote("1", "first", TaskStatus::Open, t0), t0)
            .await
            .unwrap();
        assert_eq!(created.title, "first");
        assert_eq!(created.source.as_ref().unwrap().remote_id, "1");
        assert!(!created.dirty);

        // remote changed, local not dirty → remote wins
        let updated = store
            .upsert_remote_task(acct, remote("1", "renamed", TaskStatus::Done, t1), t1)
            .await
            .unwrap();
        assert_eq!(
            updated.id, created.id,
            "same row (matched by account+remote_id)"
        );
        assert_eq!(updated.title, "renamed");
        assert_eq!(updated.status, TaskStatus::Done);
    }

    #[tokio::test]
    async fn upsert_remote_preserves_local_dirty_edits() {
        let store = mem_store().await;
        let acct = uuid::Uuid::new_v4();
        let t0 = datetime!(2026-06-15 10:00:00 UTC);
        let t1 = datetime!(2026-06-15 11:00:00 UTC);
        let created = store
            .upsert_remote_task(acct, remote("1", "first", TaskStatus::Open, t0), t0)
            .await
            .unwrap();
        // user edits locally → dirty
        store
            .update_task(
                created.id,
                crate::domain::TaskPatch {
                    title: Some("my edit".into()),
                    ..Default::default()
                },
                t1,
            )
            .await
            .unwrap();
        // remote pull comes in; must NOT clobber the dirty local edit
        let after = store
            .upsert_remote_task(acct, remote("1", "remote change", TaskStatus::Done, t1), t1)
            .await
            .unwrap();
        assert_eq!(
            after.title, "my edit",
            "local dirty edit preserved over incoming remote change"
        );
        assert!(after.dirty);
    }

    #[tokio::test]
    async fn list_dirty_and_mark_synced() {
        let store = mem_store().await;
        let acct = uuid::Uuid::new_v4();
        let t0 = datetime!(2026-06-15 10:00:00 UTC);
        let t = store
            .upsert_remote_task(acct, remote("9", "task", TaskStatus::Open, t0), t0)
            .await
            .unwrap();
        store
            .update_task(
                t.id,
                crate::domain::TaskPatch {
                    status: Some(TaskStatus::Done),
                    ..Default::default()
                },
                t0,
            )
            .await
            .unwrap();

        let dirty = store.list_dirty(acct).await.unwrap();
        assert_eq!(dirty.len(), 1);
        assert_eq!(dirty[0].id, t.id);

        store.mark_synced(t.id, t0, t0).await.unwrap();
        assert!(store.list_dirty(acct).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn connect_runs_migration_and_table_exists() {
        let store = mem_store().await;
        assert_eq!(count(&store, "tasks").await, 0);
    }

    #[tokio::test]
    async fn create_then_get_roundtrips() {
        let store = mem_store().await;
        let now = datetime!(2026-06-15 12:00:00 UTC);
        let mut draft = TaskDraft::new("Buy milk");
        draft.body = "2 litres".into();
        draft.labels = vec!["home".into(), "shopping".into()];

        let created = store.create_task(draft, now).await.unwrap();
        assert_eq!(created.title, "Buy milk");
        assert_eq!(created.status, TaskStatus::Open);
        assert!(created.source.is_none());
        assert!(!created.dirty);
        assert_eq!(created.local_updated_at, now);

        let fetched = store.get_task(created.id).await.unwrap().unwrap();
        assert_eq!(fetched, created);
    }

    #[tokio::test]
    async fn get_missing_returns_none() {
        let store = mem_store().await;
        let missing = uuid::Uuid::new_v4();
        assert!(store.get_task(missing).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn list_filters_by_status_and_excludes_deleted() {
        let store = mem_store().await;
        let now = datetime!(2026-06-15 12:00:00 UTC);
        let a = store
            .create_task(TaskDraft::new("open one"), now)
            .await
            .unwrap();
        let _b = store
            .create_task(TaskDraft::new("open two"), now)
            .await
            .unwrap();
        store
            .update_task(
                a.id,
                TaskPatch {
                    status: Some(TaskStatus::Done),
                    ..Default::default()
                },
                now,
            )
            .await
            .unwrap();

        let open = store
            .list_tasks(TaskFilter {
                status: Some(TaskStatus::Open),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(open.len(), 1);
        assert_eq!(open[0].title, "open two");

        let all = store.list_tasks(TaskFilter::default()).await.unwrap();
        assert_eq!(all.len(), 2);
    }

    #[tokio::test]
    async fn update_applies_patch_and_marks_dirty() {
        let store = mem_store().await;
        let t0 = datetime!(2026-06-15 12:00:00 UTC);
        let t1 = datetime!(2026-06-15 13:00:00 UTC);
        let task = store
            .create_task(TaskDraft::new("draft title"), t0)
            .await
            .unwrap();

        let patch = TaskPatch {
            title: Some("new title".into()),
            status: Some(TaskStatus::Done),
            ..Default::default()
        };
        let updated = store.update_task(task.id, patch, t1).await.unwrap();

        assert_eq!(updated.title, "new title");
        assert_eq!(updated.status, TaskStatus::Done);
        assert!(
            updated.dirty,
            "local edits must mark the task dirty for later push"
        );
        assert_eq!(updated.local_updated_at, t1);
        assert_eq!(updated.body, ""); // body untouched by the patch
    }

    #[tokio::test]
    async fn update_due_at_clear_set_and_untouched() {
        let store = mem_store().await;
        let t0 = datetime!(2026-06-15 12:00:00 UTC);
        let due = datetime!(2026-06-20 09:00:00 UTC);
        let mut draft = TaskDraft::new("with due");
        draft.due_at = Some(due);
        let task = store.create_task(draft, t0).await.unwrap();
        assert_eq!(task.due_at, Some(due));

        // patch.due_at == None  → untouched
        let unchanged = store
            .update_task(
                task.id,
                TaskPatch {
                    title: Some("x".into()),
                    ..Default::default()
                },
                t0,
            )
            .await
            .unwrap();
        assert_eq!(
            unchanged.due_at,
            Some(due),
            "due_at untouched when patch.due_at is None"
        );

        // patch.due_at == Some(None)  → cleared
        let cleared = store
            .update_task(
                task.id,
                TaskPatch {
                    due_at: Some(None),
                    ..Default::default()
                },
                t0,
            )
            .await
            .unwrap();
        assert_eq!(
            cleared.due_at, None,
            "due_at cleared when patch.due_at is Some(None)"
        );
    }

    #[tokio::test]
    async fn delete_tombstones_and_hides_from_default_list() {
        let store = mem_store().await;
        let now = datetime!(2026-06-15 12:00:00 UTC);
        let task = store
            .create_task(TaskDraft::new("temp"), now)
            .await
            .unwrap();

        store.delete_task(task.id, now).await.unwrap();

        let default_list = store.list_tasks(TaskFilter::default()).await.unwrap();
        assert!(default_list.is_empty(), "deleted tasks hidden by default");

        let with_deleted = store
            .list_tasks(TaskFilter {
                include_deleted: true,
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(with_deleted.len(), 1);
        assert!(with_deleted[0].deleted);
        assert!(
            with_deleted[0].dirty,
            "tombstone is dirty so a later sync can push the deletion"
        );
    }

    fn account_draft() -> AccountDraft {
        AccountDraft {
            provider: ProviderKind::Github,
            display_name: "my gh".into(),
            base_url: None,
            config: serde_json::json!({ "owner": "o", "repo": "r" }),
        }
    }

    #[tokio::test]
    async fn create_account_then_get_roundtrips() {
        let store = mem_store().await;
        let now = datetime!(2026-06-15 12:00:00 UTC);

        let created = store.create_account(account_draft(), now).await.unwrap();
        assert_eq!(created.provider, ProviderKind::Github);
        assert_eq!(created.display_name, "my gh");
        assert_eq!(created.base_url, None);
        assert_eq!(
            created.config,
            serde_json::json!({ "owner": "o", "repo": "r" })
        );
        assert_eq!(created.created_at, now);

        let fetched = store.get_account(created.id).await.unwrap().unwrap();
        assert_eq!(fetched, created);
    }

    #[tokio::test]
    async fn get_account_missing_returns_none() {
        let store = mem_store().await;
        let missing = uuid::Uuid::new_v4();
        assert!(store.get_account(missing).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn list_accounts_returns_all() {
        let store = mem_store().await;
        let now = datetime!(2026-06-15 12:00:00 UTC);

        store.create_account(account_draft(), now).await.unwrap();
        let mut second = account_draft();
        second.provider = ProviderKind::Codeberg;
        second.display_name = "cb".into();
        second.base_url = Some("https://codeberg.org/api/v1".into());
        store.create_account(second, now).await.unwrap();

        let all = store.list_accounts().await.unwrap();
        assert_eq!(all.len(), 2);
    }

    #[tokio::test]
    async fn delete_account_removes_it() {
        let store = mem_store().await;
        let now = datetime!(2026-06-15 12:00:00 UTC);
        let created = store.create_account(account_draft(), now).await.unwrap();

        store.delete_account(created.id).await.unwrap();
        assert!(store.get_account(created.id).await.unwrap().is_none());
        assert!(store.list_accounts().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn delete_missing_account_is_not_found() {
        let store = mem_store().await;
        let missing = uuid::Uuid::new_v4();
        assert!(matches!(
            store.delete_account(missing).await,
            Err(CoreError::NotFound(_))
        ));
    }

    // --- boards ---------------------------------------------------------------

    #[tokio::test]
    async fn create_and_list_boards_ordered_by_position() {
        let store = mem_store().await;
        let now = datetime!(2026-06-15 12:00:00 UTC);
        let a = store.create_board("first", now).await.unwrap();
        let b = store.create_board("second", now).await.unwrap();
        assert_eq!(a.position, 0);
        assert_eq!(b.position, 1);

        let boards = store.list_boards().await.unwrap();
        assert_eq!(boards.len(), 2);
        assert_eq!(boards[0].id, a.id);
        assert_eq!(boards[1].id, b.id);
    }

    #[tokio::test]
    async fn rename_board_updates_name_and_timestamp() {
        let store = mem_store().await;
        let t0 = datetime!(2026-06-15 12:00:00 UTC);
        let t1 = datetime!(2026-06-15 13:00:00 UTC);
        let board = store.create_board("old", t0).await.unwrap();

        let renamed = store.rename_board(board.id, "new", t1).await.unwrap();
        assert_eq!(renamed.name, "new");
        assert_eq!(renamed.updated_at, t1);

        let missing = uuid::Uuid::new_v4();
        assert!(matches!(
            store.rename_board(missing, "x", t1).await,
            Err(CoreError::NotFound(_))
        ));
    }

    #[tokio::test]
    async fn delete_board_cascades_columns_and_cards() {
        let store = mem_store().await;
        let now = datetime!(2026-06-15 12:00:00 UTC);
        let board = store.create_board("b", now).await.unwrap();
        let col = store
            .create_column(board.id, "todo", &serde_json::json!({}), now)
            .await
            .unwrap();
        store
            .place_card(board.id, col.id, "todo:1", now)
            .await
            .unwrap();

        store.delete_board(board.id).await.unwrap();

        // Soft-delete: the board + its columns/cards are tombstoned, so they are
        // invisible to gets/lists exactly like the old hard-delete made them.
        assert!(store.get_board(board.id).await.unwrap().is_none());
        assert!(store.list_columns(board.id).await.unwrap().is_empty());
        assert!(store.list_cards(board.id).await.unwrap().is_empty());
        // But the rows still physically exist (tombstones, for sync to propagate).
        assert_eq!(count(&store, "boards").await, 1);
        assert_eq!(count(&store, "board_columns").await, 1);
        assert_eq!(count(&store, "board_cards").await, 1);
    }

    #[tokio::test]
    async fn delete_missing_board_is_not_found() {
        let store = mem_store().await;
        let missing = uuid::Uuid::new_v4();
        assert!(matches!(
            store.delete_board(missing).await,
            Err(CoreError::NotFound(_))
        ));
    }

    #[tokio::test]
    async fn reorder_boards_sets_positions() {
        let store = mem_store().await;
        let now = datetime!(2026-06-15 12:00:00 UTC);
        let a = store.create_board("a", now).await.unwrap();
        let b = store.create_board("b", now).await.unwrap();
        let c = store.create_board("c", now).await.unwrap();

        store
            .reorder_boards(&[c.id, a.id, b.id], now)
            .await
            .unwrap();
        let boards = store.list_boards().await.unwrap();
        assert_eq!(
            boards.iter().map(|x| x.id).collect::<Vec<_>>(),
            vec![c.id, a.id, b.id]
        );
    }

    #[tokio::test]
    async fn soft_deleted_board_hidden_but_row_remains() {
        let store = mem_store().await;
        let now = datetime!(2026-06-15 12:00:00 UTC);
        let board = store.create_board("temp", now).await.unwrap();

        store.delete_board(board.id).await.unwrap();

        // Invisible to list/get…
        assert!(store.list_boards().await.unwrap().is_empty());
        assert!(store.get_board(board.id).await.unwrap().is_none());
        // …but the tombstone row physically remains with deleted=1.
        assert_eq!(count(&store, "boards").await, 1);
        let backend = store.conn.get_database_backend();
        let stmt = sea_orm::Statement::from_string(
            backend,
            format!("SELECT deleted FROM boards WHERE id = '{}'", board.id),
        );
        let row = store.conn.query_one(stmt).await.unwrap().unwrap();
        assert_eq!(row.try_get::<i64>("", "deleted").unwrap(), 1);
    }

    #[tokio::test]
    async fn boards_are_isolated_per_user() {
        let store = mem_store().await;
        let now = datetime!(2026-06-15 12:00:00 UTC);
        let alice_board = store.create_board_for("alice", "a-board", now).await.unwrap();
        let bob_board = store.create_board_for("bob", "b-board", now).await.unwrap();

        let alice = store.list_boards_for("alice").await.unwrap();
        assert_eq!(alice.len(), 1);
        assert_eq!(alice[0].id, alice_board.id);

        let bob = store.list_boards_for("bob").await.unwrap();
        assert_eq!(bob.len(), 1);
        assert_eq!(bob[0].id, bob_board.id);

        // The default (LOCAL_USER) list sees neither.
        assert!(store.list_boards().await.unwrap().is_empty());

        // Per-user positions restart at 0 (max is scoped to the user).
        assert_eq!(alice_board.position, 0);
        assert_eq!(bob_board.position, 0);
    }

    #[tokio::test]
    async fn user_cannot_access_anothers_board() {
        let store = mem_store().await;
        let now = datetime!(2026-06-15 12:00:00 UTC);
        let alice_board = store.create_board_for("alice", "secret", now).await.unwrap();

        // Bob can't see, rename, or delete Alice's board.
        assert!(store
            .get_board_for("bob", alice_board.id)
            .await
            .unwrap()
            .is_none());
        assert!(matches!(
            store.rename_board_for("bob", alice_board.id, "hax", now).await,
            Err(CoreError::NotFound(_))
        ));
        assert!(matches!(
            store.delete_board_for("bob", alice_board.id, now).await,
            Err(CoreError::NotFound(_))
        ));
        // Bob also can't create a column under Alice's board.
        assert!(matches!(
            store
                .create_column_for("bob", alice_board.id, "x", &serde_json::json!({}), now)
                .await,
            Err(CoreError::NotFound(_))
        ));

        // Alice's board is untouched and still hers.
        let still = store.get_board_for("alice", alice_board.id).await.unwrap();
        assert_eq!(still.unwrap().name, "secret");
    }

    #[tokio::test]
    async fn tasks_are_isolated_per_user() {
        let store = mem_store().await;
        let now = datetime!(2026-06-15 12:00:00 UTC);
        let a = store
            .create_task_for("alice", TaskDraft::new("a-task"), now)
            .await
            .unwrap();
        store
            .create_task_for("bob", TaskDraft::new("b-task"), now)
            .await
            .unwrap();

        assert_eq!(store.list_tasks_for("alice", TaskFilter::default()).await.unwrap().len(), 1);
        assert_eq!(store.list_tasks_for("bob", TaskFilter::default()).await.unwrap().len(), 1);
        // Default (LOCAL_USER) sees neither.
        assert!(store.list_tasks(TaskFilter::default()).await.unwrap().is_empty());

        // Bob can't get or update or delete Alice's task.
        assert!(store.get_task_for("bob", a.id).await.unwrap().is_none());
        assert!(matches!(
            store
                .update_task_for("bob", a.id, TaskPatch { title: Some("x".into()), ..Default::default() }, now)
                .await,
            Err(CoreError::NotFound(_))
        ));
        assert!(matches!(
            store.delete_task_for("bob", a.id, now).await,
            Err(CoreError::NotFound(_))
        ));
    }

    // --- columns --------------------------------------------------------------

    #[tokio::test]
    async fn create_update_delete_columns() {
        let store = mem_store().await;
        let now = datetime!(2026-06-15 12:00:00 UTC);
        let board = store.create_board("b", now).await.unwrap();

        let filter = serde_json::json!({ "state": "open" });
        let col = store
            .create_column(board.id, "Open", &filter, now)
            .await
            .unwrap();
        assert_eq!(col.position, 0);
        assert_eq!(col.filter, filter);

        let col2 = store
            .create_column(board.id, "Done", &serde_json::json!({}), now)
            .await
            .unwrap();
        assert_eq!(col2.position, 1);

        let new_filter = serde_json::json!({ "state": "closed" });
        let updated = store
            .update_column(col.id, "Open (edited)", &new_filter, now)
            .await
            .unwrap();
        assert_eq!(updated.name, "Open (edited)");
        assert_eq!(updated.filter, new_filter);

        store.delete_column(col.id, now).await.unwrap();
        let cols = store.list_columns(board.id).await.unwrap();
        assert_eq!(cols.len(), 1);
        assert_eq!(cols[0].id, col2.id);
    }

    #[tokio::test]
    async fn delete_column_removes_its_cards() {
        let store = mem_store().await;
        let now = datetime!(2026-06-15 12:00:00 UTC);
        let board = store.create_board("b", now).await.unwrap();
        let col = store
            .create_column(board.id, "c", &serde_json::json!({}), now)
            .await
            .unwrap();
        store
            .place_card(board.id, col.id, "todo:1", now)
            .await
            .unwrap();

        store.delete_column(col.id, now).await.unwrap();
        assert!(store.list_cards(board.id).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn reorder_columns_sets_positions() {
        let store = mem_store().await;
        let now = datetime!(2026-06-15 12:00:00 UTC);
        let board = store.create_board("b", now).await.unwrap();
        let a = store
            .create_column(board.id, "a", &serde_json::json!({}), now)
            .await
            .unwrap();
        let b = store
            .create_column(board.id, "b", &serde_json::json!({}), now)
            .await
            .unwrap();

        store
            .reorder_columns(board.id, &[b.id, a.id], now)
            .await
            .unwrap();
        let cols = store.list_columns(board.id).await.unwrap();
        assert_eq!(
            cols.iter().map(|x| x.id).collect::<Vec<_>>(),
            vec![b.id, a.id]
        );
    }

    // --- cards ----------------------------------------------------------------

    #[tokio::test]
    async fn place_card_upsert_moves_on_conflict() {
        let store = mem_store().await;
        let now = datetime!(2026-06-15 12:00:00 UTC);
        let board = store.create_board("b", now).await.unwrap();
        let col_a = store
            .create_column(board.id, "A", &serde_json::json!({}), now)
            .await
            .unwrap();
        let col_b = store
            .create_column(board.id, "B", &serde_json::json!({}), now)
            .await
            .unwrap();

        let first = store
            .place_card(board.id, col_a.id, "todo:1", now)
            .await
            .unwrap();
        assert_eq!(first.column_id, col_a.id);

        // Placing the same item_key again moves it (same row), not a second card.
        let moved = store
            .place_card(board.id, col_b.id, "todo:1", now)
            .await
            .unwrap();
        assert_eq!(moved.id, first.id, "same card row (unique board+item_key)");
        assert_eq!(moved.column_id, col_b.id);

        let cards = store.list_cards(board.id).await.unwrap();
        assert_eq!(cards.len(), 1, "no duplicate card on conflict");
    }

    #[tokio::test]
    async fn remove_card_deletes_and_is_idempotent() {
        let store = mem_store().await;
        let now = datetime!(2026-06-15 12:00:00 UTC);
        let board = store.create_board("b", now).await.unwrap();
        let col = store
            .create_column(board.id, "c", &serde_json::json!({}), now)
            .await
            .unwrap();
        store
            .place_card(board.id, col.id, "todo:1", now)
            .await
            .unwrap();

        store.remove_card(board.id, "todo:1", now).await.unwrap();
        assert!(store.list_cards(board.id).await.unwrap().is_empty());
        // Idempotent: removing again does not error.
        store.remove_card(board.id, "todo:1", now).await.unwrap();
    }

    // --- repo snapshots -------------------------------------------------------

    #[tokio::test]
    async fn upsert_snapshot_inserts_then_overwrites_same_day() {
        let store = mem_store().await;
        let acct = "acct-1";

        let first = store
            .upsert_snapshot(acct, "o/alpha", "2026-06-16", 5, 1, 3)
            .await
            .unwrap();
        assert_eq!(first.stars, 5);
        assert_eq!(first.forks, 1);
        assert_eq!(first.open_issues, 3);

        // Re-capture the SAME day → overwrites with latest numbers, still one row.
        let second = store
            .upsert_snapshot(acct, "o/alpha", "2026-06-16", 9, 2, 1)
            .await
            .unwrap();
        assert_eq!(second.stars, 9);
        assert_eq!(second.forks, 2);
        assert_eq!(second.open_issues, 1);

        let all = store.list_snapshots(acct, None).await.unwrap();
        assert_eq!(all.len(), 1, "same (account,repo,day) collapses to one row");
        assert_eq!(all[0].stars, 9);
    }

    #[tokio::test]
    async fn list_snapshots_orders_and_filters_by_since() {
        let store = mem_store().await;
        let acct = "acct-1";
        store
            .upsert_snapshot(acct, "o/beta", "2026-06-15", 1, 0, 0)
            .await
            .unwrap();
        store
            .upsert_snapshot(acct, "o/alpha", "2026-06-15", 2, 0, 0)
            .await
            .unwrap();
        store
            .upsert_snapshot(acct, "o/alpha", "2026-06-16", 3, 0, 0)
            .await
            .unwrap();
        // Different account should never leak in.
        store
            .upsert_snapshot("other", "x/y", "2026-06-16", 99, 0, 0)
            .await
            .unwrap();

        // Ordered by day asc then repo_full_name asc.
        let all = store.list_snapshots(acct, None).await.unwrap();
        let keys: Vec<(&str, &str)> = all
            .iter()
            .map(|s| (s.day.as_str(), s.repo_full_name.as_str()))
            .collect();
        assert_eq!(
            keys,
            vec![
                ("2026-06-15", "o/alpha"),
                ("2026-06-15", "o/beta"),
                ("2026-06-16", "o/alpha"),
            ]
        );

        // since filter is inclusive (day >= since_day).
        let recent = store
            .list_snapshots(acct, Some("2026-06-16"))
            .await
            .unwrap();
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].day, "2026-06-16");
    }

    #[tokio::test]
    async fn latest_two_days_returns_two_most_recent_distinct() {
        let store = mem_store().await;
        let acct = "acct-1";
        for day in ["2026-06-14", "2026-06-15", "2026-06-16"] {
            store
                .upsert_snapshot(acct, "o/alpha", day, 1, 0, 0)
                .await
                .unwrap();
            store
                .upsert_snapshot(acct, "o/beta", day, 1, 0, 0)
                .await
                .unwrap();
        }
        let days = store.latest_two_days(acct).await.unwrap();
        assert_eq!(days, vec!["2026-06-16", "2026-06-15"]);

        // With a single day of data, only one is returned.
        let store2 = mem_store().await;
        store2
            .upsert_snapshot("a", "o/x", "2026-06-16", 1, 0, 0)
            .await
            .unwrap();
        assert_eq!(
            store2.latest_two_days("a").await.unwrap(),
            vec!["2026-06-16"]
        );
    }
}
