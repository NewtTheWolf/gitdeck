use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Board {
    pub id: Uuid,
    pub name: String,
    pub position: i64,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BoardColumn {
    pub id: Uuid,
    pub board_id: Uuid,
    pub name: String,
    pub position: i64,
    /// Opaque smart-filter JSON; interpreted by the frontend, not by Rust.
    /// `{}` = manual-only column.
    pub filter: serde_json::Value,
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BoardCard {
    pub id: Uuid,
    pub board_id: Uuid,
    pub column_id: Uuid,
    pub item_key: String,
    pub position: i64,
    pub created_at: OffsetDateTime,
}
