//! SeaORM entities mirroring the 0001-0004 SQL schema. Kept DB-agnostic so the
//! same `Store` works over SQLite (desktop/mobile) and Postgres (hosted server).

pub mod accounts;
pub mod board_cards;
pub mod board_columns;
pub mod boards;
pub mod repo_snapshots;
pub mod tasks;
