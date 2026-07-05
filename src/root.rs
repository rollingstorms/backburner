use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

pub fn find_git_root(start: &Path) -> Result<PathBuf> {
    let mut current = start
        .canonicalize()
        .with_context(|| format!("could not read {}", start.display()))?;
    loop {
        if current.join(".git").exists() {
            return Ok(current);
        }
        if !current.pop() {
            bail!("not inside a git repository");
        }
    }
}

pub fn backburner_dir(root: &Path) -> PathBuf {
    root.join(".backburner")
}

pub fn database_path(root: &Path) -> PathBuf {
    backburner_dir(root).join("backburner.db")
}

pub fn ensure_initialized(root: &Path) -> Result<PathBuf> {
    let db_path = database_path(root);
    if !db_path.exists() {
        bail!("backburner is not initialized here; run `bb init` first");
    }
    Ok(db_path)
}

pub fn init_project(root: &Path) -> Result<PathBuf> {
    let dir = backburner_dir(root);
    fs::create_dir_all(&dir).with_context(|| format!("could not create {}", dir.display()))?;

    let info_exclude = root.join(".git").join("info").join("exclude");
    if info_exclude.exists() {
        let current = fs::read_to_string(&info_exclude).unwrap_or_default();
        if !current.lines().any(|line| line.trim() == ".backburner/") {
            let prefix = if current.is_empty() || current.ends_with('\n') {
                ""
            } else {
                "\n"
            };
            fs::write(&info_exclude, format!("{current}{prefix}.backburner/\n"))
                .with_context(|| format!("could not update {}", info_exclude.display()))?;
        }
    }

    Ok(database_path(root))
}
