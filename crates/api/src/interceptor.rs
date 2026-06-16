//! Auth middleware for the gRPC server (Phase K1).
//!
//! [`AuthLayer`] is a tower layer wrapping the routed tonic service. It runs
//! BEFORE method dispatch, so unlike a tonic `Interceptor` it can see the gRPC
//! method path and therefore exempt the auth RPCs (`Register`, `Login`).
//!
//! Behaviour by [`AuthMode`]:
//! - [`AuthMode::Single`] (DEFAULT — today's behaviour, backwards-compatible):
//!   no enforcement. An optional API key (`GITDECK_API_KEY`) is still checked if
//!   configured. No per-user identity is injected.
//! - [`AuthMode::Multi`]: every request carrying `authorization: Bearer <jwt>`
//!   has the JWT verified; the resulting user id is injected as an [`AuthContext`]
//!   request extension. Requests to data/GitHub RPCs WITHOUT a valid token are
//!   rejected `unauthenticated`. `Register` and `Login` are exempt (so users can
//!   bootstrap a session). The optional API key, if set, is also enforced.
//!
//! Downstream the [`AuthContext`] extension is read by `GitdeckService` to
//! resolve the per-user GitHub token and to authorize `SetGithubToken`/`GetMe`.

use std::sync::Arc;
use std::task::{Context, Poll};

use tonic::body::BoxBody;
use tonic::codegen::http;
use tonic::Status;
use tower::{Layer, Service};

use crate::jwt::JwtCodec;

/// Server auth mode, from `GITDECK_AUTH` (`single` default, or `multi`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuthMode {
    /// Today's behaviour: no per-user auth. Backwards-compatible default.
    Single,
    /// Multi-user: bearer JWT required for data RPCs; user id injected.
    Multi,
}

impl AuthMode {
    /// Parse from the `GITDECK_AUTH` env value. Unknown/empty → `Single`.
    pub fn parse(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "multi" => AuthMode::Multi,
            _ => AuthMode::Single,
        }
    }
}

/// The authenticated identity injected into request extensions by [`AuthLayer`].
/// `user_id` is `Some` only in multi mode with a valid bearer token.
#[derive(Clone, Debug)]
pub struct AuthContext {
    pub mode: AuthMode,
    pub user_id: Option<String>,
}

/// Tower layer applying [`AuthService`] over the routed tonic service.
#[derive(Clone)]
pub struct AuthLayer {
    mode: AuthMode,
    jwt: Option<JwtCodec>,
    api_key: Option<Arc<String>>,
}

impl AuthLayer {
    /// Build the layer.
    /// - `mode`: single (BC) or multi.
    /// - `jwt`: required in multi mode to verify bearer tokens.
    /// - `api_key`: optional bearer API key (`GITDECK_API_KEY`); when set, every
    ///   request (except the auth RPCs) must present it.
    pub fn new(mode: AuthMode, jwt: Option<JwtCodec>, api_key: Option<String>) -> Self {
        Self {
            mode,
            jwt,
            api_key: api_key.map(Arc::new),
        }
    }
}

impl<S> Layer<S> for AuthLayer {
    type Service = AuthService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        AuthService {
            inner,
            mode: self.mode,
            jwt: self.jwt.clone(),
            api_key: self.api_key.clone(),
        }
    }
}

#[derive(Clone)]
pub struct AuthService<S> {
    inner: S,
    mode: AuthMode,
    jwt: Option<JwtCodec>,
    api_key: Option<Arc<String>>,
}

/// Methods that bootstrap a session and are therefore exempt from auth.
fn is_exempt(path: &str) -> bool {
    path.ends_with("/Register") || path.ends_with("/Login")
}

fn bearer<B>(req: &http::Request<B>) -> Option<&str> {
    req.headers()
        .get(http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
}

impl<S, ReqBody> Service<http::Request<ReqBody>> for AuthService<S>
where
    S: Service<http::Request<ReqBody>, Response = http::Response<BoxBody>> + Clone + Send + 'static,
    S::Future: Send + 'static,
    ReqBody: Send + 'static,
{
    type Response = http::Response<BoxBody>;
    type Error = S::Error;
    type Future = std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>,
    >;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: http::Request<ReqBody>) -> Self::Future {
        let path = req.uri().path().to_string();
        let exempt = is_exempt(&path);

        // 1) Optional API key gate (applies in both modes; exempt auth RPCs so a
        //    client can register/login before it has anything else). BC: when no
        //    key is configured this is skipped entirely.
        if !exempt {
            if let Some(expected) = &self.api_key {
                let ok = bearer(&req).is_some_and(|t| t == expected.as_str());
                if !ok {
                    return reject("missing or invalid API key");
                }
            }
        }

        // 2) Identity. Single mode: inject a context with no user (BC). Multi
        //    mode: verify the JWT; require it for non-exempt RPCs.
        match self.mode {
            AuthMode::Single => {
                req.extensions_mut().insert(AuthContext {
                    mode: AuthMode::Single,
                    user_id: None,
                });
            }
            AuthMode::Multi => {
                // In multi mode an API key (if any) is separate from the JWT, so
                // we still need to read the JWT from the bearer header. When both
                // are configured the bearer header carries the JWT and the API
                // key is not usable as a header credential simultaneously; the
                // standalone server does not set both. Verify the JWT here.
                let user_id = match (&self.jwt, bearer(&req)) {
                    (Some(jwt), Some(token)) => jwt.verify(token),
                    _ => None,
                };
                if !exempt && user_id.is_none() {
                    return reject("missing or invalid bearer token");
                }
                req.extensions_mut().insert(AuthContext {
                    mode: AuthMode::Multi,
                    user_id,
                });
            }
        }

        let mut inner = self.inner.clone();
        Box::pin(async move { inner.call(req).await })
    }
}

/// Short-circuit with a gRPC `unauthenticated` status as an HTTP response.
fn reject<E>(
    msg: &str,
) -> std::pin::Pin<
    Box<dyn std::future::Future<Output = Result<http::Response<BoxBody>, E>> + Send>,
> {
    let resp = Status::unauthenticated(msg.to_string()).into_http();
    Box::pin(async move { Ok(resp) })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_mode() {
        assert_eq!(AuthMode::parse("multi"), AuthMode::Multi);
        assert_eq!(AuthMode::parse("MULTI"), AuthMode::Multi);
        assert_eq!(AuthMode::parse("single"), AuthMode::Single);
        assert_eq!(AuthMode::parse(""), AuthMode::Single);
        assert_eq!(AuthMode::parse("bogus"), AuthMode::Single);
    }

    #[test]
    fn exempt_paths() {
        assert!(is_exempt("/gitdeck.v1.Gitdeck/Register"));
        assert!(is_exempt("/gitdeck.v1.Gitdeck/Login"));
        assert!(!is_exempt("/gitdeck.v1.Gitdeck/ListTasks"));
    }
}
