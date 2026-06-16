//! SeaORM entity for the `boards` table.

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "boards")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub name: String,
    pub position: i64,
    pub created_at: String,
    pub updated_at: String,
    /// Owning user; `'local'` for the offline desktop's single user.
    pub user_id: String,
    /// Set on local mutations so a later sync can push them.
    pub dirty: i64,
    /// Tombstone: `1` means soft-deleted (hidden from lists/gets).
    pub deleted: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
