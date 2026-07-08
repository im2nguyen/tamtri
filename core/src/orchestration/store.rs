//! Recipe files live in `<vault>/recipes/*.json`. Starter templates ship with the core.

use std::fs;
use std::path::{Path, PathBuf};

use crate::orchestration::recipe::{Recipe, RecipeSummary};
use crate::orchestration::run::{OrchestrationRunMeta, RUN_SCHEMA_VERSION};
use crate::{CoreError, Result};

const STARTER_HANDOFF: &str = include_str!("../../recipes/handoff.json");
const STARTER_COMMITTEE: &str = include_str!("../../recipes/committee.json");

pub fn recipes_dir(vault_root: &Path) -> PathBuf {
    vault_root.join("recipes")
}

pub fn orchestration_dir(vault_root: &Path) -> PathBuf {
    vault_root.join("orchestration")
}

pub fn run_dir(vault_root: &Path, run_id: &str) -> PathBuf {
    orchestration_dir(vault_root).join(run_id)
}

pub fn seed_starter_recipes(vault_root: &Path) -> Result<()> {
    let dir = recipes_dir(vault_root);
    fs::create_dir_all(&dir)?;
    for (name, contents) in [("handoff.json", STARTER_HANDOFF), ("committee.json", STARTER_COMMITTEE)] {
        let path = dir.join(name);
        if !path.exists() {
            fs::write(&path, contents)?;
        }
    }
    Ok(())
}

pub fn list_recipes(vault_root: &Path) -> Result<Vec<RecipeSummary>> {
    seed_starter_recipes(vault_root)?;
    let dir = recipes_dir(vault_root);
    let mut summaries = Vec::new();
    for entry in fs::read_dir(&dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        let recipe = read_recipe_file(&path)?;
        summaries.push(recipe.summary());
    }
    summaries.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(summaries)
}

pub fn load_recipe(vault_root: &Path, recipe_id: &str) -> Result<Recipe> {
    seed_starter_recipes(vault_root)?;
    let path = recipes_dir(vault_root).join(format!("{recipe_id}.json"));
    if !path.is_file() {
        return Err(CoreError::Protocol(format!("recipe not found: {recipe_id}")));
    }
    read_recipe_file(&path)
}

pub fn load_recipe_json(vault_root: &Path, recipe_id: &str) -> Result<String> {
    let recipe = load_recipe(vault_root, recipe_id)?;
    Ok(serde_json::to_string_pretty(&recipe)?)
}

fn read_recipe_file(path: &Path) -> Result<Recipe> {
    let raw = fs::read_to_string(path)?;
    let recipe: Recipe = serde_json::from_str(&raw)?;
    recipe
        .validate()
        .map_err(|err| CoreError::Protocol(format!("invalid recipe {}: {err}", path.display())))?;
    Ok(recipe)
}

pub fn save_run(vault_root: &Path, run: &OrchestrationRunMeta) -> Result<()> {
    if run.schema_version != RUN_SCHEMA_VERSION {
        return Err(CoreError::UnsupportedSchemaVersion(run.schema_version));
    }
    let dir = run_dir(vault_root, &run.id);
    fs::create_dir_all(&dir)?;
    let tmp = dir.join("meta.json.tmp");
    let final_path = dir.join("meta.json");
    fs::write(&tmp, serde_json::to_string_pretty(run)?)?;
    fs::rename(tmp, final_path)?;
    Ok(())
}

pub fn load_run(vault_root: &Path, run_id: &str) -> Result<OrchestrationRunMeta> {
    let path = run_dir(vault_root, run_id).join("meta.json");
    if !path.is_file() {
        return Err(CoreError::Protocol(format!("orchestration run not found: {run_id}")));
    }
    let raw = fs::read_to_string(path)?;
    let run: OrchestrationRunMeta = serde_json::from_str(&raw)?;
    if run.schema_version != RUN_SCHEMA_VERSION {
        return Err(CoreError::UnsupportedSchemaVersion(run.schema_version));
    }
    Ok(run)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn seed_and_list_starter_recipes() {
        let dir = tempdir().unwrap();
        seed_starter_recipes(dir.path()).unwrap();
        let recipes = list_recipes(dir.path()).unwrap();
        assert!(recipes.iter().any(|r| r.id == "handoff"));
        assert!(recipes.iter().any(|r| r.id == "committee"));
    }
}
