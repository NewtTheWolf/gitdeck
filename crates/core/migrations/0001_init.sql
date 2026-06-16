CREATE TABLE tasks (
    id                TEXT PRIMARY KEY NOT NULL,
    account_id        TEXT,
    remote_id         TEXT,
    html_url          TEXT,
    title             TEXT NOT NULL,
    body              TEXT NOT NULL DEFAULT '',
    status            TEXT NOT NULL DEFAULT 'open',
    labels            TEXT NOT NULL DEFAULT '[]',
    due_at            TEXT,
    project_id        TEXT,
    remote_updated_at TEXT,
    local_updated_at  TEXT NOT NULL,
    dirty             INTEGER NOT NULL DEFAULT 0,
    deleted           INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX idx_tasks_status ON tasks(status);
CREATE UNIQUE INDEX idx_tasks_source ON tasks(account_id, remote_id)
    WHERE account_id IS NOT NULL AND remote_id IS NOT NULL;
