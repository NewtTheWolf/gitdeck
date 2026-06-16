use std::collections::HashMap;
use std::sync::Mutex;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OAuthToken {
    pub access_token: String,
    pub refresh_token: Option<String>,
}

#[derive(Debug, thiserror::Error)]
#[error("token store error: {0}")]
pub struct TokenError(pub String);

#[async_trait]
pub trait TokenStore: Send + Sync {
    async fn save(&self, account_id: &str, token: &OAuthToken) -> Result<(), TokenError>;
    async fn load(&self, account_id: &str) -> Result<Option<OAuthToken>, TokenError>;
    async fn delete(&self, account_id: &str) -> Result<(), TokenError>;
}

/// In-memory token store for tests.
#[derive(Default)]
pub struct MemoryTokenStore {
    inner: Mutex<HashMap<String, OAuthToken>>,
}

impl MemoryTokenStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl TokenStore for MemoryTokenStore {
    async fn save(&self, account_id: &str, token: &OAuthToken) -> Result<(), TokenError> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|e| TokenError(format!("lock poisoned: {e}")))?;
        guard.insert(account_id.to_string(), token.clone());
        Ok(())
    }

    async fn load(&self, account_id: &str) -> Result<Option<OAuthToken>, TokenError> {
        let guard = self
            .inner
            .lock()
            .map_err(|e| TokenError(format!("lock poisoned: {e}")))?;
        Ok(guard.get(account_id).cloned())
    }

    async fn delete(&self, account_id: &str) -> Result<(), TokenError> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|e| TokenError(format!("lock poisoned: {e}")))?;
        guard.remove(account_id);
        Ok(())
    }
}

/// Real token store backed by the OS secret service via the `keyring` crate.
///
/// Tokens are serialized as JSON and stored under the service name `newt-todo`,
/// keyed by `account_id`. Not unit-tested (needs an OS secret service).
pub struct KeyringTokenStore;

impl KeyringTokenStore {
    pub fn new() -> Self {
        Self
    }

    fn entry(account_id: &str) -> Result<keyring::Entry, TokenError> {
        keyring::Entry::new("newt-todo", account_id).map_err(|e| TokenError(e.to_string()))
    }
}

impl Default for KeyringTokenStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TokenStore for KeyringTokenStore {
    async fn save(&self, account_id: &str, token: &OAuthToken) -> Result<(), TokenError> {
        let json = serde_json::to_string(token).map_err(|e| TokenError(e.to_string()))?;
        let entry = Self::entry(account_id)?;
        entry
            .set_password(&json)
            .map_err(|e| TokenError(e.to_string()))
    }

    async fn load(&self, account_id: &str) -> Result<Option<OAuthToken>, TokenError> {
        let entry = Self::entry(account_id)?;
        match entry.get_password() {
            Ok(json) => {
                let token = serde_json::from_str(&json).map_err(|e| TokenError(e.to_string()))?;
                Ok(Some(token))
            }
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(TokenError(e.to_string())),
        }
    }

    async fn delete(&self, account_id: &str) -> Result<(), TokenError> {
        let entry = Self::entry(account_id)?;
        match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(TokenError(e.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn token() -> OAuthToken {
        OAuthToken {
            access_token: "gho_abc".into(),
            refresh_token: Some("ghr_xyz".into()),
        }
    }

    #[tokio::test]
    async fn memory_save_load_roundtrip() {
        let store = MemoryTokenStore::new();
        store.save("acct-1", &token()).await.unwrap();
        let loaded = store.load("acct-1").await.unwrap();
        assert_eq!(loaded, Some(token()));
    }

    #[tokio::test]
    async fn memory_load_missing_returns_none() {
        let store = MemoryTokenStore::new();
        assert_eq!(store.load("nope").await.unwrap(), None);
    }

    #[tokio::test]
    async fn memory_delete_removes() {
        let store = MemoryTokenStore::new();
        store.save("acct-1", &token()).await.unwrap();
        store.delete("acct-1").await.unwrap();
        assert_eq!(store.load("acct-1").await.unwrap(), None);
    }
}
