use url::Url;

use crate::{CoreError, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedHandoffUrl {
    pub origin: String,
    pub url: String,
}

pub fn validate_handoff_url(raw: &str) -> Result<ValidatedHandoffUrl> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(CoreError::Protocol("handoff URL is empty".to_string()));
    }
    if trimmed.contains('\\') {
        return Err(CoreError::Protocol(
            "handoff URL must not contain backslashes".to_string(),
        ));
    }

    let parsed = Url::parse(trimmed)
        .map_err(|err| CoreError::Protocol(format!("invalid handoff URL: {err}")))?;

    if !parsed.has_host() {
        return Err(CoreError::Protocol(
            "handoff URL must include a host".to_string(),
        ));
    }
    if parsed.username() != "" || parsed.password().is_some() {
        return Err(CoreError::Protocol(
            "handoff URL must not include userinfo".to_string(),
        ));
    }

    let scheme = parsed.scheme().to_ascii_lowercase();
    let host = parsed
        .host_str()
        .ok_or_else(|| CoreError::Protocol("handoff URL must include a host".to_string()))?;

    match scheme.as_str() {
        "https" => {}
        "http" if is_loopback_host(host) => {}
        "http" => {
            return Err(CoreError::Protocol(
                "handoff URL must use HTTPS except loopback OAuth callbacks".to_string(),
            ));
        }
        _ => {
            return Err(CoreError::Protocol(format!(
                "unsupported handoff URL scheme: {scheme}"
            )));
        }
    }

    if parsed.fragment().is_some() {
        return Err(CoreError::Protocol(
            "handoff URL must not include a fragment".to_string(),
        ));
    }

    let origin = format!(
        "{}://{}",
        scheme,
        host
    );
    let port = parsed.port();
    let origin = if let Some(port) = port {
        format!("{origin}:{port}")
    } else {
        origin
    };

    Ok(ValidatedHandoffUrl {
        origin,
        url: parsed.to_string(),
    })
}

pub fn redact_url_for_audit(raw: &str) -> String {
    let Ok(mut parsed) = Url::parse(raw.trim()) else {
        return raw.to_string();
    };
    parsed.set_query(None);
    parsed.to_string()
}

pub fn is_loopback_host(host: &str) -> bool {
    let normalized = host.trim_matches('[').trim_matches(']').to_ascii_lowercase();
    matches!(
        normalized.as_str(),
        "localhost" | "127.0.0.1" | "::1" | "0:0:0:0:0:0:0:1"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn https_url_is_allowed() {
        let validated = validate_handoff_url("https://example.com/oauth/authorize?state=abc")
            .unwrap();
        assert_eq!(validated.origin, "https://example.com");
        assert!(validated.url.contains("state=abc"));
    }

    #[test]
    fn http_non_loopback_is_rejected() {
        assert!(validate_handoff_url("http://example.com/login").is_err());
    }

    #[test]
    fn http_loopback_is_allowed() {
        let validated = validate_handoff_url("http://127.0.0.1:3847/callback").unwrap();
        assert_eq!(validated.origin, "http://127.0.0.1:3847");
    }

    #[test]
    fn userinfo_url_is_rejected() {
        assert!(validate_handoff_url("https://user:pass@example.com/path").is_err());
    }

    #[test]
    fn query_string_is_redacted_for_audit() {
        let redacted = redact_url_for_audit("https://example.com/path?client_id=x&code=secret");
        assert_eq!(redacted, "https://example.com/path");
    }
}
