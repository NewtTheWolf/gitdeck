CREATE TABLE accounts (
    id           TEXT PRIMARY KEY NOT NULL,
    provider     TEXT NOT NULL,           -- 'github' | 'codeberg' | 'clickup'
    display_name TEXT NOT NULL,
    base_url     TEXT,                     -- API base (null = provider default)
    config       TEXT NOT NULL DEFAULT '{}', -- provider-specific JSON, e.g. {"owner":"o","repo":"r"}
    created_at   TEXT NOT NULL
);
