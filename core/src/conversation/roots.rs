use std::path::{Component, Path, PathBuf};

use url::Url;

use crate::conversation::{Conversation, Root, RootKind, RootOrigin, RootScope};
use crate::{CoreError, Result};

pub fn attach_root(
    conversation: &mut Conversation,
    name: impl Into<String>,
    uri: impl Into<String>,
    kind: RootKind,
    scope: RootScope,
) -> Result<Root> {
    let uri = normalize_root_uri(&uri.into(), &kind)?;
    let root = Root {
        id: uuid::Uuid::now_v7().to_string(),
        name: name.into(),
        uri,
        kind,
        scope,
        origin: RootOrigin::Conversation,
    };
    validate_root(&root)?;
    conversation.roots.push(root.clone());
    conversation.touch();
    Ok(root)
}

pub fn remove_root(conversation: &mut Conversation, root_id: &str) -> Result<Root> {
    let index = conversation
        .roots
        .iter()
        .position(|root| root.id == root_id)
        .ok_or_else(|| CoreError::NotFound(uuid::Uuid::nil()))?;
    let removed = conversation.roots.remove(index);
    conversation.touch();
    Ok(removed)
}

/// Filesystem roots require a shell security-scoped bookmark before the gateway can read paths.
pub fn filesystem_root_requires_bookmark(root: &Root) -> bool {
    matches!(root.kind, RootKind::Filesystem)
}

/// Returns the user-facing error state when a filesystem root has no bookmark.
pub fn missing_bookmark_error_state(root: &Root, bookmark_present: bool) -> Option<String> {
    if filesystem_root_requires_bookmark(root) && !bookmark_present {
        Some(format!(
            "Missing access bookmark for root \"{}\". Re-pick the folder in conversation settings.",
            root.name
        ))
    } else {
        None
    }
}

pub fn validate_root(root: &Root) -> Result<()> {
    if root.uri.trim().is_empty() {
        return Err(CoreError::MalformedVault(
            "root uri is required".to_string(),
        ));
    }
    if root.name.trim().is_empty() {
        return Err(CoreError::MalformedVault(
            "root name is required".to_string(),
        ));
    }
    if root.id.trim().is_empty() {
        return Err(CoreError::MalformedVault("root id is required".to_string()));
    }
    if matches!(root.kind, RootKind::Filesystem) {
        root_filesystem_path(&root.uri)?;
    }
    Ok(())
}

pub fn normalize_root_uri(uri: &str, kind: &RootKind) -> Result<String> {
    let trimmed = uri.trim();
    if trimmed.is_empty() {
        return Err(CoreError::MalformedVault(
            "root uri is required".to_string(),
        ));
    }
    if !matches!(kind, RootKind::Filesystem) {
        return Ok(trimmed.to_string());
    }
    if trimmed.starts_with("file://") {
        return Ok(trimmed.to_string());
    }
    let path = Path::new(trimmed);
    if !path.is_absolute() {
        return Err(CoreError::MalformedVault(format!(
            "filesystem root must be an absolute path or file:// URI: {trimmed}"
        )));
    }
    Url::from_file_path(path)
        .map(|url| url.to_string())
        .map_err(|_| {
            CoreError::MalformedVault(format!("filesystem root path is invalid: {trimmed}"))
        })
}

pub fn root_filesystem_path(uri: &str) -> Result<PathBuf> {
    if let Ok(url) = Url::parse(uri)
        && url.scheme() == "file"
    {
        return url.to_file_path().map_err(|_| {
            CoreError::MalformedVault(format!("root uri is not a local path: {uri}"))
        });
    }
    let path = PathBuf::from(uri);
    if path.is_absolute() {
        return Ok(path);
    }
    Err(CoreError::MalformedVault(format!(
        "filesystem root must be an absolute path or file:// URI: {uri}"
    )))
}

pub fn is_path_under_root(path: &str, root: &Root) -> Result<bool> {
    if !matches!(root.kind, RootKind::Filesystem) {
        return Ok(false);
    }
    let root_path = root_filesystem_path(&root.uri)?;
    let candidate = resolve_candidate_path(path)?;
    Ok(path_starts_with_root(&candidate, &root_path))
}

fn path_starts_with_root(candidate: &Path, root: &Path) -> bool {
    let root_canonical = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    if let Ok(candidate_canonical) = candidate.canonicalize() {
        return candidate_canonical.starts_with(&root_canonical);
    }
    if let (Some(parent), Some(name)) = (candidate.parent(), candidate.file_name())
        && let Ok(parent_canonical) = parent.canonicalize()
    {
        return parent_canonical.join(name).starts_with(&root_canonical);
    }
    candidate.starts_with(&root_canonical)
}

pub fn is_path_under_any_root(path: &str, roots: &[Root]) -> Result<bool> {
    for root in roots {
        if is_path_under_root(path, root)? {
            return Ok(true);
        }
    }
    Ok(false)
}

fn resolve_candidate_path(path: &str) -> Result<PathBuf> {
    if let Ok(url) = Url::parse(path)
        && url.scheme() == "file"
    {
        return url
            .to_file_path()
            .map_err(|_| CoreError::MalformedVault(format!("path is not local: {path}")));
    }
    let candidate = PathBuf::from(path);
    if candidate.is_absolute() {
        return Ok(candidate);
    }
    for component in candidate.components() {
        if matches!(component, Component::ParentDir) {
            return Err(CoreError::MalformedVault(format!(
                "path must not contain parent traversal: {path}"
            )));
        }
    }
    Ok(candidate)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conversation::Conversation;

    #[test]
    fn attach_and_remove_root() {
        let mut conversation = Conversation::new("roots");
        let root = attach_root(
            &mut conversation,
            "Data",
            "/tmp/tamtri-root-data",
            RootKind::Filesystem,
            RootScope::Conversation,
        )
        .unwrap();
        assert_eq!(conversation.roots.len(), 1);
        assert!(root.uri.starts_with("file://"));
        let removed = remove_root(&mut conversation, &root.id).unwrap();
        assert_eq!(removed.id, root.id);
        assert!(conversation.roots.is_empty());
    }

    #[test]
    fn fork_copies_root_refs() {
        let mut conversation = Conversation::new("parent");
        attach_root(
            &mut conversation,
            "Docs",
            "file:///tmp/docs",
            RootKind::Filesystem,
            RootScope::Conversation,
        )
        .unwrap();
        let fork = conversation.fork();
        assert_eq!(fork.roots.len(), 1);
        assert_eq!(fork.roots[0].uri, conversation.roots[0].uri);
        assert_eq!(fork.roots[0].id, conversation.roots[0].id);
    }

    #[test]
    fn missing_bookmark_error_state_for_filesystem_only() {
        let root = Root {
            id: "r1".into(),
            name: "Reports".into(),
            uri: "file:///tmp/reports".into(),
            kind: RootKind::Filesystem,
            scope: RootScope::Conversation,
            origin: RootOrigin::Conversation,
        };
        assert!(missing_bookmark_error_state(&root, false).is_some());
        assert!(missing_bookmark_error_state(&root, true).is_none());
    }
}
