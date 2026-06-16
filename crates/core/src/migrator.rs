//! DB-agnostic schema migration recreating the original 0001-0004 SQL schema via
//! SeaORM's schema builder. Runs on `Store::connect` for both SQLite and Postgres.
//!
//! Idempotency / compatibility: every table is created `IF NOT EXISTS`, so an
//! existing SQLite database created by the old `sqlx::migrate!` schema is left
//! untouched (its tables already exist with identical column names/types). Fresh
//! databases (SQLite in-memory or Postgres) get the full schema built here.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

/// Column-name identifiers. Using raw `Alias` keeps names byte-identical to the
/// hand-written SQL so old SQLite DBs and new ones share one schema.
fn col(name: &str) -> Alias {
    Alias::new(name)
}

/// True if `table` already has a column named `column`, probed against the live
/// schema. Works on SQLite (`PRAGMA table_info`) and Postgres
/// (`information_schema.columns`).
async fn column_exists(
    manager: &SchemaManager<'_>,
    table: &str,
    column: &str,
) -> Result<bool, DbErr> {
    use sea_orm::{ConnectionTrait, DatabaseBackend, Statement, Value};
    let conn = manager.get_connection();
    let backend = manager.get_database_backend();
    let stmt = match backend {
        DatabaseBackend::Postgres => Statement::from_sql_and_values(
            backend,
            "SELECT 1 FROM information_schema.columns WHERE table_name = $1 AND column_name = $2",
            [Value::from(table), Value::from(column)],
        ),
        // SQLite: PRAGMA can't be parameterized; the names are internal constants.
        _ => Statement::from_string(
            backend,
            format!("SELECT 1 FROM pragma_table_info('{table}') WHERE name = '{column}'"),
        ),
    };
    Ok(conn.query_one(stmt).await?.is_some())
}

/// Add `column` to `table` only if it isn't already present (SQLite lacks
/// `ADD COLUMN IF NOT EXISTS`, so this is the portable equivalent).
async fn add_column_if_missing(
    manager: &SchemaManager<'_>,
    table: &str,
    column: &str,
    def: &mut ColumnDef,
) -> Result<(), DbErr> {
    if column_exists(manager, table, column).await? {
        return Ok(());
    }
    manager
        .alter_table(
            Table::alter()
                .table(Alias::new(table))
                .add_column(def)
                .to_owned(),
        )
        .await
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let backend = manager.get_database_backend();

        // --- tasks (0001) -----------------------------------------------------
        manager
            .create_table(
                Table::create()
                    .table(Alias::new("tasks"))
                    .if_not_exists()
                    .col(ColumnDef::new(col("id")).text().not_null().primary_key())
                    .col(ColumnDef::new(col("account_id")).text())
                    .col(ColumnDef::new(col("remote_id")).text())
                    .col(ColumnDef::new(col("html_url")).text())
                    .col(ColumnDef::new(col("title")).text().not_null())
                    .col(
                        ColumnDef::new(col("body"))
                            .text()
                            .not_null()
                            .default(""),
                    )
                    .col(
                        ColumnDef::new(col("status"))
                            .text()
                            .not_null()
                            .default("open"),
                    )
                    .col(
                        ColumnDef::new(col("labels"))
                            .text()
                            .not_null()
                            .default("[]"),
                    )
                    .col(ColumnDef::new(col("due_at")).text())
                    .col(ColumnDef::new(col("project_id")).text())
                    .col(ColumnDef::new(col("remote_updated_at")).text())
                    .col(ColumnDef::new(col("local_updated_at")).text().not_null())
                    .col(
                        ColumnDef::new(col("dirty"))
                            .big_integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(col("deleted"))
                            .big_integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(col("user_id"))
                            .text()
                            .not_null()
                            .default("local"),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_tasks_status")
                    .table(Alias::new("tasks"))
                    .col(col("status"))
                    .to_owned(),
            )
            .await?;

        // Partial unique index — both SQLite and Postgres support `WHERE` on an
        // index. SeaORM's index builder has no portable partial-index API, so
        // emit raw SQL guarded by backend (the predicate syntax is identical on
        // both engines here).
        manager
            .get_connection()
            .execute_unprepared(
                "CREATE UNIQUE INDEX IF NOT EXISTS idx_tasks_source ON tasks (account_id, remote_id) \
                 WHERE account_id IS NOT NULL AND remote_id IS NOT NULL",
            )
            .await?;

        // --- accounts (0002) --------------------------------------------------
        manager
            .create_table(
                Table::create()
                    .table(Alias::new("accounts"))
                    .if_not_exists()
                    .col(ColumnDef::new(col("id")).text().not_null().primary_key())
                    .col(ColumnDef::new(col("provider")).text().not_null())
                    .col(ColumnDef::new(col("display_name")).text().not_null())
                    .col(ColumnDef::new(col("base_url")).text())
                    .col(
                        ColumnDef::new(col("config"))
                            .text()
                            .not_null()
                            .default("{}"),
                    )
                    .col(ColumnDef::new(col("created_at")).text().not_null())
                    .to_owned(),
            )
            .await?;

        // --- boards (0003) ----------------------------------------------------
        manager
            .create_table(
                Table::create()
                    .table(Alias::new("boards"))
                    .if_not_exists()
                    .col(ColumnDef::new(col("id")).text().not_null().primary_key())
                    .col(ColumnDef::new(col("name")).text().not_null())
                    .col(
                        ColumnDef::new(col("position"))
                            .big_integer()
                            .not_null()
                            .default(0),
                    )
                    .col(ColumnDef::new(col("created_at")).text().not_null())
                    .col(ColumnDef::new(col("updated_at")).text().not_null())
                    .col(
                        ColumnDef::new(col("user_id"))
                            .text()
                            .not_null()
                            .default("local"),
                    )
                    .col(
                        ColumnDef::new(col("dirty"))
                            .big_integer()
                            .not_null()
                            .default(1),
                    )
                    .col(
                        ColumnDef::new(col("deleted"))
                            .big_integer()
                            .not_null()
                            .default(0),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Alias::new("board_columns"))
                    .if_not_exists()
                    .col(ColumnDef::new(col("id")).text().not_null().primary_key())
                    .col(ColumnDef::new(col("board_id")).text().not_null())
                    .col(ColumnDef::new(col("name")).text().not_null())
                    .col(
                        ColumnDef::new(col("position"))
                            .big_integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(col("filter"))
                            .text()
                            .not_null()
                            .default("{}"),
                    )
                    .col(ColumnDef::new(col("created_at")).text().not_null())
                    .col(
                        ColumnDef::new(col("updated_at"))
                            .text()
                            .not_null()
                            .default(""),
                    )
                    .col(
                        ColumnDef::new(col("dirty"))
                            .big_integer()
                            .not_null()
                            .default(1),
                    )
                    .col(
                        ColumnDef::new(col("deleted"))
                            .big_integer()
                            .not_null()
                            .default(0),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_board_columns_board")
                            .from(Alias::new("board_columns"), col("board_id"))
                            .to(Alias::new("boards"), col("id"))
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_board_columns_board")
                    .table(Alias::new("board_columns"))
                    .col(col("board_id"))
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Alias::new("board_cards"))
                    .if_not_exists()
                    .col(ColumnDef::new(col("id")).text().not_null().primary_key())
                    .col(ColumnDef::new(col("board_id")).text().not_null())
                    .col(ColumnDef::new(col("column_id")).text().not_null())
                    .col(ColumnDef::new(col("item_key")).text().not_null())
                    .col(
                        ColumnDef::new(col("position"))
                            .big_integer()
                            .not_null()
                            .default(0),
                    )
                    .col(ColumnDef::new(col("created_at")).text().not_null())
                    .col(
                        ColumnDef::new(col("updated_at"))
                            .text()
                            .not_null()
                            .default(""),
                    )
                    .col(
                        ColumnDef::new(col("dirty"))
                            .big_integer()
                            .not_null()
                            .default(1),
                    )
                    .col(
                        ColumnDef::new(col("deleted"))
                            .big_integer()
                            .not_null()
                            .default(0),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_board_cards_board")
                            .from(Alias::new("board_cards"), col("board_id"))
                            .to(Alias::new("boards"), col("id"))
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_board_cards_column")
                            .from(Alias::new("board_cards"), col("column_id"))
                            .to(Alias::new("board_columns"), col("id"))
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_board_cards_board")
                    .table(Alias::new("board_cards"))
                    .col(col("board_id"))
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .unique()
                    .name("idx_board_cards_board_item")
                    .table(Alias::new("board_cards"))
                    .col(col("board_id"))
                    .col(col("item_key"))
                    .to_owned(),
            )
            .await?;

        // --- repo_snapshots (0004) -------------------------------------------
        manager
            .create_table(
                Table::create()
                    .table(Alias::new("repo_snapshots"))
                    .if_not_exists()
                    .col(ColumnDef::new(col("id")).text().not_null().primary_key())
                    .col(ColumnDef::new(col("account_id")).text().not_null())
                    .col(ColumnDef::new(col("repo_full_name")).text().not_null())
                    .col(ColumnDef::new(col("day")).text().not_null())
                    .col(
                        ColumnDef::new(col("stars"))
                            .big_integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(col("forks"))
                            .big_integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(col("open_issues"))
                            .big_integer()
                            .not_null()
                            .default(0),
                    )
                    .col(ColumnDef::new(col("captured_at")).text().not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .unique()
                    .name("idx_repo_snapshots_unique")
                    .table(Alias::new("repo_snapshots"))
                    .col(col("account_id"))
                    .col(col("repo_full_name"))
                    .col(col("day"))
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_repo_snapshots_acct_day")
                    .table(Alias::new("repo_snapshots"))
                    .col(col("account_id"))
                    .col(col("day"))
                    .to_owned(),
            )
            .await?;

        // --- sync metadata back-fill (K2) ------------------------------------
        // Existing SQLite DBs created by the old `sqlx::migrate!` schema (or an
        // earlier run of this migrator) already have these tables WITHOUT the new
        // sync columns. The `create_table ... if_not_exists` above is a no-op for
        // them, so add the columns here. SQLite's `ALTER TABLE ADD COLUMN` has no
        // `IF NOT EXISTS`, so we probe the live column set first; the same probe
        // path works for Postgres (`add_column_if_not_exists` would also work on
        // PG, but the probe keeps a single code path).
        add_column_if_missing(
            manager,
            "tasks",
            "user_id",
            ColumnDef::new(col("user_id")).text().not_null().default("local"),
        )
        .await?;
        add_column_if_missing(
            manager,
            "boards",
            "user_id",
            ColumnDef::new(col("user_id")).text().not_null().default("local"),
        )
        .await?;
        add_column_if_missing(
            manager,
            "boards",
            "dirty",
            ColumnDef::new(col("dirty")).big_integer().not_null().default(1),
        )
        .await?;
        add_column_if_missing(
            manager,
            "boards",
            "deleted",
            ColumnDef::new(col("deleted")).big_integer().not_null().default(0),
        )
        .await?;
        for table in ["board_columns", "board_cards"] {
            add_column_if_missing(
                manager,
                table,
                "updated_at",
                ColumnDef::new(col("updated_at")).text().not_null().default(""),
            )
            .await?;
            add_column_if_missing(
                manager,
                table,
                "dirty",
                ColumnDef::new(col("dirty")).big_integer().not_null().default(1),
            )
            .await?;
            add_column_if_missing(
                manager,
                table,
                "deleted",
                ColumnDef::new(col("deleted")).big_integer().not_null().default(0),
            )
            .await?;
        }

        let _ = backend;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        // Personal app: no rollback path needed for this consolidated migration.
        Err(DbErr::Custom("down migration not supported".into()))
    }
}

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![Box::new(Migration)]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::{ConnectionTrait, Database, Statement};
    use sea_orm_migration::MigratorTrait;

    /// Simulate a pre-K2 SQLite database (old `boards` table WITHOUT the sync
    /// columns + a row) and confirm the migrator back-fills the new columns and
    /// defaults the existing row to the local user / dirty tombstone state.
    #[tokio::test]
    async fn backfills_sync_columns_on_legacy_db() {
        let conn = Database::connect("sqlite::memory:").await.unwrap();

        // Legacy schema: boards as it existed before this migration ran.
        conn.execute(Statement::from_string(
            conn.get_database_backend(),
            "CREATE TABLE boards (id TEXT PRIMARY KEY, name TEXT NOT NULL, \
             position BIGINT NOT NULL DEFAULT 0, created_at TEXT NOT NULL, \
             updated_at TEXT NOT NULL)"
                .to_owned(),
        ))
        .await
        .unwrap();
        conn.execute(Statement::from_string(
            conn.get_database_backend(),
            "INSERT INTO boards (id, name, position, created_at, updated_at) \
             VALUES ('b1', 'legacy', 0, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')"
                .to_owned(),
        ))
        .await
        .unwrap();

        // Running the migrator must ADD the new columns to the existing table.
        Migrator::up(&conn, None).await.unwrap();

        let row = conn
            .query_one(Statement::from_string(
                conn.get_database_backend(),
                "SELECT user_id, dirty, deleted FROM boards WHERE id = 'b1'".to_owned(),
            ))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(row.try_get::<String>("", "user_id").unwrap(), "local");
        assert_eq!(row.try_get::<i64>("", "dirty").unwrap(), 1);
        assert_eq!(row.try_get::<i64>("", "deleted").unwrap(), 0);

        // Idempotent: a second run does not error (columns already present).
        Migrator::up(&conn, None).await.unwrap();
    }
}
