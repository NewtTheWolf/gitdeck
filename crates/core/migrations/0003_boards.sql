CREATE TABLE boards (
    id         TEXT PRIMARY KEY NOT NULL,
    name       TEXT NOT NULL,
    position   INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE board_columns (
    id         TEXT PRIMARY KEY NOT NULL,
    board_id   TEXT NOT NULL REFERENCES boards(id) ON DELETE CASCADE,
    name       TEXT NOT NULL,
    position   INTEGER NOT NULL DEFAULT 0,
    filter     TEXT NOT NULL DEFAULT '{}',   -- opaque smart-filter JSON; {} = manual-only
    created_at TEXT NOT NULL
);

CREATE INDEX idx_board_columns_board ON board_columns(board_id);

CREATE TABLE board_cards (             -- manual placements / overrides (secondary)
    id         TEXT PRIMARY KEY NOT NULL,
    board_id   TEXT NOT NULL REFERENCES boards(id) ON DELETE CASCADE,
    column_id  TEXT NOT NULL REFERENCES board_columns(id) ON DELETE CASCADE,
    item_key   TEXT NOT NULL,            -- "todo:<id>" | "gh-issue:<acct>:<owner/repo>#<n>" | "gh-pr:..."
    position   INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL
);

CREATE INDEX idx_board_cards_board ON board_cards(board_id);
CREATE UNIQUE INDEX idx_board_cards_board_item ON board_cards(board_id, item_key);
