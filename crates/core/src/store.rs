use sqlx::sqlite::SqlitePoolOptions;
use sqlx::Row;
use sqlx::SqlitePool;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::account::{Account, AccountDraft, ProviderKind};
use crate::board::{Board, BoardCard, BoardColumn};
use crate::domain::{SourceRef, Task, TaskDraft, TaskFilter, TaskPatch, TaskStatus};
use crate::provider::RemoteTask;
use crate::CoreError;

pub struct Store {
    pool: SqlitePool,
}

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
    pub async fn connect(url: &str) -> Result<Self, CoreError> {
        let mut options = SqlitePoolOptions::new();
        // In-memory SQLite gives each connection its OWN database, so a multi-connection
        // pool would hand out empty/un-migrated databases. Pin in-memory to one connection.
        if url.contains(":memory:") {
            options = options.max_connections(1);
        }
        let pool = options.connect(url).await?;
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .map_err(|e| CoreError::Db(sqlx::Error::Migrate(Box::new(e))))?;
        Ok(Self { pool })
    }

    pub async fn create_task(
        &self,
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

        sqlx::query(
            "INSERT INTO tasks (id, title, body, status, labels, due_at, local_updated_at, dirty, deleted)
             VALUES (?, ?, ?, 'open', ?, ?, ?, 0, 0)",
        )
        .bind(id.to_string())
        .bind(&draft.title)
        .bind(&draft.body)
        .bind(&labels_json)
        .bind(&due)
        .bind(&updated)
        .execute(&self.pool)
        .await?;

        self.get_task(id).await?.ok_or(CoreError::NotFound(id))
    }

    pub async fn get_task(&self, id: Uuid) -> Result<Option<Task>, CoreError> {
        let row = sqlx::query("SELECT * FROM tasks WHERE id = ?")
            .bind(id.to_string())
            .fetch_optional(&self.pool)
            .await?;
        row.map(map_row).transpose()
    }

    pub async fn list_tasks(&self, filter: TaskFilter) -> Result<Vec<Task>, CoreError> {
        let mut sql = String::from("SELECT * FROM tasks WHERE 1 = 1");
        if !filter.include_deleted {
            sql.push_str(" AND deleted = 0");
        }
        if filter.status.is_some() {
            sql.push_str(" AND status = ?");
        }
        if filter.label.is_some() {
            sql.push_str(" AND labels LIKE ?");
        }
        if filter.query.is_some() {
            sql.push_str(" AND (title LIKE ? OR body LIKE ?)");
        }
        sql.push_str(" ORDER BY local_updated_at DESC");

        let mut q = sqlx::query(&sql);
        if let Some(status) = filter.status {
            q = q.bind(status.as_str().to_string());
        }
        if let Some(label) = &filter.label {
            // Approximate JSON-array membership: matches the quoted label as a substring of
            // the labels JSON. Good enough for local todos; does not escape LIKE wildcards
            // (% _) or quotes in the label value. Exact membership would use json_each().
            q = q.bind(format!("%\"{}\"%", label));
        }
        if let Some(query) = &filter.query {
            let like = format!("%{}%", query);
            q = q.bind(like.clone()).bind(like);
        }

        let rows = q.fetch_all(&self.pool).await?;
        rows.into_iter().map(map_row).collect()
    }

    pub async fn update_task(
        &self,
        id: Uuid,
        patch: TaskPatch,
        now: OffsetDateTime,
    ) -> Result<Task, CoreError> {
        let mut current = self.get_task(id).await?.ok_or(CoreError::NotFound(id))?;

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

        sqlx::query(
            "UPDATE tasks SET title = ?, body = ?, status = ?, labels = ?, due_at = ?,
                              local_updated_at = ?, dirty = 1 WHERE id = ?",
        )
        .bind(&current.title)
        .bind(&current.body)
        .bind(current.status.as_str())
        .bind(&labels_json)
        .bind(&due)
        .bind(&updated)
        .bind(id.to_string())
        .execute(&self.pool)
        .await?;

        self.get_task(id).await?.ok_or(CoreError::NotFound(id))
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
        let existing =
            sqlx::query("SELECT id, dirty FROM tasks WHERE account_id = ? AND remote_id = ?")
                .bind(account_id.to_string())
                .bind(&remote.remote_id)
                .fetch_optional(&self.pool)
                .await?;

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
                sqlx::query(
                    "INSERT INTO tasks (id, account_id, remote_id, html_url, title, body, status, labels,
                                        remote_updated_at, local_updated_at, dirty, deleted)
                     VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 0, 0)",
                )
                .bind(id.to_string())
                .bind(account_id.to_string())
                .bind(&remote.remote_id)
                .bind(&remote.html_url)
                .bind(&remote.title)
                .bind(&remote.body)
                .bind(remote.status.as_str())
                .bind(&labels_json)
                .bind(&r_updated)
                .bind(&now_s)
                .execute(&self.pool)
                .await?;
                self.get_task(id).await?.ok_or(CoreError::NotFound(id))
            }
            Some(row) => {
                let id = Uuid::parse_str(&row.try_get::<String, _>("id")?)
                    .map_err(|_| CoreError::DataFormat("id"))?;
                let dirty: i64 = row.try_get("dirty")?;
                if dirty != 0 {
                    // local edits pending → don't overwrite; just return current row
                    return self.get_task(id).await?.ok_or(CoreError::NotFound(id));
                }
                sqlx::query(
                    "UPDATE tasks SET html_url = ?, title = ?, body = ?, status = ?, labels = ?,
                                      remote_updated_at = ?, local_updated_at = ? WHERE id = ?",
                )
                .bind(&remote.html_url)
                .bind(&remote.title)
                .bind(&remote.body)
                .bind(remote.status.as_str())
                .bind(&labels_json)
                .bind(&r_updated)
                .bind(&now_s)
                .bind(id.to_string())
                .execute(&self.pool)
                .await?;
                self.get_task(id).await?.ok_or(CoreError::NotFound(id))
            }
        }
    }

    /// Tasks with unpushed local edits for an account (have a source on that account).
    pub async fn list_dirty(&self, account_id: Uuid) -> Result<Vec<Task>, CoreError> {
        let rows = sqlx::query("SELECT * FROM tasks WHERE account_id = ? AND dirty = 1")
            .bind(account_id.to_string())
            .fetch_all(&self.pool)
            .await?;
        rows.into_iter().map(map_row).collect()
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
        let res = sqlx::query(
            "UPDATE tasks SET dirty = 0, remote_updated_at = ?, local_updated_at = ? WHERE id = ?",
        )
        .bind(&r)
        .bind(&n)
        .bind(id.to_string())
        .execute(&self.pool)
        .await?;
        if res.rows_affected() == 0 {
            return Err(CoreError::NotFound(id));
        }
        Ok(())
    }

    pub async fn delete_task(&self, id: Uuid, now: OffsetDateTime) -> Result<(), CoreError> {
        let updated = now
            .format(&Rfc3339)
            .map_err(|_| CoreError::DataFormat("now"))?;
        let result = sqlx::query(
            "UPDATE tasks SET deleted = 1, dirty = 1, local_updated_at = ? WHERE id = ?",
        )
        .bind(&updated)
        .bind(id.to_string())
        .execute(&self.pool)
        .await?;
        if result.rows_affected() == 0 {
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

        sqlx::query(
            "INSERT INTO accounts (id, provider, display_name, base_url, config, created_at)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(id.to_string())
        .bind(draft.provider.as_str())
        .bind(&draft.display_name)
        .bind(&draft.base_url)
        .bind(&config_json)
        .bind(&created)
        .execute(&self.pool)
        .await?;

        self.get_account(id).await?.ok_or(CoreError::NotFound(id))
    }

    pub async fn get_account(&self, id: Uuid) -> Result<Option<Account>, CoreError> {
        let row = sqlx::query("SELECT * FROM accounts WHERE id = ?")
            .bind(id.to_string())
            .fetch_optional(&self.pool)
            .await?;
        row.map(map_account_row).transpose()
    }

    pub async fn list_accounts(&self) -> Result<Vec<Account>, CoreError> {
        let rows = sqlx::query("SELECT * FROM accounts ORDER BY created_at DESC")
            .fetch_all(&self.pool)
            .await?;
        rows.into_iter().map(map_account_row).collect()
    }

    pub async fn delete_account(&self, id: Uuid) -> Result<(), CoreError> {
        let result = sqlx::query("DELETE FROM accounts WHERE id = ?")
            .bind(id.to_string())
            .execute(&self.pool)
            .await?;
        if result.rows_affected() == 0 {
            return Err(CoreError::NotFound(id));
        }
        Ok(())
    }

    // --- boards ---------------------------------------------------------------
    //
    // The sqlx sqlite pool does NOT enable `PRAGMA foreign_keys=ON`, so the
    // `ON DELETE CASCADE` declarations in the schema are inert. Cascades are
    // therefore replicated manually in `delete_board` / `delete_column`.

    pub async fn create_board(
        &self,
        name: &str,
        now: OffsetDateTime,
    ) -> Result<Board, CoreError> {
        let id = Uuid::new_v4();
        let now_s = now
            .format(&Rfc3339)
            .map_err(|_| CoreError::DataFormat("now"))?;
        let position: i64 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(position) + 1, 0) FROM boards",
        )
        .fetch_one(&self.pool)
        .await?;

        sqlx::query(
            "INSERT INTO boards (id, name, position, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(id.to_string())
        .bind(name)
        .bind(position)
        .bind(&now_s)
        .bind(&now_s)
        .execute(&self.pool)
        .await?;

        self.get_board(id).await?.ok_or(CoreError::NotFound(id))
    }

    pub async fn get_board(&self, id: Uuid) -> Result<Option<Board>, CoreError> {
        let row = sqlx::query("SELECT * FROM boards WHERE id = ?")
            .bind(id.to_string())
            .fetch_optional(&self.pool)
            .await?;
        row.map(map_board_row).transpose()
    }

    pub async fn list_boards(&self) -> Result<Vec<Board>, CoreError> {
        let rows = sqlx::query("SELECT * FROM boards ORDER BY position ASC, created_at ASC")
            .fetch_all(&self.pool)
            .await?;
        rows.into_iter().map(map_board_row).collect()
    }

    pub async fn rename_board(
        &self,
        id: Uuid,
        name: &str,
        now: OffsetDateTime,
    ) -> Result<Board, CoreError> {
        let now_s = now
            .format(&Rfc3339)
            .map_err(|_| CoreError::DataFormat("now"))?;
        let res = sqlx::query("UPDATE boards SET name = ?, updated_at = ? WHERE id = ?")
            .bind(name)
            .bind(&now_s)
            .bind(id.to_string())
            .execute(&self.pool)
            .await?;
        if res.rows_affected() == 0 {
            return Err(CoreError::NotFound(id));
        }
        self.get_board(id).await?.ok_or(CoreError::NotFound(id))
    }

    pub async fn delete_board(&self, id: Uuid) -> Result<(), CoreError> {
        // Manual cascade: cards then columns then the board itself.
        sqlx::query("DELETE FROM board_cards WHERE board_id = ?")
            .bind(id.to_string())
            .execute(&self.pool)
            .await?;
        sqlx::query("DELETE FROM board_columns WHERE board_id = ?")
            .bind(id.to_string())
            .execute(&self.pool)
            .await?;
        let res = sqlx::query("DELETE FROM boards WHERE id = ?")
            .bind(id.to_string())
            .execute(&self.pool)
            .await?;
        if res.rows_affected() == 0 {
            return Err(CoreError::NotFound(id));
        }
        Ok(())
    }

    /// Reorder boards so their `position` matches the order of `ordered_ids`.
    /// Ids not present keep their existing position; unknown ids are ignored.
    pub async fn reorder_boards(
        &self,
        ordered_ids: &[Uuid],
        now: OffsetDateTime,
    ) -> Result<(), CoreError> {
        let now_s = now
            .format(&Rfc3339)
            .map_err(|_| CoreError::DataFormat("now"))?;
        for (idx, id) in ordered_ids.iter().enumerate() {
            sqlx::query("UPDATE boards SET position = ?, updated_at = ? WHERE id = ?")
                .bind(idx as i64)
                .bind(&now_s)
                .bind(id.to_string())
                .execute(&self.pool)
                .await?;
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
        let id = Uuid::new_v4();
        let now_s = now
            .format(&Rfc3339)
            .map_err(|_| CoreError::DataFormat("now"))?;
        let filter_json = serde_json::to_string(filter)?;
        let position: i64 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(position) + 1, 0) FROM board_columns WHERE board_id = ?",
        )
        .bind(board_id.to_string())
        .fetch_one(&self.pool)
        .await?;

        sqlx::query(
            "INSERT INTO board_columns (id, board_id, name, position, filter, created_at)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(id.to_string())
        .bind(board_id.to_string())
        .bind(name)
        .bind(position)
        .bind(&filter_json)
        .bind(&now_s)
        .execute(&self.pool)
        .await?;

        self.touch_board(board_id, now).await?;
        self.get_column(id).await?.ok_or(CoreError::NotFound(id))
    }

    pub async fn get_column(&self, id: Uuid) -> Result<Option<BoardColumn>, CoreError> {
        let row = sqlx::query("SELECT * FROM board_columns WHERE id = ?")
            .bind(id.to_string())
            .fetch_optional(&self.pool)
            .await?;
        row.map(map_column_row).transpose()
    }

    pub async fn list_columns(&self, board_id: Uuid) -> Result<Vec<BoardColumn>, CoreError> {
        let rows = sqlx::query(
            "SELECT * FROM board_columns WHERE board_id = ?
             ORDER BY position ASC, created_at ASC",
        )
        .bind(board_id.to_string())
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(map_column_row).collect()
    }

    pub async fn update_column(
        &self,
        id: Uuid,
        name: &str,
        filter: &serde_json::Value,
        now: OffsetDateTime,
    ) -> Result<BoardColumn, CoreError> {
        let filter_json = serde_json::to_string(filter)?;
        let res = sqlx::query("UPDATE board_columns SET name = ?, filter = ? WHERE id = ?")
            .bind(name)
            .bind(&filter_json)
            .bind(id.to_string())
            .execute(&self.pool)
            .await?;
        if res.rows_affected() == 0 {
            return Err(CoreError::NotFound(id));
        }
        let column = self.get_column(id).await?.ok_or(CoreError::NotFound(id))?;
        self.touch_board(column.board_id, now).await?;
        Ok(column)
    }

    pub async fn delete_column(&self, id: Uuid, now: OffsetDateTime) -> Result<(), CoreError> {
        // Resolve the owning board first so we can touch it afterwards.
        let board_id = sqlx::query_scalar::<_, String>(
            "SELECT board_id FROM board_columns WHERE id = ?",
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await?
        .ok_or(CoreError::NotFound(id))?;

        // Manual cascade: drop the column's manual cards first.
        sqlx::query("DELETE FROM board_cards WHERE column_id = ?")
            .bind(id.to_string())
            .execute(&self.pool)
            .await?;
        sqlx::query("DELETE FROM board_columns WHERE id = ?")
            .bind(id.to_string())
            .execute(&self.pool)
            .await?;

        if let Ok(board_uuid) = Uuid::parse_str(&board_id) {
            self.touch_board(board_uuid, now).await?;
        }
        Ok(())
    }

    /// Reorder columns within a board so their `position` matches `ordered_ids`.
    pub async fn reorder_columns(
        &self,
        board_id: Uuid,
        ordered_ids: &[Uuid],
        now: OffsetDateTime,
    ) -> Result<(), CoreError> {
        for (idx, id) in ordered_ids.iter().enumerate() {
            sqlx::query("UPDATE board_columns SET position = ? WHERE id = ? AND board_id = ?")
                .bind(idx as i64)
                .bind(id.to_string())
                .bind(board_id.to_string())
                .execute(&self.pool)
                .await?;
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
        let now_s = now
            .format(&Rfc3339)
            .map_err(|_| CoreError::DataFormat("now"))?;

        let existing = sqlx::query_scalar::<_, String>(
            "SELECT id FROM board_cards WHERE board_id = ? AND item_key = ?",
        )
        .bind(board_id.to_string())
        .bind(item_key)
        .fetch_optional(&self.pool)
        .await?;

        let id = match existing {
            Some(existing_id) => {
                // Already placed on this board → move it to the target column.
                let position: i64 = sqlx::query_scalar(
                    "SELECT COALESCE(MAX(position) + 1, 0) FROM board_cards WHERE column_id = ?",
                )
                .bind(column_id.to_string())
                .fetch_one(&self.pool)
                .await?;
                sqlx::query(
                    "UPDATE board_cards SET column_id = ?, position = ? WHERE id = ?",
                )
                .bind(column_id.to_string())
                .bind(position)
                .bind(&existing_id)
                .execute(&self.pool)
                .await?;
                Uuid::parse_str(&existing_id).map_err(|_| CoreError::DataFormat("id"))?
            }
            None => {
                let new_id = Uuid::new_v4();
                let position: i64 = sqlx::query_scalar(
                    "SELECT COALESCE(MAX(position) + 1, 0) FROM board_cards WHERE column_id = ?",
                )
                .bind(column_id.to_string())
                .fetch_one(&self.pool)
                .await?;
                sqlx::query(
                    "INSERT INTO board_cards (id, board_id, column_id, item_key, position, created_at)
                     VALUES (?, ?, ?, ?, ?, ?)",
                )
                .bind(new_id.to_string())
                .bind(board_id.to_string())
                .bind(column_id.to_string())
                .bind(item_key)
                .bind(position)
                .bind(&now_s)
                .execute(&self.pool)
                .await?;
                new_id
            }
        };

        self.touch_board(board_id, now).await?;
        self.get_card(id).await?.ok_or(CoreError::NotFound(id))
    }

    pub async fn get_card(&self, id: Uuid) -> Result<Option<BoardCard>, CoreError> {
        let row = sqlx::query("SELECT * FROM board_cards WHERE id = ?")
            .bind(id.to_string())
            .fetch_optional(&self.pool)
            .await?;
        row.map(map_card_row).transpose()
    }

    pub async fn list_cards(&self, board_id: Uuid) -> Result<Vec<BoardCard>, CoreError> {
        let rows = sqlx::query(
            "SELECT * FROM board_cards WHERE board_id = ?
             ORDER BY position ASC, created_at ASC",
        )
        .bind(board_id.to_string())
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(map_card_row).collect()
    }

    /// Remove a manually placed card. Idempotent: removing an absent item is a no-op.
    pub async fn remove_card(
        &self,
        board_id: Uuid,
        item_key: &str,
        now: OffsetDateTime,
    ) -> Result<(), CoreError> {
        let res = sqlx::query("DELETE FROM board_cards WHERE board_id = ? AND item_key = ?")
            .bind(board_id.to_string())
            .bind(item_key)
            .execute(&self.pool)
            .await?;
        if res.rows_affected() > 0 {
            self.touch_board(board_id, now).await?;
        }
        Ok(())
    }

    /// Best-effort bump of a board's `updated_at` when its columns/cards/name change.
    async fn touch_board(&self, board_id: Uuid, now: OffsetDateTime) -> Result<(), CoreError> {
        let now_s = now
            .format(&Rfc3339)
            .map_err(|_| CoreError::DataFormat("now"))?;
        sqlx::query("UPDATE boards SET updated_at = ? WHERE id = ?")
            .bind(&now_s)
            .bind(board_id.to_string())
            .execute(&self.pool)
            .await?;
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

        // INSERT OR REPLACE on the unique (account_id, repo_full_name, day) index.
        sqlx::query(
            "INSERT INTO repo_snapshots
                (id, account_id, repo_full_name, day, stars, forks, open_issues, captured_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(account_id, repo_full_name, day) DO UPDATE SET
                id          = excluded.id,
                stars       = excluded.stars,
                forks       = excluded.forks,
                open_issues = excluded.open_issues,
                captured_at = excluded.captured_at",
        )
        .bind(&id)
        .bind(account_id)
        .bind(repo_full_name)
        .bind(day)
        .bind(stars)
        .bind(forks)
        .bind(open_issues)
        .bind(&captured_at)
        .execute(&self.pool)
        .await?;

        let row = sqlx::query(
            "SELECT * FROM repo_snapshots
             WHERE account_id = ? AND repo_full_name = ? AND day = ?",
        )
        .bind(account_id)
        .bind(repo_full_name)
        .bind(day)
        .fetch_one(&self.pool)
        .await?;
        map_snapshot_row(row)
    }

    /// All snapshots for an account, optionally filtered to `day >= since_day`,
    /// ordered by day ascending then repo_full_name ascending.
    pub async fn list_snapshots(
        &self,
        account_id: &str,
        since_day: Option<&str>,
    ) -> Result<Vec<RepoSnapshot>, CoreError> {
        let rows = match since_day {
            Some(since) => {
                sqlx::query(
                    "SELECT * FROM repo_snapshots
                     WHERE account_id = ? AND day >= ?
                     ORDER BY day ASC, repo_full_name ASC",
                )
                .bind(account_id)
                .bind(since)
                .fetch_all(&self.pool)
                .await?
            }
            None => {
                sqlx::query(
                    "SELECT * FROM repo_snapshots
                     WHERE account_id = ?
                     ORDER BY day ASC, repo_full_name ASC",
                )
                .bind(account_id)
                .fetch_all(&self.pool)
                .await?
            }
        };
        rows.into_iter().map(map_snapshot_row).collect()
    }

    /// The two most recent distinct `day` values for an account, most-recent first.
    /// Returns 0, 1, or 2 entries. Used to diff today's capture against the previous one.
    pub async fn latest_two_days(&self, account_id: &str) -> Result<Vec<String>, CoreError> {
        let rows = sqlx::query(
            "SELECT DISTINCT day FROM repo_snapshots
             WHERE account_id = ?
             ORDER BY day DESC
             LIMIT 2",
        )
        .bind(account_id)
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter()
            .map(|r| r.try_get::<String, _>("day").map_err(CoreError::from))
            .collect()
    }
}

fn map_snapshot_row(row: sqlx::sqlite::SqliteRow) -> Result<RepoSnapshot, CoreError> {
    Ok(RepoSnapshot {
        id: row.try_get("id")?,
        account_id: row.try_get("account_id")?,
        repo_full_name: row.try_get("repo_full_name")?,
        day: row.try_get("day")?,
        stars: row.try_get("stars")?,
        forks: row.try_get("forks")?,
        open_issues: row.try_get("open_issues")?,
        captured_at: row.try_get("captured_at")?,
    })
}

fn map_account_row(row: sqlx::sqlite::SqliteRow) -> Result<Account, CoreError> {
    let id: String = row.try_get("id")?;
    let provider: String = row.try_get("provider")?;
    let display_name: String = row.try_get("display_name")?;
    let base_url: Option<String> = row.try_get("base_url")?;
    let config: String = row.try_get("config")?;
    let created_at: String = row.try_get("created_at")?;

    Ok(Account {
        id: Uuid::parse_str(&id).map_err(|_| CoreError::DataFormat("id"))?,
        provider: ProviderKind::parse(&provider).ok_or(CoreError::DataFormat("provider"))?,
        display_name,
        base_url,
        config: serde_json::from_str(&config)?,
        created_at: OffsetDateTime::parse(&created_at, &Rfc3339)
            .map_err(|_| CoreError::DataFormat("created_at"))?,
    })
}

fn map_row(row: sqlx::sqlite::SqliteRow) -> Result<Task, CoreError> {
    let id: String = row.try_get("id")?;
    let account_id: Option<String> = row.try_get("account_id")?;
    let remote_id: Option<String> = row.try_get("remote_id")?;
    let html_url: Option<String> = row.try_get("html_url")?;
    let title: String = row.try_get("title")?;
    let body: String = row.try_get("body")?;
    let status: String = row.try_get("status")?;
    let labels: String = row.try_get("labels")?;
    let due_at: Option<String> = row.try_get("due_at")?;
    let local_updated_at: String = row.try_get("local_updated_at")?;
    let dirty: i64 = row.try_get("dirty")?;
    let deleted: i64 = row.try_get("deleted")?;

    let source = match (account_id, remote_id) {
        (Some(account_id), Some(remote_id)) => Some(SourceRef {
            account_id: Uuid::parse_str(&account_id)
                .map_err(|_| CoreError::DataFormat("account_id"))?,
            remote_id,
            html_url,
        }),
        _ => None,
    };

    let parse_dt =
        |s: &str| OffsetDateTime::parse(s, &Rfc3339).map_err(|_| CoreError::DataFormat("datetime"));

    Ok(Task {
        id: Uuid::parse_str(&id).map_err(|_| CoreError::DataFormat("id"))?,
        source,
        title,
        body,
        status: TaskStatus::from_str(&status).ok_or(CoreError::DataFormat("status"))?,
        labels: serde_json::from_str(&labels)?,
        due_at: due_at.as_deref().map(parse_dt).transpose()?,
        local_updated_at: parse_dt(&local_updated_at)?,
        dirty: dirty != 0,
        deleted: deleted != 0,
    })
}

fn map_board_row(row: sqlx::sqlite::SqliteRow) -> Result<Board, CoreError> {
    let id: String = row.try_get("id")?;
    let name: String = row.try_get("name")?;
    let position: i64 = row.try_get("position")?;
    let created_at: String = row.try_get("created_at")?;
    let updated_at: String = row.try_get("updated_at")?;

    Ok(Board {
        id: Uuid::parse_str(&id).map_err(|_| CoreError::DataFormat("id"))?,
        name,
        position,
        created_at: OffsetDateTime::parse(&created_at, &Rfc3339)
            .map_err(|_| CoreError::DataFormat("created_at"))?,
        updated_at: OffsetDateTime::parse(&updated_at, &Rfc3339)
            .map_err(|_| CoreError::DataFormat("updated_at"))?,
    })
}

fn map_column_row(row: sqlx::sqlite::SqliteRow) -> Result<BoardColumn, CoreError> {
    let id: String = row.try_get("id")?;
    let board_id: String = row.try_get("board_id")?;
    let name: String = row.try_get("name")?;
    let position: i64 = row.try_get("position")?;
    let filter: String = row.try_get("filter")?;
    let created_at: String = row.try_get("created_at")?;

    Ok(BoardColumn {
        id: Uuid::parse_str(&id).map_err(|_| CoreError::DataFormat("id"))?,
        board_id: Uuid::parse_str(&board_id).map_err(|_| CoreError::DataFormat("board_id"))?,
        name,
        position,
        filter: serde_json::from_str(&filter)?,
        created_at: OffsetDateTime::parse(&created_at, &Rfc3339)
            .map_err(|_| CoreError::DataFormat("created_at"))?,
    })
}

fn map_card_row(row: sqlx::sqlite::SqliteRow) -> Result<BoardCard, CoreError> {
    let id: String = row.try_get("id")?;
    let board_id: String = row.try_get("board_id")?;
    let column_id: String = row.try_get("column_id")?;
    let item_key: String = row.try_get("item_key")?;
    let position: i64 = row.try_get("position")?;
    let created_at: String = row.try_get("created_at")?;

    Ok(BoardCard {
        id: Uuid::parse_str(&id).map_err(|_| CoreError::DataFormat("id"))?,
        board_id: Uuid::parse_str(&board_id).map_err(|_| CoreError::DataFormat("board_id"))?,
        column_id: Uuid::parse_str(&column_id).map_err(|_| CoreError::DataFormat("column_id"))?,
        item_key,
        position,
        created_at: OffsetDateTime::parse(&created_at, &Rfc3339)
            .map_err(|_| CoreError::DataFormat("created_at"))?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::account::{AccountDraft, ProviderKind};
    use crate::domain::{TaskDraft, TaskFilter, TaskPatch, TaskStatus};
    use crate::provider::RemoteTask;
    use time::macros::datetime;

    async fn mem_store() -> Store {
        Store::connect("sqlite::memory:").await.unwrap()
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
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tasks")
            .fetch_one(&store.pool)
            .await
            .unwrap();
        assert_eq!(count, 0);
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
        sqlx::query("UPDATE tasks SET status = 'done' WHERE id = ?")
            .bind(a.id.to_string())
            .execute(&store.pool)
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

        assert!(store.get_board(board.id).await.unwrap().is_none());
        assert!(store.list_columns(board.id).await.unwrap().is_empty());
        assert!(store.list_cards(board.id).await.unwrap().is_empty());
        // No orphan rows anywhere.
        let orphan_cols: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM board_columns")
            .fetch_one(&store.pool)
            .await
            .unwrap();
        let orphan_cards: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM board_cards")
            .fetch_one(&store.pool)
            .await
            .unwrap();
        assert_eq!(orphan_cols, 0);
        assert_eq!(orphan_cards, 0);
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
