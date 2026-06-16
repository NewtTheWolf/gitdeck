use time::OffsetDateTime;
use uuid::Uuid;

use crate::provider::{Provider, RemotePatch};
use crate::store::Store;
use crate::CoreError;

#[derive(Debug, Default, PartialEq, serde::Serialize)]
pub struct SyncReport {
    pub pulled: usize,
    pub pushed: usize,
}

/// Pull remote tasks into the store, then push local dirty edits back to the provider.
pub async fn sync_account(
    store: &Store,
    account_id: Uuid,
    provider: &dyn Provider,
    now: OffsetDateTime,
) -> Result<SyncReport, CoreError> {
    let mut report = SyncReport::default();

    // Pull
    let remotes = provider
        .list_tasks()
        .await
        .map_err(|e| CoreError::Provider(e.0))?;
    for r in remotes {
        store.upsert_remote_task(account_id, r, now).await?;
        report.pulled += 1;
    }

    // Push local dirty edits that map to a remote task on this account.
    for task in store.list_dirty(account_id).await? {
        let Some(src) = task.source.as_ref() else {
            continue;
        };
        let patch = RemotePatch {
            title: Some(task.title.clone()),
            body: Some(task.body.clone()),
            status: Some(task.status),
            labels: Some(task.labels.clone()),
        };
        let updated = provider
            .update_task(&src.remote_id, patch)
            .await
            .map_err(|e| CoreError::Provider(e.0))?;
        store
            .mark_synced(task.id, updated.remote_updated_at, now)
            .await?;
        report.pushed += 1;
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{TaskPatch, TaskStatus};
    use crate::provider::{ProviderError, RemoteDraft, RemoteTask};
    use async_trait::async_trait;
    use std::sync::Mutex;
    use time::macros::datetime;

    struct FakeProvider {
        list: Vec<RemoteTask>,
        updated: Mutex<Vec<(String, String)>>, // (remote_id, new title)
    }
    #[async_trait]
    impl Provider for FakeProvider {
        async fn list_tasks(&self) -> Result<Vec<RemoteTask>, ProviderError> {
            Ok(self.list.clone())
        }
        async fn create_task(&self, _d: RemoteDraft) -> Result<RemoteTask, ProviderError> {
            unimplemented!()
        }
        async fn update_task(
            &self,
            remote_id: &str,
            patch: RemotePatch,
        ) -> Result<RemoteTask, ProviderError> {
            self.updated
                .lock()
                .unwrap()
                .push((remote_id.into(), patch.title.clone().unwrap_or_default()));
            Ok(RemoteTask {
                remote_id: remote_id.into(),
                title: patch.title.unwrap_or_default(),
                body: String::new(),
                status: TaskStatus::Open,
                labels: vec![],
                html_url: None,
                remote_updated_at: datetime!(2026-06-15 12:00:00 UTC),
            })
        }
    }

    #[tokio::test]
    async fn pull_then_push_roundtrip() {
        let store = Store::connect("sqlite::memory:").await.unwrap();
        let acct = Uuid::new_v4();
        let now = datetime!(2026-06-15 10:00:00 UTC);
        let provider = FakeProvider {
            list: vec![RemoteTask {
                remote_id: "1".into(),
                title: "remote".into(),
                body: "".into(),
                status: TaskStatus::Open,
                labels: vec![],
                html_url: None,
                remote_updated_at: now,
            }],
            updated: Mutex::new(vec![]),
        };

        let r1 = sync_account(&store, acct, &provider, now).await.unwrap();
        assert_eq!(r1.pulled, 1);
        assert_eq!(r1.pushed, 0);

        // user edits the pulled task → dirty
        let pulled = store
            .list_tasks(crate::domain::TaskFilter::default())
            .await
            .unwrap();
        store
            .update_task(
                pulled[0].id,
                TaskPatch {
                    title: Some("edited".into()),
                    ..Default::default()
                },
                now,
            )
            .await
            .unwrap();

        let r2 = sync_account(&store, acct, &provider, now).await.unwrap();
        assert_eq!(r2.pushed, 1, "dirty edit pushed");
        assert_eq!(
            provider.updated.lock().unwrap()[0],
            ("1".to_string(), "edited".to_string())
        );
        // after push, no longer dirty
        assert!(store.list_dirty(acct).await.unwrap().is_empty());
    }
}
