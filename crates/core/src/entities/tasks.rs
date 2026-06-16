//! SeaORM entity for the `tasks` table. Column names/types mirror the original
//! sqlx schema exactly (ids/timestamps as TEXT/String, labels as a JSON TEXT
//! string, flags as integers) so existing SQLite databases keep working and the
//! schema stays portable to Postgres.

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "tasks")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub account_id: Option<String>,
    pub remote_id: Option<String>,
    pub html_url: Option<String>,
    pub title: String,
    pub body: String,
    pub status: String,
    pub labels: String,
    pub due_at: Option<String>,
    pub project_id: Option<String>,
    pub remote_updated_at: Option<String>,
    pub local_updated_at: String,
    pub dirty: i64,
    pub deleted: i64,
    /// Owning user; `'local'` for the offline desktop's single user.
    pub user_id: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
