use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use sha2::{Digest, Sha256};
use url::Url;

use crate::tokens::{OAuthToken, TokenError};

pub struct PkcePair {
    pub verifier: String,
    pub challenge: String,
}

/// S256 PKCE: `challenge = base64url_nopad(sha256(verifier))`.
pub fn generate_pkce(verifier: impl Into<String>) -> PkcePair {
    let verifier = verifier.into();
    let digest = Sha256::digest(verifier.as_bytes());
    let challenge = URL_SAFE_NO_PAD.encode(digest);
    PkcePair {
        verifier,
        challenge,
    }
}

pub struct AuthRequest<'a> {
    pub authorize_endpoint: &'a str,
    pub client_id: &'a str,
    pub redirect_uri: &'a str,
    pub scope: &'a str,
    pub state: &'a str,
    pub pkce_challenge: Option<&'a str>,
}

pub fn build_authorize_url(req: &AuthRequest) -> String {
    let mut url = Url::parse(req.authorize_endpoint).expect("invalid authorize endpoint");
    {
        let mut pairs = url.query_pairs_mut();
        pairs.append_pair("response_type", "code");
        pairs.append_pair("client_id", req.client_id);
        pairs.append_pair("redirect_uri", req.redirect_uri);
        pairs.append_pair("scope", req.scope);
        pairs.append_pair("state", req.state);
        if let Some(challenge) = req.pkce_challenge {
            pairs.append_pair("code_challenge", challenge);
            pairs.append_pair("code_challenge_method", "S256");
        }
    }
    url.to_string()
}

pub struct TokenRequest<'a> {
    pub token_endpoint: &'a str,
    pub client_id: &'a str,
    pub client_secret: Option<&'a str>,
    pub code: &'a str,
    pub redirect_uri: &'a str,
    pub pkce_verifier: Option<&'a str>,
}

#[derive(serde::Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
}

pub async fn exchange_code(
    client: &reqwest::Client,
    req: &TokenRequest<'_>,
) -> Result<OAuthToken, TokenError> {
    let mut form: Vec<(&str, &str)> = vec![
        ("client_id", req.client_id),
        ("code", req.code),
        ("redirect_uri", req.redirect_uri),
    ];
    if let Some(secret) = req.client_secret {
        form.push(("client_secret", secret));
    }
    if let Some(verifier) = req.pkce_verifier {
        form.push(("code_verifier", verifier));
    }

    let resp = client
        .post(req.token_endpoint)
        .header(reqwest::header::ACCEPT, "application/json")
        .form(&form)
        .send()
        .await
        .map_err(|e| TokenError(e.to_string()))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(TokenError(format!(
            "token endpoint returned {status}: {body}"
        )));
    }

    let parsed: TokenResponse = resp.json().await.map_err(|e| TokenError(e.to_string()))?;
    Ok(OAuthToken {
        access_token: parsed.access_token,
        refresh_token: parsed.refresh_token,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn generate_pkce_produces_known_s256_challenge() {
        let pair = generate_pkce("test-verifier-1234567890");
        assert_eq!(pair.verifier, "test-verifier-1234567890");
        // Precomputed: base64url_nopad(sha256("test-verifier-1234567890")).
        assert_eq!(
            pair.challenge,
            "Gx2LV1Kvw_rrHrk344X_Qz0hqvHkKf-7XJ12eAI03T4"
        );
    }

    #[test]
    fn build_authorize_url_includes_params_and_pkce() {
        let req = AuthRequest {
            authorize_endpoint: "https://github.com/login/oauth/authorize",
            client_id: "cid",
            redirect_uri: "http://127.0.0.1:8765/callback",
            scope: "repo read:user",
            state: "xyz",
            pkce_challenge: Some("CHAL"),
        };
        let url = build_authorize_url(&req);
        let parsed = Url::parse(&url).unwrap();
        let pairs: std::collections::HashMap<_, _> = parsed.query_pairs().into_owned().collect();

        assert_eq!(pairs.get("response_type").unwrap(), "code");
        assert_eq!(pairs.get("client_id").unwrap(), "cid");
        assert_eq!(
            pairs.get("redirect_uri").unwrap(),
            "http://127.0.0.1:8765/callback"
        );
        assert_eq!(pairs.get("scope").unwrap(), "repo read:user");
        assert_eq!(pairs.get("state").unwrap(), "xyz");
        assert_eq!(pairs.get("code_challenge").unwrap(), "CHAL");
        assert_eq!(pairs.get("code_challenge_method").unwrap(), "S256");

        // redirect_uri must be percent-encoded in the raw query string.
        assert!(url.contains("redirect_uri=http%3A%2F%2F127.0.0.1%3A8765%2Fcallback"));
    }

    #[test]
    fn build_authorize_url_omits_pkce_when_none() {
        let req = AuthRequest {
            authorize_endpoint: "https://github.com/login/oauth/authorize",
            client_id: "cid",
            redirect_uri: "http://127.0.0.1:8765/callback",
            scope: "repo",
            state: "xyz",
            pkce_challenge: None,
        };
        let url = build_authorize_url(&req);
        assert!(!url.contains("code_challenge"));
        assert!(!url.contains("code_challenge_method"));
    }

    #[tokio::test]
    async fn exchange_code_parses_access_token() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/login/oauth/access_token"))
            .and(header("accept", "application/json"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "access_token": "gho_x",
                "token_type": "bearer"
            })))
            .mount(&server)
            .await;

        let client = reqwest::Client::new();
        let endpoint = format!("{}/login/oauth/access_token", server.uri());
        let req = TokenRequest {
            token_endpoint: &endpoint,
            client_id: "cid",
            client_secret: Some("secret"),
            code: "the-code",
            redirect_uri: "http://127.0.0.1:8765/callback",
            pkce_verifier: None,
        };

        let token = exchange_code(&client, &req).await.unwrap();
        assert_eq!(token.access_token, "gho_x");
        assert_eq!(token.refresh_token, None);
    }

    #[tokio::test]
    async fn exchange_code_maps_non_2xx_to_error() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/login/oauth/access_token"))
            .respond_with(ResponseTemplate::new(401).set_body_string("bad creds"))
            .mount(&server)
            .await;

        let client = reqwest::Client::new();
        let endpoint = format!("{}/login/oauth/access_token", server.uri());
        let req = TokenRequest {
            token_endpoint: &endpoint,
            client_id: "cid",
            client_secret: Some("secret"),
            code: "the-code",
            redirect_uri: "http://127.0.0.1:8765/callback",
            pkce_verifier: None,
        };

        let err = exchange_code(&client, &req).await.unwrap_err();
        assert!(
            err.0.contains("401"),
            "error should mention status: {}",
            err.0
        );
    }
}
