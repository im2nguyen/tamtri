use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::conversation::roots::normalize_root_uri;
use crate::conversation::{Id, Root, RootKind, RootOrigin, RootScope, validate_root};
use crate::vault::naming::slug;
use crate::{CoreError, Result};

const PROJECT_SCHEMA_VERSION: u32 = 1;
const UNFILED_ID_U128: u128 = 0x74616d74_7269_7000_0000_000000000001;
pub const UNFILED_PROJECT_NAME: &str = "Unfiled";

pub fn unfiled_project_id() -> Id {
    Id::from_u128(UNFILED_ID_U128)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Project {
    pub id: Id,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub roots: Vec<Root>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProjectMeta {
    schema_version: u32,
    #[serde(flatten)]
    project: Project,
}

impl Project {
    pub fn new(name: impl Into<String>) -> Result<Self> {
        let name = normalized_name(name.into())?;
        let now = Utc::now();
        Ok(Self {
            id: Id::now_v7(),
            name,
            created_at: now,
            updated_at: now,
            roots: Vec::new(),
        })
    }

    fn unfiled() -> Self {
        let epoch = DateTime::<Utc>::UNIX_EPOCH;
        Self {
            id: unfiled_project_id(),
            name: UNFILED_PROJECT_NAME.to_string(),
            created_at: epoch,
            updated_at: epoch,
            roots: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProjectStore {
    dir: PathBuf,
}

impl ProjectStore {
    pub fn new(vault_root: &Path) -> Result<Self> {
        let store = Self {
            dir: vault_root.join("projects"),
        };
        fs::create_dir_all(&store.dir)?;
        store.ensure_unfiled()?;
        Ok(store)
    }

    pub fn list(&self) -> Result<Vec<Project>> {
        let mut projects = Vec::new();
        for entry in fs::read_dir(&self.dir)? {
            let path = entry?.path();
            if !path.is_dir() {
                continue;
            }
            if let Ok(project) = read_project(&path) {
                projects.push(project);
            }
        }
        projects.sort_by(|a, b| {
            (a.id != unfiled_project_id())
                .cmp(&(b.id != unfiled_project_id()))
                .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
                .then_with(|| a.id.cmp(&b.id))
        });
        Ok(projects)
    }

    pub fn load(&self, id: Id) -> Result<Project> {
        let dir = self.resolve(id)?;
        read_project(&dir)
    }

    pub fn create(&self, project: &Project) -> Result<()> {
        if self.resolve(project.id).is_ok() {
            return Err(CoreError::ProjectAlreadyExists(project.id));
        }
        let dir = self
            .dir
            .join(format!("{}--{}", slug(&project.name), project.id.simple()));
        fs::create_dir(&dir)?;
        write_project_atomic(&dir, project)
    }

    pub fn save(&self, project: &Project) -> Result<()> {
        let dir = self.resolve(project.id)?;
        write_project_atomic(&dir, project)
    }

    pub fn delete(&self, id: Id) -> Result<()> {
        if id == unfiled_project_id() {
            return Err(CoreError::UnfiledProjectImmutable);
        }
        fs::remove_dir_all(self.resolve(id)?)?;
        Ok(())
    }

    pub fn update_name(&self, id: Id, name: String) -> Result<Project> {
        if id == unfiled_project_id() {
            return Err(CoreError::UnfiledProjectImmutable);
        }
        let mut project = self.load(id)?;
        project.name = normalized_name(name)?;
        project.updated_at = Utc::now();
        self.save(&project)?;
        Ok(project)
    }

    pub fn attach_root(
        &self,
        id: Id,
        name: String,
        uri: String,
        kind: RootKind,
        scope: RootScope,
    ) -> Result<Root> {
        if id == unfiled_project_id() {
            return Err(CoreError::UnfiledProjectImmutable);
        }
        let mut project = self.load(id)?;
        let root = Root {
            id: Id::now_v7().to_string(),
            name,
            uri: normalize_root_uri(&uri, &kind)?,
            kind,
            scope,
            origin: RootOrigin::Project,
        };
        validate_root(&root)?;
        project.roots.push(root.clone());
        project.updated_at = Utc::now();
        self.save(&project)?;
        Ok(root)
    }

    pub fn remove_root(&self, id: Id, root_id: &str) -> Result<Root> {
        if id == unfiled_project_id() {
            return Err(CoreError::UnfiledProjectImmutable);
        }
        let mut project = self.load(id)?;
        let index = project
            .roots
            .iter()
            .position(|root| root.id == root_id)
            .ok_or_else(|| CoreError::ProjectRootNotFound(root_id.to_string()))?;
        let root = project.roots.remove(index);
        project.updated_at = Utc::now();
        self.save(&project)?;
        Ok(root)
    }

    fn ensure_unfiled(&self) -> Result<()> {
        let id = unfiled_project_id();
        if self.resolve(id).is_err() {
            self.create(&Project::unfiled())?;
        }
        Ok(())
    }

    fn resolve(&self, id: Id) -> Result<PathBuf> {
        let suffix = format!("--{}", id.simple());
        for entry in fs::read_dir(&self.dir)? {
            let path = entry?.path();
            if path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.ends_with(&suffix))
                && read_project(&path).is_ok_and(|project| project.id == id)
            {
                return Ok(path);
            }
        }
        Err(CoreError::ProjectNotFound(id))
    }
}

pub fn effective_roots(project_roots: &[Root], conversation_roots: &[Root]) -> Vec<Root> {
    let mut roots = Vec::new();
    for root in project_roots.iter().chain(conversation_roots) {
        if roots
            .iter()
            .any(|existing: &Root| existing.kind == root.kind && existing.uri == root.uri)
        {
            continue;
        }
        roots.push(root.clone());
    }
    roots
}

fn normalized_name(name: String) -> Result<String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(CoreError::MalformedVault(
            "project name is required".to_string(),
        ));
    }
    Ok(trimmed.to_string())
}

fn read_project(dir: &Path) -> Result<Project> {
    let raw = fs::read_to_string(dir.join("meta.json"))?;
    let meta: ProjectMeta = serde_json::from_str(&raw)?;
    if meta.schema_version > PROJECT_SCHEMA_VERSION {
        return Err(CoreError::UnsupportedProjectSchemaVersion(
            meta.schema_version,
        ));
    }
    Ok(meta.project)
}

fn write_project_atomic(dir: &Path, project: &Project) -> Result<()> {
    let meta = ProjectMeta {
        schema_version: PROJECT_SCHEMA_VERSION,
        project: project.clone(),
    };
    let tmp = dir.join("meta.json.tmp");
    fs::write(&tmp, serde_json::to_string_pretty(&meta)?)?;
    fs::rename(tmp, dir.join("meta.json"))?;
    Ok(())
}
