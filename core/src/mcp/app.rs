//! MCP Apps resource model and gateway-side template registry.
//!
//! # App bridge (M7 PR2 design note)
//!
//! MCP App JavaScript talks to the host through a narrow JSON-RPC channel (`ui/*` methods).
//! Every App-initiated tool call or resource read routes through the same gateway consent/audit
//! path as direct harness tool calls. The MCP App bridge is isolated from the trusted React
//! transcript renderer bridge: App code never receives vault, credential, or renderer intents.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use serde_json::Value;
use tokio::sync::Mutex;

use crate::mcp::capabilities::{apps_available, TamtriFeatureSupport};
use crate::mcp::protocol::{CallToolResult, Resource, ServerCapabilities, Tool};
use crate::{CoreError, Result};

pub const MCP_APP_MIME: &str = "text/html;profile=mcp-app";

/// Declared network origin an App template may reach (CSP `connectDomains` / `resourceDomains`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Origin(pub String);

impl Origin {
    pub fn parse(raw: &str) -> Result<Self> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Err(CoreError::Protocol("app origin must not be empty".to_string()));
        }
        if trimmed.contains(char::is_whitespace) {
            return Err(CoreError::Protocol(format!(
                "app origin must not contain whitespace: {trimmed}"
            )));
        }
        Ok(Self(trimmed.to_string()))
    }
}

/// HTML template declared by a downstream server and validated before any webview loads.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AppTemplate {
    pub template_ref: String,
    pub server_id: String,
    pub html: String,
    pub allowed_origins: Vec<Origin>,
    pub metadata: Value,
}

/// Live App instance materialized from a tool or resource return.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AppInstance {
    pub uri: String,
    pub template_ref: String,
    pub server_id: String,
    pub state: Value,
    pub origin_tool_call_id: Option<String>,
}

#[derive(Debug, Default, Clone)]
pub struct AppTemplateRegistry {
    templates: HashMap<String, AppTemplate>,
    tool_resource_uris: HashMap<String, String>,
    declared_refs: HashSet<String>,
}

impl AppTemplateRegistry {
    pub fn insert_template(&mut self, template: AppTemplate) {
        self.declared_refs.insert(template.template_ref.clone());
        self.templates
            .insert(template.template_ref.clone(), template);
    }

    pub fn template(&self, template_ref: &str) -> Option<&AppTemplate> {
        self.templates.get(template_ref)
    }

    pub fn declare_ref(&mut self, template_ref: &str) {
        if is_ui_resource_uri(template_ref) {
            self.declared_refs.insert(template_ref.to_string());
        }
    }

    pub fn is_declared(&self, template_ref: &str) -> bool {
        self.declared_refs.contains(template_ref)
            || self.tool_resource_uris.values().any(|uri| uri == template_ref)
    }

    pub fn record_tool_uri(&mut self, tool_name: &str, resource_uri: String) {
        self.declare_ref(&resource_uri);
        self.tool_resource_uris
            .insert(tool_name.to_string(), resource_uri);
    }

    pub fn tool_resource_uri(&self, tool_name: &str) -> Option<&str> {
        self.tool_resource_uris
            .get(tool_name)
            .map(String::as_str)
    }
}

pub fn is_ui_resource_uri(uri: &str) -> bool {
    uri.starts_with("ui://")
}

pub fn tool_ui_resource_uri(tool: &Tool) -> Option<String> {
    tool.meta.as_ref().and_then(tool_ui_resource_uri_from_meta)
}

pub fn tool_ui_resource_uri_from_meta(meta: &Value) -> Option<String> {
    meta.pointer("/ui/resourceUri")
        .or_else(|| meta.pointer("/ui.resourceUri"))
        .and_then(Value::as_str)
        .map(str::to_string)
}

pub fn tool_ui_resource_uri_from_value(tool: &Value) -> Option<String> {
    tool.pointer("/_meta/ui/resourceUri")
        .or_else(|| tool.pointer("/_meta/ui.resourceUri"))
        .and_then(Value::as_str)
        .map(str::to_string)
}

pub fn allowed_origins_from_csp(csp: Option<&Value>) -> Result<Vec<Origin>> {
    let Some(csp) = csp else {
        return Ok(Vec::new());
    };
    let mut origins = Vec::new();
    for key in ["connectDomains", "resourceDomains", "frameDomains", "baseUriDomains"] {
        if let Some(items) = csp.get(key).and_then(Value::as_array) {
            for item in items {
                if let Some(origin) = item.as_str() {
                    origins.push(Origin::parse(origin)?);
                }
            }
        }
    }
    origins.sort_by(|left, right| left.0.cmp(&right.0));
    origins.dedup_by(|left, right| left.0 == right.0);
    Ok(origins)
}

pub fn template_from_resource_contents(
    server_id: &str,
    template_ref: &str,
    contents: &[Value],
) -> Result<AppTemplate> {
    if !is_ui_resource_uri(template_ref) {
        return Err(CoreError::Protocol(format!(
            "app template must use ui:// URI scheme: {template_ref}"
        )));
    }
    let entry = contents
        .iter()
        .find(|item| item.get("uri").and_then(Value::as_str) == Some(template_ref))
        .or_else(|| contents.first())
        .ok_or_else(|| {
            CoreError::Protocol(format!(
                "resources/read returned no contents for {template_ref}"
            ))
        })?;
    let mime = entry
        .get("mimeType")
        .or_else(|| entry.get("mime_type"))
        .and_then(Value::as_str)
        .unwrap_or_default();
    if mime != MCP_APP_MIME {
        return Err(CoreError::Protocol(format!(
            "app template must use mime type {MCP_APP_MIME}, got {mime}"
        )));
    }
    let html = entry
        .get("text")
        .and_then(Value::as_str)
        .map(str::to_string)
        .or_else(|| {
            entry
                .get("blob")
                .and_then(Value::as_str)
                .and_then(|blob| {
                    use base64::Engine;
                    base64::engine::general_purpose::STANDARD
                        .decode(blob)
                        .ok()
                        .and_then(|bytes| String::from_utf8(bytes).ok())
                })
        })
        .ok_or_else(|| {
            CoreError::Protocol(format!(
                "app template {template_ref} is missing HTML text content"
            ))
        })?;
    let metadata = entry
        .get("_meta")
        .cloned()
        .unwrap_or_else(|| Value::Object(serde_json::Map::new()));
    let allowed_origins =
        allowed_origins_from_csp(metadata.get("ui").and_then(|ui| ui.get("csp")))?;
    Ok(AppTemplate {
        template_ref: template_ref.to_string(),
        server_id: server_id.to_string(),
        html,
        allowed_origins,
        metadata,
    })
}

pub fn app_instance_from_tool_result(
    server_id: &str,
    template_ref: &str,
    result: &CallToolResult,
    origin_tool_call_id: Option<String>,
) -> AppInstance {
    let state = result
        .structured_content
        .clone()
        .unwrap_or_else(|| Value::Object(serde_json::Map::new()));
    AppInstance {
        uri: template_ref.to_string(),
        template_ref: template_ref.to_string(),
        server_id: server_id.to_string(),
        state,
        origin_tool_call_id,
    }
}

pub fn origin_allowed(template: &AppTemplate, origin: &str) -> bool {
    template
        .allowed_origins
        .iter()
        .any(|allowed| allowed.0 == origin)
}

pub fn navigation_allowed(template: &AppTemplate, url: &str) -> bool {
    let trimmed = url.trim();
    if trimmed.is_empty() || trimmed.starts_with("about:") {
        return true;
    }
    let Ok(parsed) = url::Url::parse(trimmed) else {
        return false;
    };
    let origin = parsed.origin().ascii_serialization();
    template.allowed_origins.iter().any(|allowed| {
        allowed.0 == origin || allowed.0 == trimmed
    })
}

pub fn app_sandbox_csp(allowed_origins: &[Origin]) -> String {
    let connect = allowed_origins
        .iter()
        .map(|origin| origin.0.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    format!(
        "default-src 'none'; script-src 'unsafe-inline'; style-src 'unsafe-inline' 'self'; \
         connect-src {connect} about:; img-src data:; base-uri 'none'; form-action 'none'"
    )
}

/// Injected into App webviews only. Artifacts must never include this script.
pub fn app_bridge_bootstrap_script(handler_name: &str) -> String {
    format!(
        r#"(function() {{
  if (window.__tamtriAppBridgeInstalled) return;
  window.__tamtriAppBridgeInstalled = true;
  const pending = new Map();
  window.tamtri = window.tamtri || {{}};
  window.tamtri.request = function(method, params) {{
    return new Promise(function(resolve, reject) {{
      const id = crypto.randomUUID();
      pending.set(id, {{ resolve: resolve, reject: reject }});
      window.webkit.messageHandlers.{handler_name}.postMessage(JSON.stringify({{
        jsonrpc: "2.0",
        id: id,
        method: method,
        params: params || {{}}
      }}));
    }});
  }};
  window.__tamtriAppBridgeDeliver = function(payload) {{
    try {{
      const msg = JSON.parse(payload);
      const entry = pending.get(String(msg.id));
      if (!entry) return;
      pending.delete(String(msg.id));
      if (msg.error) entry.reject(msg.error);
      else entry.resolve(msg.result);
    }} catch (err) {{
      console.error("tamtri bridge deliver failed", err);
    }}
  }};
}})();"#
    )
}

pub fn require_declared_template<'a>(
    registry: &'a AppTemplateRegistry,
    template_ref: &str,
) -> Result<&'a AppTemplate> {
    if !registry.is_declared(template_ref) {
        return Err(CoreError::Protocol(format!(
            "undeclared app template: {template_ref}"
        )));
    }
    registry.template(template_ref).ok_or_else(|| {
        CoreError::Protocol(format!(
            "declared app template not loaded: {template_ref}"
        ))
    })
}

#[derive(Debug, Default)]
pub struct GatewayAppState {
    per_server: HashMap<String, AppTemplateRegistry>,
}

impl GatewayAppState {
    pub async fn registry_for(
        store: &Arc<Mutex<GatewayAppState>>,
        server_id: &str,
    ) -> AppTemplateRegistry {
        store
            .lock()
            .await
            .per_server
            .get(server_id)
            .cloned()
            .unwrap_or_default()
    }

    pub async fn with_registry<F, T>(
        store: &Arc<Mutex<GatewayAppState>>,
        server_id: &str,
        f: F,
    ) -> T
    where
        F: FnOnce(&mut AppTemplateRegistry) -> T,
    {
        let mut guard = store.lock().await;
        let registry = guard
            .per_server
            .entry(server_id.to_string())
            .or_default();
        f(registry)
    }

    pub fn index_tools(server_id: &str, tools: &[Tool], registry: &mut AppTemplateRegistry) {
        for tool in tools {
            if let Some(uri) = tool_ui_resource_uri(tool) {
                registry.record_tool_uri(&tool.name, uri);
            }
        }
        let _ = server_id;
    }

    pub fn index_tools_from_values(
        server_id: &str,
        tools: &[Value],
        registry: &mut AppTemplateRegistry,
    ) {
        for tool in tools {
            let Some(name) = tool.get("name").and_then(Value::as_str) else {
                continue;
            };
            if let Some(uri) = tool_ui_resource_uri_from_value(tool) {
                registry.record_tool_uri(name, uri);
            }
        }
        let _ = server_id;
    }

    pub fn index_resources(resources: &[Resource], registry: &mut AppTemplateRegistry) {
        for resource in resources {
            registry.declare_ref(&resource.uri);
        }
    }
}

pub fn apps_enabled_for_server(
    capabilities: Option<&ServerCapabilities>,
    support: TamtriFeatureSupport,
) -> bool {
    capabilities
        .map(|caps| apps_available(caps, support))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn app_template_declared_origin_loads() {
        let contents = [json!({
            "uri": "ui://m7-app/demo",
            "mimeType": MCP_APP_MIME,
            "text": "<!DOCTYPE html><html><body>demo</body></html>",
            "_meta": {
                "ui": {
                    "csp": {
                        "connectDomains": ["https://api.example.com"]
                    }
                }
            }
        })];
        let template =
            template_from_resource_contents("m7-app", "ui://m7-app/demo", &contents).unwrap();
        assert_eq!(template.template_ref, "ui://m7-app/demo");
        assert!(template.html.contains("demo"));
        assert_eq!(template.allowed_origins.len(), 1);
        assert_eq!(template.allowed_origins[0].0, "https://api.example.com");
        assert!(origin_allowed(&template, "https://api.example.com"));
        assert!(!origin_allowed(&template, "https://evil.example"));
    }

    #[test]
    fn app_template_undeclared_origin_blocked() {
        let mut registry = AppTemplateRegistry::default();
        let err = require_declared_template(&registry, "ui://missing/template")
            .expect_err("undeclared template");
        assert!(err.to_string().contains("undeclared"));

        registry.insert_template(AppTemplate {
            template_ref: "ui://m7-app/demo".into(),
            server_id: "m7-app".into(),
            html: "<html></html>".into(),
            allowed_origins: vec![Origin("https://api.example.com".into())],
            metadata: json!({}),
        });
        assert!(require_declared_template(&registry, "ui://m7-app/demo").is_ok());
    }

    #[test]
    fn bad_origin_rejected() {
        assert!(Origin::parse("").is_err());
        assert!(Origin::parse("https://bad origin").is_err());
    }

    #[test]
    fn app_instance_uses_structured_content_state() {
        let result = CallToolResult {
            content: vec![json!({"type": "text", "text": "summary"})],
            is_error: Some(false),
            structured_content: Some(json!({"count": 3})),
        };
        let instance = app_instance_from_tool_result(
            "m7-app",
            "ui://m7-app/demo",
            &result,
            Some("tool-1".into()),
        );
        assert_eq!(instance.state["count"], 3);
        assert_eq!(instance.origin_tool_call_id.as_deref(), Some("tool-1"));
    }
}
