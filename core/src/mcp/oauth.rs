use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use url::Url;

use crate::config::OAuthConfig;
use crate::mcp::url_handoff::{ValidatedHandoffUrl, validate_handoff_url};
use crate::{CoreError, Result};

/// JSON blob stored in the keychain at `OAuthConfig.token_ref`. Never written to events.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StoredOAuthBundle {
    pub access_token: String,
    #[serde(default)]
    pub refresh_token: Option<String>,
    #[serde(default)]
    pub expires_at: Option<i64>,
    #[serde(default)]
    pub reauth_required: bool,
}

pub const OAUTH_REFRESH_LEEWAY_SECS: i64 = 300;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PkceChallenge {
    pub verifier: String,
    pub challenge: String,
    pub method: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OAuthTokenBundle {
    pub access_ref: String,
    pub refresh_ref: Option<String>,
    pub expires_at: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OAuthConnectionStatus {
    NotConfigured,
    MissingCredential,
    Connected,
    Expired,
    ReauthRequired,
}

#[derive(Clone, Deserialize)]
pub struct TokenEndpointResponse {
    pub access_token: String,
    #[serde(default)]
    pub refresh_token: Option<String>,
    #[serde(default)]
    pub expires_in: Option<u64>,
    #[serde(default)]
    pub token_type: Option<String>,
}

pub fn generate_pkce() -> PkceChallenge {
    let verifier = format!("{}{}", uuid::Uuid::now_v7(), uuid::Uuid::now_v7());
    let digest = Sha256::digest(verifier.as_bytes());
    let challenge = URL_SAFE_NO_PAD.encode(digest);
    PkceChallenge {
        verifier,
        challenge,
        method: "S256".to_string(),
    }
}

pub fn build_authorization_url(
    config: &OAuthConfig,
    redirect_uri: &str,
    pkce: &PkceChallenge,
    state: &str,
) -> Result<String> {
    let authorization_endpoint = config.authorization_endpoint.as_deref().ok_or_else(|| {
        CoreError::Protocol("oauth authorization_endpoint is required".to_string())
    })?;
    let validated = validate_handoff_url(authorization_endpoint)?;
    let redirect = validate_handoff_url(redirect_uri)?;

    let mut url = Url::parse(&validated.url)
        .map_err(|err| CoreError::Protocol(format!("invalid authorization URL: {err}")))?;
    {
        let mut query = url.query_pairs_mut();
        query.append_pair("response_type", "code");
        query.append_pair("client_id", &config.client_id);
        query.append_pair("redirect_uri", &redirect.url);
        query.append_pair("state", state);
        query.append_pair("code_challenge", &pkce.challenge);
        query.append_pair("code_challenge_method", &pkce.method);
        if !config.scopes.is_empty() {
            query.append_pair("scope", &config.scopes.join(" "));
        }
    }
    Ok(url.to_string())
}

pub fn validate_callback_url(raw: &str, expected_state: &str) -> Result<(String, String)> {
    let validated = validate_handoff_url(raw)?;
    let parsed = Url::parse(&validated.url)
        .map_err(|err| CoreError::Protocol(format!("invalid callback URL: {err}")))?;
    let state = parsed
        .query_pairs()
        .find(|(key, _)| key == "state")
        .map(|(_, value)| value.into_owned())
        .ok_or_else(|| CoreError::Protocol("oauth callback missing state".to_string()))?;
    if state != expected_state {
        return Err(CoreError::Protocol(
            "oauth callback state mismatch".to_string(),
        ));
    }
    let code = parsed
        .query_pairs()
        .find(|(key, _)| key == "code")
        .map(|(_, value)| value.into_owned())
        .ok_or_else(|| CoreError::Protocol("oauth callback missing code".to_string()))?;
    Ok((code, validated.origin))
}

pub async fn exchange_authorization_code(
    client: &reqwest::Client,
    config: &OAuthConfig,
    code: &str,
    redirect_uri: &str,
    pkce: &PkceChallenge,
) -> Result<TokenEndpointResponse> {
    let token_endpoint = config
        .token_endpoint
        .as_deref()
        .ok_or_else(|| CoreError::Protocol("oauth token_endpoint is required".to_string()))?;
    let ValidatedHandoffUrl { url, .. } = validate_handoff_url(token_endpoint)?;
    let response = client
        .post(url)
        .header("accept", "application/json")
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", redirect_uri),
            ("client_id", config.client_id.as_str()),
            ("code_verifier", pkce.verifier.as_str()),
        ])
        .send()
        .await
        .map_err(|err| CoreError::Protocol(format!("oauth token exchange failed: {err}")))?;
    if !response.status().is_success() {
        return Err(CoreError::Protocol(format!(
            "oauth token exchange returned {}",
            response.status()
        )));
    }
    response
        .json::<TokenEndpointResponse>()
        .await
        .map_err(|err| CoreError::Protocol(format!("oauth token response invalid: {err}")))
}

pub async fn refresh_access_token(
    client: &reqwest::Client,
    config: &OAuthConfig,
    refresh_token: &str,
) -> Result<TokenEndpointResponse> {
    let token_endpoint = config
        .token_endpoint
        .as_deref()
        .ok_or_else(|| CoreError::Protocol("oauth token_endpoint is required".to_string()))?;
    let ValidatedHandoffUrl { url, .. } = validate_handoff_url(token_endpoint)?;
    let response = client
        .post(url)
        .header("accept", "application/json")
        .form(&[
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
            ("client_id", config.client_id.as_str()),
        ])
        .send()
        .await
        .map_err(|err| CoreError::Protocol(format!("oauth refresh failed: {err}")))?;
    if !response.status().is_success() {
        return Err(CoreError::Protocol(format!(
            "oauth refresh returned {}",
            response.status()
        )));
    }
    response
        .json::<TokenEndpointResponse>()
        .await
        .map_err(|err| CoreError::Protocol(format!("oauth refresh response invalid: {err}")))
}

pub fn oauth_connection_status(
    config: Option<&OAuthConfig>,
    has_access_token: bool,
    expires_at: Option<i64>,
    reauth_required: bool,
) -> OAuthConnectionStatus {
    if config.is_none() {
        return OAuthConnectionStatus::NotConfigured;
    }
    if reauth_required {
        return OAuthConnectionStatus::ReauthRequired;
    }
    if !has_access_token {
        return OAuthConnectionStatus::MissingCredential;
    }
    if let Some(expires_at) = expires_at
        && expires_at <= Utc::now().timestamp()
    {
        return OAuthConnectionStatus::Expired;
    }
    OAuthConnectionStatus::Connected
}

pub fn parse_stored_oauth(raw: &str) -> Result<StoredOAuthBundle> {
    serde_json::from_str(raw)
        .map_err(|err| CoreError::Protocol(format!("invalid stored oauth bundle: {err}")))
}

pub fn serialize_stored_oauth(bundle: &StoredOAuthBundle) -> Result<String> {
    serde_json::to_string(bundle)
        .map_err(|err| CoreError::Protocol(format!("failed to serialize oauth bundle: {err}")))
}

pub fn stored_oauth_from_token_response(response: &TokenEndpointResponse) -> StoredOAuthBundle {
    let expires_at = response
        .expires_in
        .map(|seconds| Utc::now().timestamp() + i64::try_from(seconds).unwrap_or(i64::MAX));
    StoredOAuthBundle {
        access_token: response.access_token.clone(),
        refresh_token: response.refresh_token.clone(),
        expires_at,
        reauth_required: false,
    }
}

pub fn oauth_needs_refresh(expires_at: Option<i64>) -> bool {
    let Some(expires_at) = expires_at else {
        return false;
    };
    expires_at <= Utc::now().timestamp() + OAUTH_REFRESH_LEEWAY_SECS
}

pub fn oauth_status_label(status: OAuthConnectionStatus) -> &'static str {
    match status {
        OAuthConnectionStatus::NotConfigured => "not_configured",
        OAuthConnectionStatus::MissingCredential => "missing",
        OAuthConnectionStatus::Connected => "connected",
        OAuthConnectionStatus::Expired => "expired",
        OAuthConnectionStatus::ReauthRequired => "reauth_required",
    }
}

pub enum OAuthResolveOutcome {
    AccessToken(String),
    ReauthRequired,
    Missing,
}

pub async fn resolve_oauth_access_token(
    client: &reqwest::Client,
    config: &OAuthConfig,
    stored_raw: &str,
) -> Result<(OAuthResolveOutcome, Option<StoredOAuthBundle>)> {
    let mut bundle = parse_stored_oauth(stored_raw)?;
    if bundle.reauth_required {
        return Ok((OAuthResolveOutcome::ReauthRequired, Some(bundle)));
    }
    if bundle.access_token.is_empty() {
        return Ok((OAuthResolveOutcome::Missing, None));
    }
    if oauth_needs_refresh(bundle.expires_at) {
        let Some(refresh_token) = bundle.refresh_token.clone() else {
            bundle.reauth_required = true;
            return Ok((OAuthResolveOutcome::ReauthRequired, Some(bundle)));
        };
        match refresh_access_token(client, config, &refresh_token).await {
            Ok(tokens) => {
                bundle = stored_oauth_from_token_response(&tokens);
                let access = bundle.access_token.clone();
                return Ok((OAuthResolveOutcome::AccessToken(access), Some(bundle)));
            }
            Err(_) => {
                bundle.reauth_required = true;
                return Ok((OAuthResolveOutcome::ReauthRequired, Some(bundle)));
            }
        }
    }
    Ok((
        OAuthResolveOutcome::AccessToken(bundle.access_token.clone()),
        None,
    ))
}

impl std::fmt::Debug for TokenEndpointResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TokenEndpointResponse")
            .field("access_token", &"[redacted]")
            .field(
                "refresh_token",
                &self.refresh_token.as_ref().map(|_| "[redacted]"),
            )
            .field("expires_in", &self.expires_in)
            .field("token_type", &self.token_type)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    fn sample_config(token_endpoint: String) -> OAuthConfig {
        OAuthConfig {
            issuer: None,
            authorization_endpoint: Some("https://auth.example.com/authorize".to_string()),
            token_endpoint: Some(token_endpoint),
            client_id: "tamtri-client".to_string(),
            scopes: vec!["mcp".to_string()],
            token_ref: "keychain://remote-mcp".to_string(),
        }
    }

    #[test]
    fn pkce_challenge_uses_s256() {
        let pkce = generate_pkce();
        assert_eq!(pkce.method, "S256");
        assert!(!pkce.verifier.is_empty());
        assert!(!pkce.challenge.is_empty());
        assert_ne!(pkce.verifier, pkce.challenge);
    }

    #[test]
    fn authorization_url_includes_pkce_and_state() {
        let config = sample_config("https://auth.example.com/token".to_string());
        let pkce = generate_pkce();
        let url = build_authorization_url(
            &config,
            "http://127.0.0.1:3847/callback",
            &pkce,
            "state-123",
        )
        .unwrap();
        assert!(url.contains("response_type=code"));
        assert!(url.contains("client_id=tamtri-client"));
        assert!(url.contains("code_challenge="));
        assert!(url.contains("code_challenge_method=S256"));
        assert!(url.contains("state=state-123"));
    }

    #[test]
    fn callback_state_must_match() {
        let err = validate_callback_url(
            "http://127.0.0.1:3847/callback?code=abc&state=wrong",
            "expected",
        )
        .unwrap_err();
        assert!(err.to_string().contains("state mismatch"));
    }

    #[tokio::test]
    async fn token_exchange_uses_mock_endpoint() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let token_endpoint = format!("http://127.0.0.1:{}/token", addr.port());

        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0u8; 4096];
            let n = socket.read(&mut buf).await.unwrap();
            let request = String::from_utf8_lossy(&buf[..n]);
            assert!(request.contains("grant_type=authorization_code"));
            assert!(request.contains("code_verifier="));
            let body = r#"{"access_token":"access-1","refresh_token":"refresh-1","expires_in":3600,"token_type":"Bearer"}"#;
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                body.len(),
                body
            );
            socket.write_all(response.as_bytes()).await.unwrap();
        });

        let config = sample_config(token_endpoint);
        let pkce = generate_pkce();
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .unwrap();
        let tokens = exchange_authorization_code(
            &client,
            &config,
            "auth-code",
            "http://127.0.0.1:3847/callback",
            &pkce,
        )
        .await
        .unwrap();
        assert_eq!(tokens.access_token, "access-1");
        assert_eq!(tokens.refresh_token.as_deref(), Some("refresh-1"));
        server.await.unwrap();
    }

    #[tokio::test]
    async fn refresh_failure_marks_reauth_required_status() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let token_endpoint = format!("http://127.0.0.1:{}/token", addr.port());

        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0u8; 4096];
            let _ = socket.read(&mut buf).await.unwrap();
            let body = r#"{"error":"invalid_grant"}"#;
            let response = format!(
                "HTTP/1.1 400 Bad Request\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                body.len(),
                body
            );
            socket.write_all(response.as_bytes()).await.unwrap();
        });

        let config = sample_config(token_endpoint);
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .unwrap();
        let err = refresh_access_token(&client, &config, "stale-refresh")
            .await
            .unwrap_err();
        assert!(err.to_string().contains("400"));
        assert_eq!(
            oauth_connection_status(Some(&config), false, None, true),
            OAuthConnectionStatus::ReauthRequired
        );
        server.await.unwrap();
    }
}
