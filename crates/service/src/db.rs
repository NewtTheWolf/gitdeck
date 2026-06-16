use std::path::Path;

/// Build a sqlx SQLite URL for a filesystem path, creating the DB if missing.
pub fn db_url_for_path(path: &Path) -> String {
    format!("sqlite://{}?mode=rwc", path.display())
}

/// Resolve the DB URL: `NEWT_TODO_DB` env var (a filesystem path) wins; otherwise
/// the default OS data dir `<data_dir>/newt-todo/todo.db`. Ensures the parent dir exists.
pub fn resolve_db_url() -> anyhow::Result<String> {
    let path = match std::env::var_os("NEWT_TODO_DB") {
        Some(p) => std::path::PathBuf::from(p),
        None => {
            let dirs = directories::ProjectDirs::from("dev", "NewtTheWolf", "newt-todo")
                .ok_or_else(|| anyhow::anyhow!("cannot determine OS data dir"))?;
            dirs.data_dir().join("todo.db")
        }
    };
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    Ok(db_url_for_path(&path))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn db_url_for_path_uses_rwc_mode() {
        let url = db_url_for_path(&PathBuf::from("/tmp/x/todo.db"));
        assert_eq!(url, "sqlite:///tmp/x/todo.db?mode=rwc");
    }
}
