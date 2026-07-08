use std::fs;
use std::path::{Path, PathBuf};

use anyhow::bail;
use anyhow::{Context as _, Result};
use serde::{Deserialize, Serialize};

use crate::models::today_key;
use crate::repository::Repository;
use crate::root;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub last_rollover_date: String,
    #[serde(default)]
    pub active_session: Option<String>,
}

pub fn path(root: &Path) -> PathBuf {
    root::backburner_dir(root).join("settings.json")
}

pub fn init(root: &Path) -> Result<()> {
    save(
        root,
        &Settings {
            last_rollover_date: today_key(),
            active_session: None,
        },
    )
}

pub fn rollover_if_needed(root: &Path, repository: &mut Repository) -> Result<()> {
    let today = today_key();
    let mut settings = load_or_init(root)?;
    if settings.last_rollover_date < today {
        repository.finish_session()?;
        settings.last_rollover_date = today;
        save(root, &settings)?;
    }
    Ok(())
}

pub fn active_session(root: &Path) -> Result<Option<String>> {
    Ok(load_or_init(root)?.active_session)
}

pub fn start_session(root: &Path, name: &str) -> Result<String> {
    let name = normalize_session_name(name)?;
    let mut settings = load_or_init(root)?;
    settings.active_session = Some(name.clone());
    save(root, &settings)?;
    Ok(name)
}

pub fn end_session(root: &Path) -> Result<Option<String>> {
    let mut settings = load_or_init(root)?;
    let previous = settings.active_session.take();
    save(root, &settings)?;
    Ok(previous)
}

fn load_or_init(root: &Path) -> Result<Settings> {
    let settings_path = path(root);
    if !settings_path.exists() {
        let settings = Settings {
            last_rollover_date: today_key(),
            active_session: None,
        };
        save(root, &settings)?;
        return Ok(settings);
    }

    let contents = fs::read_to_string(&settings_path)
        .with_context(|| format!("could not read {}", settings_path.display()))?;
    let settings = serde_json::from_str(&contents)
        .with_context(|| format!("could not parse {}", settings_path.display()))?;
    Ok(settings)
}

fn normalize_session_name(name: &str) -> Result<String> {
    let name = name.trim();
    if name.is_empty() {
        bail!("session name cannot be empty");
    }
    Ok(name.to_string())
}

fn save(root: &Path, settings: &Settings) -> Result<()> {
    let settings_path = path(root);
    let contents = serde_json::to_string_pretty(settings)?;
    fs::write(&settings_path, format!("{contents}\n"))
        .with_context(|| format!("could not write {}", settings_path.display()))?;
    Ok(())
}
