//! Headless token source for the standalone server.

use async_trait::async_trait;
use newt_todo_auth::{OAuthToken, TokenError, TokenStore};

/// A headless [`TokenStore`] that returns a single configured token for ANY
/// account id.
///
/// LIMITATION (v1, "single token for now"): the standalone server hosts one
/// GitHub token (from `GITHUB_TOKEN`), used for every account. Per-account
/// server-side tokens are a later phase. `save`/`delete` are no-ops; `load`
/// returns the configured token (or `None` when unset, so token-less RPCs still
/// surface a clear "no token stored" error from the service layer).
pub struct EnvTokenStore {
    token: Option<String>,
}

impl EnvTokenStore {
    /// Build from an explicit token (e.g. read from `GITHUB_TOKEN`).
    pub fn new(token: Option<String>) -> Self {
        Self { token }
    }

    /// Build by reading `GITHUB_TOKEN` from the environment.
    pub fn from_env() -> Self {
        Self::new(std::env::var("GITHUB_TOKEN").ok().filter(|s| !s.is_empty()))
    }

    pub fn has_token(&self) -> bool {
        self.token.is_some()
    }
}

#[async_trait]
impl TokenStore for EnvTokenStore {
    async fn save(&self, _account_id: &str, _token: &OAuthToken) -> Result<(), TokenError> {
        // No-op: the server's token is configured out of band (env), not persisted here.
        Ok(())
    }

    async fn load(&self, _account_id: &str) -> Result<Option<OAuthToken>, TokenError> {
        Ok(self.token.clone().map(|access_token| OAuthToken {
            access_token,
            refresh_token: None,
        }))
    }

    async fn delete(&self, _account_id: &str) -> Result<(), TokenError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn returns_configured_token_for_any_account() {
        let store = EnvTokenStore::new(Some("gho_x".into()));
        let t = store.load("any-account").await.unwrap().unwrap();
        assert_eq!(t.access_token, "gho_x");
    }

    #[tokio::test]
    async fn none_when_unset() {
        let store = EnvTokenStore::new(None);
        assert!(store.load("any").await.unwrap().is_none());
    }
}
