CREATE TABLE repo_snapshots (
    id             TEXT PRIMARY KEY NOT NULL,
    account_id     TEXT NOT NULL,
    repo_full_name TEXT NOT NULL,
    day            TEXT NOT NULL,         -- 'YYYY-MM-DD'
    stars          INTEGER NOT NULL DEFAULT 0,
    forks          INTEGER NOT NULL DEFAULT 0,
    open_issues    INTEGER NOT NULL DEFAULT 0,
    captured_at    TEXT NOT NULL
);
CREATE UNIQUE INDEX idx_repo_snapshots_unique ON repo_snapshots(account_id, repo_full_name, day);
CREATE INDEX idx_repo_snapshots_acct_day ON repo_snapshots(account_id, day);
