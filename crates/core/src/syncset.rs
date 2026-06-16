//! Cross-device sync set: the LWW change protocol over the four synced entities
//! (`tasks`, `boards`, `board_columns`, `board_cards`).
//!
//! A [`SyncChange`] is the *current state* of one synced row, transport-agnostic:
//! an envelope (`kind`, `id`, `updated_at`, `deleted`, parent `board_id`/`column_id`)
//! plus a `data_json` blob carrying the entity's remaining persisted fields. The
//! reconciliation rule is **last-writer-wins by `updated_at`** (RFC3339 strings,
//! which sort lexicographically in UTC `Z` form).
//!
//! - The client calls [`Store::dirty_changes`] to gather everything it must PUSH,
//!   then [`Store::mark_synced_changes`] once the push succeeds.
//! - The server answers a PULL with [`Store::changes_since`].
//! - Both sides apply incoming changes with [`Store::apply_change`].

use sea_orm::{ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};

use crate::entities::{board_cards, board_columns, boards, tasks};
use crate::store::Store;
use crate::CoreError;

/// The kind of synced entity a [`SyncChange`] describes.
pub const KIND_TASK: &str = "task";
pub const KIND_BOARD: &str = "board";
pub const KIND_COLUMN: &str = "column";
pub const KIND_CARD: &str = "card";

/// The current state of one synced row, as exchanged over the wire.
///
/// `data_json` holds the entity's persisted fields that are NOT already in the
/// envelope (see the per-kind `*Data` structs below); it is `{}`-ish JSON, never
/// empty in practice. `board_id`/`column_id` carry parent linkage for columns and
/// cards (empty for tasks/boards).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncChange {
    pub kind: String,
    pub id: String,
    /// RFC3339 timestamp used as the LWW clock.
    pub updated_at: String,
    /// Tombstone flag: `true` = the row was deleted.
    pub deleted: bool,
    /// serde-JSON of the entity's remaining fields (see `TaskData`/`BoardData`/…).
    pub data_json: String,
    /// Parent board for columns and cards; empty otherwise.
    pub board_id: String,
    /// Parent column for cards; empty otherwise.
    pub column_id: String,
}

// --- per-kind `data_json` payloads ------------------------------------------
//
// Each carries exactly the persisted columns of its entity that aren't already
// represented by the SyncChange envelope (id/updated_at/deleted/board_id/column_id).

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TaskData {
    account_id: Option<String>,
    remote_id: Option<String>,
    html_url: Option<String>,
    title: String,
    body: String,
    status: String,
    labels: String,
    due_at: Option<String>,
    project_id: Option<String>,
    remote_updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BoardData {
    name: String,
    position: i64,
    created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ColumnData {
    name: String,
    position: i64,
    filter: String,
    created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CardData {
    item_key: String,
    position: i64,
    created_at: String,
}

// --- row → SyncChange -------------------------------------------------------

fn task_to_change(m: &tasks::Model) -> Result<SyncChange, CoreError> {
    let data = TaskData {
        account_id: m.account_id.clone(),
        remote_id: m.remote_id.clone(),
        html_url: m.html_url.clone(),
        title: m.title.clone(),
        body: m.body.clone(),
        status: m.status.clone(),
        labels: m.labels.clone(),
        due_at: m.due_at.clone(),
        project_id: m.project_id.clone(),
        remote_updated_at: m.remote_updated_at.clone(),
    };
    Ok(SyncChange {
        kind: KIND_TASK.into(),
        id: m.id.clone(),
        updated_at: m.local_updated_at.clone(),
        deleted: m.deleted != 0,
        data_json: serde_json::to_string(&data)?,
        board_id: String::new(),
        column_id: String::new(),
    })
}

fn board_to_change(m: &boards::Model) -> Result<SyncChange, CoreError> {
    let data = BoardData {
        name: m.name.clone(),
        position: m.position,
        created_at: m.created_at.clone(),
    };
    Ok(SyncChange {
        kind: KIND_BOARD.into(),
        id: m.id.clone(),
        updated_at: m.updated_at.clone(),
        deleted: m.deleted != 0,
        data_json: serde_json::to_string(&data)?,
        board_id: String::new(),
        column_id: String::new(),
    })
}

fn column_to_change(m: &board_columns::Model) -> Result<SyncChange, CoreError> {
    let data = ColumnData {
        name: m.name.clone(),
        position: m.position,
        filter: m.filter.clone(),
        created_at: m.created_at.clone(),
    };
    Ok(SyncChange {
        kind: KIND_COLUMN.into(),
        id: m.id.clone(),
        updated_at: m.updated_at.clone(),
        deleted: m.deleted != 0,
        data_json: serde_json::to_string(&data)?,
        board_id: m.board_id.clone(),
        column_id: String::new(),
    })
}

fn card_to_change(m: &board_cards::Model) -> Result<SyncChange, CoreError> {
    let data = CardData {
        item_key: m.item_key.clone(),
        position: m.position,
        created_at: m.created_at.clone(),
    };
    Ok(SyncChange {
        kind: KIND_CARD.into(),
        id: m.id.clone(),
        updated_at: m.updated_at.clone(),
        deleted: m.deleted != 0,
        data_json: serde_json::to_string(&data)?,
        board_id: m.board_id.clone(),
        column_id: m.column_id.clone(),
    })
}

impl Store {
    /// All locally-mutated rows (`dirty=1`) across the four synced entities for
    /// `user_id`, INCLUDING tombstoned (`deleted=1`) rows so deletes propagate.
    /// This is what a client gathers to PUSH. Columns/cards are scoped via their
    /// board's owner.
    pub async fn dirty_changes(&self, user_id: &str) -> Result<Vec<SyncChange>, CoreError> {
        let conn = self.conn();
        let mut out = Vec::new();

        for m in tasks::Entity::find()
            .filter(tasks::Column::UserId.eq(user_id))
            .filter(tasks::Column::Dirty.eq(1))
            .all(conn)
            .await
            .map_err(CoreError::Db)?
        {
            out.push(task_to_change(&m)?);
        }

        for m in boards::Entity::find()
            .filter(boards::Column::UserId.eq(user_id))
            .filter(boards::Column::Dirty.eq(1))
            .all(conn)
            .await
            .map_err(CoreError::Db)?
        {
            out.push(board_to_change(&m)?);
        }

        let board_ids = self.user_board_ids(user_id).await?;
        if !board_ids.is_empty() {
            for m in board_columns::Entity::find()
                .filter(board_columns::Column::BoardId.is_in(board_ids.clone()))
                .filter(board_columns::Column::Dirty.eq(1))
                .all(conn)
                .await
                .map_err(CoreError::Db)?
            {
                out.push(column_to_change(&m)?);
            }

            for m in board_cards::Entity::find()
                .filter(board_cards::Column::BoardId.is_in(board_ids))
                .filter(board_cards::Column::Dirty.eq(1))
                .all(conn)
                .await
                .map_err(CoreError::Db)?
            {
                out.push(card_to_change(&m)?);
            }
        }

        Ok(out)
    }

    /// All rows (incl. tombstones) for `user_id` whose `updated_at` is strictly
    /// greater than `cursor`, ordered by `updated_at` ascending. `cursor = None`
    /// returns everything. This answers a PULL.
    pub async fn changes_since(
        &self,
        user_id: &str,
        cursor: Option<&str>,
    ) -> Result<Vec<SyncChange>, CoreError> {
        let conn = self.conn();
        let mut out = Vec::new();

        {
            let mut q = tasks::Entity::find().filter(tasks::Column::UserId.eq(user_id));
            if let Some(c) = cursor {
                q = q.filter(tasks::Column::LocalUpdatedAt.gt(c));
            }
            for m in q.all(conn).await.map_err(CoreError::Db)? {
                out.push(task_to_change(&m)?);
            }
        }

        {
            let mut q = boards::Entity::find().filter(boards::Column::UserId.eq(user_id));
            if let Some(c) = cursor {
                q = q.filter(boards::Column::UpdatedAt.gt(c));
            }
            for m in q.all(conn).await.map_err(CoreError::Db)? {
                out.push(board_to_change(&m)?);
            }
        }

        let board_ids = self.user_board_ids(user_id).await?;
        if !board_ids.is_empty() {
            {
                let mut q = board_columns::Entity::find()
                    .filter(board_columns::Column::BoardId.is_in(board_ids.clone()));
                if let Some(c) = cursor {
                    q = q.filter(board_columns::Column::UpdatedAt.gt(c));
                }
                for m in q.all(conn).await.map_err(CoreError::Db)? {
                    out.push(column_to_change(&m)?);
                }
            }
            {
                let mut q = board_cards::Entity::find()
                    .filter(board_cards::Column::BoardId.is_in(board_ids));
                if let Some(c) = cursor {
                    q = q.filter(board_cards::Column::UpdatedAt.gt(c));
                }
                for m in q.all(conn).await.map_err(CoreError::Db)? {
                    out.push(card_to_change(&m)?);
                }
            }
        }

        out.sort_by(|a, b| a.updated_at.cmp(&b.updated_at));
        Ok(out)
    }

    /// LWW upsert of one incoming change into `user_id`'s scope.
    ///
    /// Finds the row by id; writes it iff it doesn't exist locally OR the incoming
    /// `updated_at` is strictly greater than the local one. The written row's
    /// `dirty` is cleared (this *is* synced state) and its `updated_at`/`deleted`
    /// come from the change. Tombstones (`deleted=true`) are stored as tombstones.
    /// For columns/cards the parent board is created-or-verified under `user_id`.
    ///
    /// Returns `true` iff the change was applied (`false` = our copy was newer or
    /// equal, so it was ignored).
    pub async fn apply_change(
        &self,
        user_id: &str,
        change: &SyncChange,
    ) -> Result<bool, CoreError> {
        match change.kind.as_str() {
            KIND_TASK => self.apply_task_change(user_id, change).await,
            KIND_BOARD => self.apply_board_change(user_id, change).await,
            KIND_COLUMN => self.apply_column_change(user_id, change).await,
            KIND_CARD => self.apply_card_change(user_id, change).await,
            _ => Err(CoreError::DataFormat("change.kind")),
        }
    }

    async fn apply_task_change(
        &self,
        user_id: &str,
        change: &SyncChange,
    ) -> Result<bool, CoreError> {
        let conn = self.conn();
        let existing = tasks::Entity::find_by_id(change.id.clone())
            .filter(tasks::Column::UserId.eq(user_id))
            .one(conn)
            .await
            .map_err(CoreError::Db)?;
        if let Some(ref row) = existing {
            if change.updated_at <= row.local_updated_at {
                return Ok(false);
            }
        }
        let data: TaskData = serde_json::from_str(&change.data_json)?;
        let model = tasks::ActiveModel {
            id: Set(change.id.clone()),
            account_id: Set(data.account_id),
            remote_id: Set(data.remote_id),
            html_url: Set(data.html_url),
            title: Set(data.title),
            body: Set(data.body),
            status: Set(data.status),
            labels: Set(data.labels),
            due_at: Set(data.due_at),
            project_id: Set(data.project_id),
            remote_updated_at: Set(data.remote_updated_at),
            local_updated_at: Set(change.updated_at.clone()),
            dirty: Set(0),
            deleted: Set(i64::from(change.deleted)),
            user_id: Set(user_id.to_string()),
        };
        if existing.is_some() {
            tasks::Entity::update(model).exec(conn).await.map_err(CoreError::Db)?;
        } else {
            tasks::Entity::insert(model).exec(conn).await.map_err(CoreError::Db)?;
        }
        Ok(true)
    }

    async fn apply_board_change(
        &self,
        user_id: &str,
        change: &SyncChange,
    ) -> Result<bool, CoreError> {
        let conn = self.conn();
        let existing = boards::Entity::find_by_id(change.id.clone())
            .filter(boards::Column::UserId.eq(user_id))
            .one(conn)
            .await
            .map_err(CoreError::Db)?;
        if let Some(ref row) = existing {
            if change.updated_at <= row.updated_at {
                return Ok(false);
            }
        }
        let data: BoardData = serde_json::from_str(&change.data_json)?;
        let model = boards::ActiveModel {
            id: Set(change.id.clone()),
            name: Set(data.name),
            position: Set(data.position),
            created_at: Set(data.created_at),
            updated_at: Set(change.updated_at.clone()),
            user_id: Set(user_id.to_string()),
            dirty: Set(0),
            deleted: Set(i64::from(change.deleted)),
        };
        if existing.is_some() {
            boards::Entity::update(model).exec(conn).await.map_err(CoreError::Db)?;
        } else {
            boards::Entity::insert(model).exec(conn).await.map_err(CoreError::Db)?;
        }
        Ok(true)
    }

    async fn apply_column_change(
        &self,
        user_id: &str,
        change: &SyncChange,
    ) -> Result<bool, CoreError> {
        let conn = self.conn();
        // Ensure the parent board exists under this user (create a placeholder if
        // the column arrives before its board).
        self.ensure_board_placeholder(user_id, &change.board_id).await?;
        let existing = board_columns::Entity::find_by_id(change.id.clone())
            .one(conn)
            .await
            .map_err(CoreError::Db)?;
        if let Some(ref row) = existing {
            // Only LWW-compare rows under a board owned by this user.
            if row.board_id == change.board_id && change.updated_at <= row.updated_at {
                return Ok(false);
            }
        }
        let data: ColumnData = serde_json::from_str(&change.data_json)?;
        let model = board_columns::ActiveModel {
            id: Set(change.id.clone()),
            board_id: Set(change.board_id.clone()),
            name: Set(data.name),
            position: Set(data.position),
            filter: Set(data.filter),
            created_at: Set(data.created_at),
            updated_at: Set(change.updated_at.clone()),
            dirty: Set(0),
            deleted: Set(i64::from(change.deleted)),
        };
        if existing.is_some() {
            board_columns::Entity::update(model).exec(conn).await.map_err(CoreError::Db)?;
        } else {
            board_columns::Entity::insert(model).exec(conn).await.map_err(CoreError::Db)?;
        }
        Ok(true)
    }

    async fn apply_card_change(
        &self,
        user_id: &str,
        change: &SyncChange,
    ) -> Result<bool, CoreError> {
        let conn = self.conn();
        self.ensure_board_placeholder(user_id, &change.board_id).await?;
        let existing = board_cards::Entity::find_by_id(change.id.clone())
            .one(conn)
            .await
            .map_err(CoreError::Db)?;
        if let Some(ref row) = existing {
            if row.board_id == change.board_id && change.updated_at <= row.updated_at {
                return Ok(false);
            }
        }
        let data: CardData = serde_json::from_str(&change.data_json)?;
        let model = board_cards::ActiveModel {
            id: Set(change.id.clone()),
            board_id: Set(change.board_id.clone()),
            column_id: Set(change.column_id.clone()),
            item_key: Set(data.item_key),
            position: Set(data.position),
            created_at: Set(data.created_at),
            updated_at: Set(change.updated_at.clone()),
            dirty: Set(0),
            deleted: Set(i64::from(change.deleted)),
        };
        if existing.is_some() {
            board_cards::Entity::update(model).exec(conn).await.map_err(CoreError::Db)?;
        } else {
            board_cards::Entity::insert(model).exec(conn).await.map_err(CoreError::Db)?;
        }
        Ok(true)
    }

    /// Clear the `dirty` flag on the given `(kind, id)` rows (typically all the
    /// changes a client just pushed successfully). Unknown ids are ignored.
    pub async fn mark_synced_changes(
        &self,
        user_id: &str,
        ids: &[(String, String)],
    ) -> Result<(), CoreError> {
        let conn = self.conn();
        for (kind, id) in ids {
            match kind.as_str() {
                KIND_TASK => {
                    tasks::Entity::update_many()
                        .set(tasks::ActiveModel { dirty: Set(0), ..Default::default() })
                        .filter(tasks::Column::Id.eq(id.clone()))
                        .filter(tasks::Column::UserId.eq(user_id))
                        .exec(conn)
                        .await
                        .map_err(CoreError::Db)?;
                }
                KIND_BOARD => {
                    boards::Entity::update_many()
                        .set(boards::ActiveModel { dirty: Set(0), ..Default::default() })
                        .filter(boards::Column::Id.eq(id.clone()))
                        .filter(boards::Column::UserId.eq(user_id))
                        .exec(conn)
                        .await
                        .map_err(CoreError::Db)?;
                }
                KIND_COLUMN => {
                    let board_ids = self.user_board_ids(user_id).await?;
                    board_columns::Entity::update_many()
                        .set(board_columns::ActiveModel { dirty: Set(0), ..Default::default() })
                        .filter(board_columns::Column::Id.eq(id.clone()))
                        .filter(board_columns::Column::BoardId.is_in(board_ids))
                        .exec(conn)
                        .await
                        .map_err(CoreError::Db)?;
                }
                KIND_CARD => {
                    let board_ids = self.user_board_ids(user_id).await?;
                    board_cards::Entity::update_many()
                        .set(board_cards::ActiveModel { dirty: Set(0), ..Default::default() })
                        .filter(board_cards::Column::Id.eq(id.clone()))
                        .filter(board_cards::Column::BoardId.is_in(board_ids))
                        .exec(conn)
                        .await
                        .map_err(CoreError::Db)?;
                }
                _ => return Err(CoreError::DataFormat("change.kind")),
            }
        }
        Ok(())
    }

    /// The highest `updated_at` across all of `user_id`'s synced rows (incl.
    /// tombstones), or `None` if the user has no synced rows. This is the sync
    /// cursor the server hands back so the next PULL resumes from here.
    pub async fn max_cursor(&self, user_id: &str) -> Result<Option<String>, CoreError> {
        let changes = self.changes_since(user_id, None).await?;
        Ok(changes.into_iter().map(|c| c.updated_at).max())
    }

    // --- helpers --------------------------------------------------------------

    /// Ids of ALL of `user_id`'s boards (incl. tombstoned) — the scoping set for
    /// that user's columns/cards.
    async fn user_board_ids(&self, user_id: &str) -> Result<Vec<String>, CoreError> {
        use sea_orm::QuerySelect;
        let ids: Vec<String> = boards::Entity::find()
            .select_only()
            .column(boards::Column::Id)
            .filter(boards::Column::UserId.eq(user_id))
            .into_tuple()
            .all(self.conn())
            .await
            .map_err(CoreError::Db)?;
        Ok(ids)
    }

    /// Ensure a board row with `board_id` exists owned by `user_id`. If it is
    /// missing entirely, insert a minimal placeholder (a later board change will
    /// LWW-overwrite it with the real name/position). If it exists but belongs to
    /// another user, that is a scoping error.
    async fn ensure_board_placeholder(
        &self,
        user_id: &str,
        board_id: &str,
    ) -> Result<(), CoreError> {
        if board_id.is_empty() {
            return Err(CoreError::DataFormat("board_id"));
        }
        let conn = self.conn();
        let existing = boards::Entity::find_by_id(board_id.to_string())
            .one(conn)
            .await
            .map_err(CoreError::Db)?;
        match existing {
            Some(row) if row.user_id == user_id => Ok(()),
            Some(_) => Err(CoreError::DataFormat("board_id")),
            None => {
                // Minimal placeholder with a low `updated_at` so the real board
                // change always wins LWW.
                let model = boards::ActiveModel {
                    id: Set(board_id.to_string()),
                    name: Set(String::new()),
                    position: Set(0),
                    created_at: Set(String::new()),
                    updated_at: Set(String::new()),
                    user_id: Set(user_id.to_string()),
                    dirty: Set(0),
                    deleted: Set(0),
                };
                boards::Entity::insert(model)
                    .exec(conn)
                    .await
                    .map_err(CoreError::Db)?;
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::TaskFilter;
    use time::macros::datetime;

    const U: &str = "alice";

    fn board_change(id: &str, name: &str, updated: &str, deleted: bool) -> SyncChange {
        let data = serde_json::to_string(&BoardData {
            name: name.into(),
            position: 0,
            created_at: updated.into(),
        })
        .unwrap();
        SyncChange {
            kind: KIND_BOARD.into(),
            id: id.into(),
            updated_at: updated.into(),
            deleted,
            data_json: data,
            board_id: String::new(),
            column_id: String::new(),
        }
    }

    async fn mem() -> Store {
        Store::connect("sqlite::memory:").await.unwrap()
    }

    #[tokio::test]
    async fn apply_creates_then_older_is_ignored_and_newer_applied() {
        let store = mem().await;
        let id = uuid::Uuid::new_v4().to_string();

        // First apply creates the row.
        let c1 = board_change(&id, "v1", "2026-06-16T10:00:00Z", false);
        assert!(store.apply_change(U, &c1).await.unwrap(), "create applies");

        // An OLDER change is ignored.
        let older = board_change(&id, "old", "2026-06-16T09:00:00Z", false);
        assert!(!store.apply_change(U, &older).await.unwrap(), "older ignored");
        assert_eq!(store.list_boards_for(U).await.unwrap()[0].name, "v1");

        // An EQUAL change is ignored (LWW is strict >).
        let equal = board_change(&id, "eq", "2026-06-16T10:00:00Z", false);
        assert!(!store.apply_change(U, &equal).await.unwrap(), "equal ignored");

        // A NEWER change applies.
        let newer = board_change(&id, "v2", "2026-06-16T11:00:00Z", false);
        assert!(store.apply_change(U, &newer).await.unwrap(), "newer applies");
        assert_eq!(store.list_boards_for(U).await.unwrap()[0].name, "v2");
    }

    #[tokio::test]
    async fn tombstone_applies_and_hides_from_lists() {
        let store = mem().await;
        let id = uuid::Uuid::new_v4().to_string();
        store
            .apply_change(U, &board_change(&id, "b", "2026-06-16T10:00:00Z", false))
            .await
            .unwrap();
        assert_eq!(store.list_boards_for(U).await.unwrap().len(), 1);

        // Tombstone change wins and hides it.
        let tomb = board_change(&id, "b", "2026-06-16T12:00:00Z", true);
        assert!(store.apply_change(U, &tomb).await.unwrap());
        assert!(store.list_boards_for(U).await.unwrap().is_empty(), "tombstone hides");
    }

    #[tokio::test]
    async fn dirty_changes_includes_dirty_tombstones() {
        let store = mem().await;
        let now = datetime!(2026-06-16 10:00:00 UTC);
        let board = store.create_board_for(U, "b", now).await.unwrap();
        // create_board sets dirty=1.
        let dirty = store.dirty_changes(U).await.unwrap();
        assert_eq!(dirty.len(), 1);
        assert_eq!(dirty[0].kind, KIND_BOARD);
        assert!(!dirty[0].deleted);

        // Soft-delete → still dirty, now tombstoned.
        store
            .delete_board_for(U, board.id, datetime!(2026-06-16 11:00:00 UTC))
            .await
            .unwrap();
        let dirty = store.dirty_changes(U).await.unwrap();
        assert_eq!(dirty.len(), 1);
        assert!(dirty[0].deleted, "tombstone included in dirty set");
    }

    #[tokio::test]
    async fn changes_since_filters_and_orders() {
        let store = mem().await;
        store
            .create_board_for(U, "a", datetime!(2026-06-16 10:00:00 UTC))
            .await
            .unwrap();
        store
            .create_board_for(U, "b", datetime!(2026-06-16 12:00:00 UTC))
            .await
            .unwrap();

        // All changes, ordered ascending.
        let all = store.changes_since(U, None).await.unwrap();
        assert_eq!(all.len(), 2);
        assert!(all[0].updated_at <= all[1].updated_at, "ascending order");

        // Cursor at the first board's timestamp returns only the later one.
        let since = store
            .changes_since(U, Some("2026-06-16T11:00:00Z"))
            .await
            .unwrap();
        assert_eq!(since.len(), 1);
        assert_eq!(since[0].updated_at, "2026-06-16T12:00:00Z");
    }

    #[tokio::test]
    async fn mark_synced_clears_dirty() {
        let store = mem().await;
        let board = store
            .create_board_for(U, "b", datetime!(2026-06-16 10:00:00 UTC))
            .await
            .unwrap();
        assert_eq!(store.dirty_changes(U).await.unwrap().len(), 1);

        store
            .mark_synced_changes(U, &[(KIND_BOARD.to_string(), board.id.to_string())])
            .await
            .unwrap();
        assert!(store.dirty_changes(U).await.unwrap().is_empty(), "dirty cleared");
    }

    #[tokio::test]
    async fn column_and_card_apply_under_placeholder_board() {
        let store = mem().await;
        let board_id = uuid::Uuid::new_v4().to_string();
        let col_id = uuid::Uuid::new_v4().to_string();
        let card_id = uuid::Uuid::new_v4().to_string();

        // Column arrives BEFORE its board → placeholder board is created.
        let col = SyncChange {
            kind: KIND_COLUMN.into(),
            id: col_id.clone(),
            updated_at: "2026-06-16T10:00:00Z".into(),
            deleted: false,
            data_json: serde_json::to_string(&ColumnData {
                name: "todo".into(),
                position: 0,
                filter: "{}".into(),
                created_at: "2026-06-16T10:00:00Z".into(),
            })
            .unwrap(),
            board_id: board_id.clone(),
            column_id: String::new(),
        };
        assert!(store.apply_change(U, &col).await.unwrap());

        let card = SyncChange {
            kind: KIND_CARD.into(),
            id: card_id.clone(),
            updated_at: "2026-06-16T10:30:00Z".into(),
            deleted: false,
            data_json: serde_json::to_string(&CardData {
                item_key: "issue:1".into(),
                position: 0,
                created_at: "2026-06-16T10:30:00Z".into(),
            })
            .unwrap(),
            board_id: board_id.clone(),
            column_id: col_id.clone(),
        };
        assert!(store.apply_change(U, &card).await.unwrap());

        let bid = uuid::Uuid::parse_str(&board_id).unwrap();
        let cols = store.list_columns(bid).await.unwrap();
        assert_eq!(cols.len(), 1);
        assert_eq!(cols[0].name, "todo");
        let cards = store.list_cards(bid).await.unwrap();
        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].item_key, "issue:1");

        // changes_since for this user now sees board(placeholder)+column+card.
        let kinds: Vec<&str> = store
            .changes_since(U, None)
            .await
            .unwrap()
            .iter()
            .map(|c| {
                if c.kind == KIND_BOARD {
                    "board"
                } else if c.kind == KIND_COLUMN {
                    "column"
                } else {
                    "card"
                }
            })
            .collect();
        assert!(kinds.contains(&"column") && kinds.contains(&"card"));
    }

    #[tokio::test]
    async fn task_change_roundtrips_via_data_json() {
        let store = mem().await;
        let draft = crate::domain::TaskDraft::new("hello");
        let task = store
            .create_task_for(U, draft, datetime!(2026-06-16 10:00:00 UTC))
            .await
            .unwrap();
        let changes = store.changes_since(U, None).await.unwrap();
        let tc = changes.iter().find(|c| c.kind == KIND_TASK).unwrap().clone();
        assert_eq!(tc.id, task.id.to_string());

        // Apply that same change into a fresh store → task reconstructed.
        let store2 = mem().await;
        assert!(store2.apply_change(U, &tc).await.unwrap());
        let listed = store2.list_tasks_for(U, TaskFilter::default()).await.unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].title, "hello");
    }
}

