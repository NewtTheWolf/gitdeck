//! SeaORM entity for the `board_cards` table.

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "board_cards")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub board_id: String,
    pub column_id: String,
    pub item_key: String,
    pub position: i64,
    pub created_at: String,
    /// RFC3339; bumped on every create/update/move.
    pub updated_at: String,
    /// Set on local mutations so a later sync can push them.
    pub dirty: i64,
    /// Tombstone: `1` means soft-deleted (hidden from lists/gets).
    pub deleted: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
