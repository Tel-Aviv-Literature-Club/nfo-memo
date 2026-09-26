use anyhow::{Context, Result};
use directories::ProjectDirs;
use std::{fs, path::PathBuf};

fn settings_path() -> Option<PathBuf> {
    ProjectDirs::from("dev", "NFO Memo", "NFO Memo")
        .map(|dirs| dirs.config_dir().join("settings.conf"))
}

pub fn load_default_gpg_key() -> Option<String> {
    let path = settings_path()?;
    fs::read_to_string(path)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

pub fn save_default_gpg_key(fingerprint: &str) -> Result<()> {
    let path = settings_path().context("could not determine the configuration directory")?;
    let parent = path.parent().context("invalid configuration path")?;
    fs::create_dir_all(parent).with_context(|| format!("could not create {}", parent.display()))?;
    fs::write(&path, format!("{fingerprint}\n"))
        .with_context(|| format!("could not write {}", path.display()))
}
