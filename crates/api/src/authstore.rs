//! Server-only user/auth store (Phase K1).
//!
//! This is a SERVER concern and deliberately lives in `crates/api`, NOT in
//! `crates/core` (which the offline desktop shares and must stay single-tenant).
//!
//! It opens its OWN SeaORM [`DatabaseConnection`] to the same `GITDECK_DB` the
//! core `Store` uses (both can point at the same SQLite file / Postgres db), and
//! runs a small `sea-orm-migration` migrator that creates the server tables
//! idempotently (`IF NOT EXISTS`), DB-agnostic across SQLite + Postgres:
//!
//! - `users        { id TEXT PK, username TEXT UNIQUE, password_hash TEXT, created_at TEXT }`
//! - `user_tokens  { user_id TEXT PK/FK, github_token TEXT, updated_at TEXT }`
//!
//! Passwords are hashed with argon2. The `user_tokens` table holds the per-user
//! GitHub token that replaces the single-token `EnvTokenStore` in multi-user mode.

use anyhow::{anyhow, Context, Result};
use argon2::password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use sea_orm::{
    ActiveValue::Set, ColumnTrait, ConnectOptions, Database, DatabaseConnection, DbErr,
    EntityTrait, QueryFilter,
};
use sea_orm::sea_query::OnConflict;
use sea_orm_migration::prelude::*;
use sea_orm_migration::MigratorTrait;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use uuid::Uuid;

// --- entities ---------------------------------------------------------------

pub mod users {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
    #[sea_orm(table_name = "users")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: String,
        #[sea_orm(unique)]
        pub username: String,
        pub password_hash: String,
        pub created_at: String,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod user_tokens {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
    #[sea_orm(table_name = "user_tokens")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub user_id: String,
        pub github_token: String,
        pub updated_at: String,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

// --- migrator ---------------------------------------------------------------

#[derive(DeriveMigrationName)]
struct Migration;

fn col(name: &str) -> Alias {
    Alias::new(name)
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Alias::new("users"))
                    .if_not_exists()
                    .col(ColumnDef::new(col("id")).text().not_null().primary_key())
                    .col(ColumnDef::new(col("username")).text().not_null())
                    .col(ColumnDef::new(col("password_hash")).text().not_null())
                    .col(ColumnDef::new(col("created_at")).text().not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .unique()
                    .name("idx_users_username")
                    .table(Alias::new("users"))
                    .col(col("username"))
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Alias::new("user_tokens"))
                    .if_not_exists()
                    .col(
                        ColumnDef::new(col("user_id"))
                            .text()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(col("github_token")).text().not_null())
                    .col(ColumnDef::new(col("updated_at")).text().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_user_tokens_user")
                            .from(Alias::new("user_tokens"), col("user_id"))
                            .to(Alias::new("users"), col("id"))
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Custom("down migration not supported".into()))
    }
}

struct AuthMigrator;

#[async_trait::async_trait]
impl MigratorTrait for AuthMigrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![Box::new(Migration)]
    }
}

// --- store ------------------------------------------------------------------

/// Server-side user/auth store backed by its own SeaORM connection.
pub struct AuthStore {
    conn: DatabaseConnection,
}

impl AuthStore {
    /// Open a connection to `url` (the same DSN as `GITDECK_DB`) and run the
    /// server-table migrator. Accepts `sqlite:...` (incl. `sqlite::memory:`) and
    /// `postgres://...` — SeaORM picks the backend from the scheme.
    pub async fn connect(url: &str) -> Result<Self> {
        let mut options = ConnectOptions::new(url.to_owned());
        // In-memory SQLite gives each connection its own database; pin to one.
        if url.contains(":memory:") {
            options.max_connections(1);
        }
        let conn = Database::connect(options)
            .await
            .context("connect auth store")?;
        AuthMigrator::up(&conn, None)
            .await
            .context("run auth migrator")?;
        Ok(Self { conn })
    }

    /// Register a new user. Returns the new `user_id`. Errors if the username is
    /// already taken.
    pub async fn register(&self, username: &str, password: &str) -> Result<String> {
        let username = username.trim();
        if username.is_empty() {
            return Err(anyhow!("username must not be empty"));
        }
        if password.is_empty() {
            return Err(anyhow!("password must not be empty"));
        }

        let existing = users::Entity::find()
            .filter(users::Column::Username.eq(username))
            .one(&self.conn)
            .await
            .context("lookup username")?;
        if existing.is_some() {
            return Err(anyhow!("username already taken"));
        }

        let password_hash = hash_password(password)?;
        let id = Uuid::new_v4().to_string();
        let created_at = now_rfc3339()?;

        let model = users::ActiveModel {
            id: Set(id.clone()),
            username: Set(username.to_string()),
            password_hash: Set(password_hash),
            created_at: Set(created_at),
        };
        // A unique-index race (two concurrent registers) surfaces as a DB error.
        users::Entity::insert(model)
            .exec(&self.conn)
            .await
            .map_err(|e| anyhow!("username already taken or insert failed: {e}"))?;

        Ok(id)
    }

    /// Verify a username/password. Returns the `user_id` on success, `None` on
    /// unknown user or wrong password.
    pub async fn verify_login(&self, username: &str, password: &str) -> Result<Option<String>> {
        let row = users::Entity::find()
            .filter(users::Column::Username.eq(username.trim()))
            .one(&self.conn)
            .await
            .context("lookup user")?;
        let Some(user) = row else {
            return Ok(None);
        };
        if verify_password(password, &user.password_hash) {
            Ok(Some(user.id))
        } else {
            Ok(None)
        }
    }

    /// Look up a user by id (for GetMe).
    pub async fn get_user(&self, user_id: &str) -> Result<Option<users::Model>> {
        users::Entity::find_by_id(user_id.to_string())
            .one(&self.conn)
            .await
            .context("lookup user by id")
    }

    /// Store (upsert) the GitHub token for a user.
    pub async fn set_github_token(&self, user_id: &str, token: &str) -> Result<()> {
        let updated_at = now_rfc3339()?;
        let model = user_tokens::ActiveModel {
            user_id: Set(user_id.to_string()),
            github_token: Set(token.to_string()),
            updated_at: Set(updated_at),
        };
        user_tokens::Entity::insert(model)
            .on_conflict(
                OnConflict::column(user_tokens::Column::UserId)
                    .update_columns([
                        user_tokens::Column::GithubToken,
                        user_tokens::Column::UpdatedAt,
                    ])
                    .to_owned(),
            )
            .exec(&self.conn)
            .await
            .context("upsert github token")?;
        Ok(())
    }

    /// Retrieve a user's stored GitHub token, if any.
    pub async fn get_github_token(&self, user_id: &str) -> Result<Option<String>> {
        let row = user_tokens::Entity::find_by_id(user_id.to_string())
            .one(&self.conn)
            .await
            .context("lookup github token")?;
        Ok(row.map(|r| r.github_token))
    }
}

// --- helpers ----------------------------------------------------------------

fn now_rfc3339() -> Result<String> {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .context("format timestamp")
}

fn hash_password(password: &str) -> Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| anyhow!("hash password: {e}"))?;
    Ok(hash.to_string())
}

fn verify_password(password: &str, stored_hash: &str) -> bool {
    let Ok(parsed) = PasswordHash::new(stored_hash) else {
        return false;
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn mem_store() -> AuthStore {
        AuthStore::connect("sqlite::memory:").await.unwrap()
    }

    #[tokio::test]
    async fn register_then_verify_login() {
        let store = mem_store().await;
        let id = store.register("alice", "hunter2").await.unwrap();
        let logged = store.verify_login("alice", "hunter2").await.unwrap();
        assert_eq!(logged.as_deref(), Some(id.as_str()));
    }

    #[tokio::test]
    async fn duplicate_username_errors() {
        let store = mem_store().await;
        store.register("bob", "pw").await.unwrap();
        assert!(store.register("bob", "other").await.is_err());
    }

    #[tokio::test]
    async fn wrong_password_no_login() {
        let store = mem_store().await;
        store.register("carol", "correct").await.unwrap();
        assert!(store.verify_login("carol", "wrong").await.unwrap().is_none());
        assert!(store.verify_login("nope", "x").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn github_token_roundtrip() {
        let store = mem_store().await;
        let id = store.register("dave", "pw").await.unwrap();
        assert!(store.get_github_token(&id).await.unwrap().is_none());
        store.set_github_token(&id, "gho_abc").await.unwrap();
        assert_eq!(
            store.get_github_token(&id).await.unwrap().as_deref(),
            Some("gho_abc")
        );
        // Upsert overwrites.
        store.set_github_token(&id, "gho_new").await.unwrap();
        assert_eq!(
            store.get_github_token(&id).await.unwrap().as_deref(),
            Some("gho_new")
        );
    }
}
