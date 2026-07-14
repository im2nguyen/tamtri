use std::path::Path;

use reqwest::header::{CONTENT_TYPE, HeaderMap, HeaderValue};
use reqwest::{Client, StatusCode};
use serde_json::{Value, json};

use crate::harness::ModelInfo;
use crate::{CoreError, Result};

#[derive(Clone)]
pub struct OpenCodeClient {
    client: Client,
    base_url: String,
    directory: String,
}

impl OpenCodeClient {
    pub fn new(base_url: String, directory: String) -> Result<Self> {
        let client = Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|err| CoreError::Protocol(format!("OpenCode HTTP client failed: {err}")))?;
        Ok(Self {
            client,
            base_url,
            directory,
        })
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub async fn create_session(&self) -> Result<String> {
        let response = self.post("/session", json!({})).await?;
        session_id_from(&response)
    }

    pub async fn resume_session(&self, session_id: &str) -> Result<String> {
        Ok(session_id.to_string())
    }

    pub async fn prompt_async(&self, session_id: &str, prompt: &str, model_id: &str) -> Result<()> {
        let mut body = json!({
            "parts": [{ "type": "text", "text": prompt }]
        });
        if !model_id.trim().is_empty()
            && model_id != "default"
            && let Some(object) = body.as_object_mut()
        {
            object.insert("model".to_string(), json!(model_id));
        }
        let path = format!("/session/{session_id}/prompt_async");
        let response = self
            .client
            .post(self.url(&path))
            .headers(self.headers())
            .json(&body)
            .send()
            .await
            .map_err(map_reqwest)?;
        if response.status() == StatusCode::NO_CONTENT || response.status().is_success() {
            return Ok(());
        }
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        Err(CoreError::Protocol(format!(
            "OpenCode prompt_async failed ({status}): {text}"
        )))
    }

    pub async fn abort_session(&self, session_id: &str) -> Result<()> {
        let path = format!("/session/{session_id}/abort");
        let _ = self.post(&path, json!({})).await?;
        Ok(())
    }

    pub async fn respond_permission(
        &self,
        session_id: &str,
        permission_id: &str,
        allow: bool,
    ) -> Result<()> {
        let path = format!("/session/{session_id}/permissions/{permission_id}");
        let body = json!({
            "response": if allow { "allow" } else { "deny" }
        });
        let _ = self.post(&path, body).await?;
        Ok(())
    }

    pub async fn list_models(&self) -> Result<Vec<ModelInfo>> {
        let response = self.get("/config/providers").await?;
        let providers = response
            .get("providers")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let mut models = Vec::new();
        for provider in providers {
            let provider_id = provider
                .get("id")
                .or_else(|| provider.get("providerID"))
                .and_then(Value::as_str)
                .unwrap_or_default();
            let Some(items) = provider.get("models").and_then(Value::as_array) else {
                continue;
            };
            for model in items {
                let Some(model_id) = model.get("id").and_then(Value::as_str) else {
                    continue;
                };
                let composite = if provider_id.is_empty() {
                    model_id.to_string()
                } else {
                    format!("{provider_id}/{model_id}")
                };
                let display_name = model
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or(model_id)
                    .to_string();
                models.push(ModelInfo {
                    id: composite,
                    display_name,
                });
            }
        }
        Ok(models)
    }

    pub async fn subscribe_events(&self) -> Result<reqwest::Response> {
        self.client
            .get(self.url("/event"))
            .headers(self.headers())
            .header(reqwest::header::ACCEPT, "text/event-stream")
            .send()
            .await
            .map_err(map_reqwest)
    }

    async fn get(&self, path: &str) -> Result<Value> {
        let response = self
            .client
            .get(self.url(path))
            .headers(self.headers())
            .send()
            .await
            .map_err(map_reqwest)?;
        parse_json_response(response).await
    }

    async fn post(&self, path: &str, body: Value) -> Result<Value> {
        let response = self
            .client
            .post(self.url(path))
            .headers(self.headers())
            .json(&body)
            .send()
            .await
            .map_err(map_reqwest)?;
        parse_json_response(response).await
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url.trim_end_matches('/'), path)
    }

    fn headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-opencode-directory",
            HeaderValue::from_str(&encode_uri_component(&self.directory))
                .unwrap_or_else(|_| HeaderValue::from_static("")),
        );
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers
    }
}

pub fn session_id_from(value: &Value) -> Result<String> {
    let session_id = value
        .get("id")
        .or_else(|| value.get("sessionID"))
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_default();
    if session_id.is_empty() {
        return Err(CoreError::Protocol(
            "OpenCode session response missing id".to_string(),
        ));
    }
    Ok(session_id)
}

async fn parse_json_response(response: reqwest::Response) -> Result<Value> {
    let status = response.status();
    if status == StatusCode::NO_CONTENT {
        return Ok(Value::Null);
    }
    let text = response.text().await.map_err(map_reqwest)?;
    if !status.is_success() {
        return Err(CoreError::Protocol(format!(
            "OpenCode HTTP {status}: {text}"
        )));
    }
    if text.trim().is_empty() {
        return Ok(Value::Null);
    }
    serde_json::from_str(&text)
        .map_err(|err| CoreError::Protocol(format!("invalid OpenCode JSON: {err}")))
}

fn map_reqwest(err: reqwest::Error) -> CoreError {
    CoreError::Protocol(format!("OpenCode HTTP request failed: {err}"))
}

fn encode_uri_component(input: &str) -> String {
    input
        .chars()
        .map(|ch| match ch {
            'A'..='Z'
            | 'a'..='z'
            | '0'..='9'
            | '-'
            | '_'
            | '.'
            | '!'
            | '~'
            | '*'
            | '\''
            | '('
            | ')' => ch.to_string(),
            _ => format!("%{:02X}", ch as u8),
        })
        .collect()
}

pub fn absolute_directory(path: &Path) -> Result<String> {
    crate::harness::opencode::server::absolute_directory(path)
}
