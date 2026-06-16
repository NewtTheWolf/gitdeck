//! JWT session tokens for server multi-user auth (Phase K1).
//!
//! A [`JwtCodec`] holds the HS256 signing secret (`GITDECK_JWT_SECRET`). It
//! issues tokens whose subject (`sub`) is the user id and which expire ~30 days
//! after issuance, and verifies tokens back to a user id.

use std::sync::Arc;

use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use time::{Duration, OffsetDateTime};

/// 30-day session lifetime.
const SESSION_TTL_DAYS: i64 = 30;

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    /// subject = user id
    sub: String,
    /// expiry, seconds since epoch
    exp: i64,
    /// issued-at, seconds since epoch
    iat: i64,
}

/// Issues and verifies signed session tokens (HS256).
#[derive(Clone)]
pub struct JwtCodec {
    encoding: Arc<EncodingKey>,
    decoding: Arc<DecodingKey>,
}

impl JwtCodec {
    /// Build from a shared secret.
    pub fn new(secret: &str) -> Self {
        Self {
            encoding: Arc::new(EncodingKey::from_secret(secret.as_bytes())),
            decoding: Arc::new(DecodingKey::from_secret(secret.as_bytes())),
        }
    }

    /// Issue a signed token for `user_id`, expiring in ~30 days.
    pub fn issue(&self, user_id: &str) -> Result<String, jsonwebtoken::errors::Error> {
        let now = OffsetDateTime::now_utc();
        let exp = now + Duration::days(SESSION_TTL_DAYS);
        let claims = Claims {
            sub: user_id.to_string(),
            iat: now.unix_timestamp(),
            exp: exp.unix_timestamp(),
        };
        encode(&Header::default(), &claims, &self.encoding)
    }

    /// Verify a token and return the user id (`sub`). Returns `None` on any
    /// failure (bad signature, expired, malformed).
    pub fn verify(&self, token: &str) -> Option<String> {
        let data = decode::<Claims>(token, &self.decoding, &Validation::default()).ok()?;
        Some(data.claims.sub)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn issue_then_verify_roundtrips() {
        let codec = JwtCodec::new("test-secret");
        let token = codec.issue("user-123").unwrap();
        assert_eq!(codec.verify(&token).as_deref(), Some("user-123"));
    }

    #[test]
    fn wrong_secret_rejected() {
        let a = JwtCodec::new("secret-a");
        let b = JwtCodec::new("secret-b");
        let token = a.issue("u").unwrap();
        assert!(b.verify(&token).is_none());
    }

    #[test]
    fn garbage_rejected() {
        let codec = JwtCodec::new("s");
        assert!(codec.verify("not.a.jwt").is_none());
    }
}
