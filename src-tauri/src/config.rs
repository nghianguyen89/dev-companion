use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};
use thiserror::Error;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfiguration {
    pub theme: Theme,
    pub portable_mode: bool,
    pub create_safety_backups: bool,
    pub language: Language,
    pub log_level: LogLevel,
    #[serde(default)]
    pub codex_environments: Vec<crate::codex_environment::Environment>,
    #[serde(default)]
    pub file_transfer_profiles: Vec<crate::file_transfer::TransferProfile>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    System,
    Light,
    Dark,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    En,
    Vi,
}

impl Default for AppConfiguration {
    fn default() -> Self {
        Self {
            theme: Theme::System,
            portable_mode: crate::platform::portable_root().is_some(),
            create_safety_backups: true,
            language: Language::En,
            log_level: LogLevel::Warn,
            codex_environments: Vec::new(),
            file_transfer_profiles: Vec::new(),
        }
    }
}

#[derive(Debug, Error)]
pub enum ConfigurationError {
    #[error("Unable to read configuration: {0}")]
    Read(#[from] std::io::Error),
    #[error("Configuration file is invalid: {0}")]
    Parse(#[from] serde_json::Error),
}

fn config_file() -> Result<PathBuf, ConfigurationError> {
    let root = crate::platform::portable_root().unwrap_or_else(crate::platform::app_data_dir);
    migrate_legacy_config(&root)?;
    Ok(root.join("configs").join("settings.json"))
}

fn migrate_legacy_config(root: &Path) -> std::io::Result<()> {
    let legacy = root.join("config");
    let target = root.join("configs");
    crate::fs_safety::check(&legacy).map_err(std::io::Error::other)?;
    crate::fs_safety::check(&target).map_err(std::io::Error::other)?;
    if !legacy.exists() {
        return Ok(());
    }
    let entries = fs::read_dir(&legacy)?.collect::<Result<Vec<_>, _>>()?;
    // Inspect the whole source before any move; never follow links or overwrite.
    fn check_tree(path: &Path) -> std::io::Result<()> {
        crate::fs_safety::check(path).map_err(std::io::Error::other)?;
        let metadata = fs::symlink_metadata(path)?;
        if metadata.is_dir() {
            for entry in fs::read_dir(path)? {
                check_tree(&entry?.path())?;
            }
        } else if !metadata.is_file() {
            return Err(std::io::Error::other("Unsupported legacy config entry."));
        }
        Ok(())
    }
    for entry in &entries {
        check_tree(&entry.path())?;
        if target.join(entry.file_name()).try_exists()? {
            return Err(std::io::Error::other(
                "Config migration conflict: keep both folders and resolve duplicate entries before retrying.",
            ));
        }
    }
    fs::create_dir_all(&target)?;
    for entry in entries {
        let destination = target.join(entry.file_name());
        #[cfg(windows)]
        move_without_replace(&entry.path(), &destination)?;
        #[cfg(not(windows))]
        if entry.file_type()?.is_file() {
            // Creating a link fails if the destination appeared after preflight.
            fs::hard_link(entry.path(), &destination)?;
            fs::remove_file(entry.path())?;
        } else {
            fs::rename(entry.path(), destination)?;
        }
    }
    fs::remove_dir(&legacy)
}

#[cfg(windows)]
fn move_without_replace(source: &Path, destination: &Path) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    #[link(name = "kernel32")]
    extern "system" {
        fn MoveFileW(source: *const u16, destination: *const u16) -> i32;
    }
    let source: Vec<u16> = source.as_os_str().encode_wide().chain(Some(0)).collect();
    let destination: Vec<u16> = destination
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect();
    // Supports portable drives without hard links and refuses existing targets.
    if unsafe { MoveFileW(source.as_ptr(), destination.as_ptr()) } == 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}
pub fn load() -> Result<AppConfiguration, ConfigurationError> {
    let configuration = load_from(&config_file()?)?;
    migrate_data_config(&configuration)?;
    Ok(configuration)
}

fn migrate_data_config(configuration: &AppConfiguration) -> std::io::Result<()> {
    let directory = crate::platform::config_dir(configuration);
    migrate_legacy_config(directory.parent().expect("config storage root"))
}

fn load_from(path: &Path) -> Result<AppConfiguration, ConfigurationError> {
    if !path.exists() {
        return Ok(AppConfiguration::default());
    }
    Ok(serde_json::from_slice(&fs::read(path)?)?)
}

pub fn save(configuration: &AppConfiguration) -> Result<(), ConfigurationError> {
    migrate_data_config(configuration)?;
    save_to(&config_file()?, configuration)
}

fn save_to(path: &Path, configuration: &AppConfiguration) -> Result<(), ConfigurationError> {
    let directory = path.parent().expect("settings parent");
    fs::create_dir_all(directory)?;
    let content = serde_json::to_vec_pretty(configuration)?;
    let temporary = path.with_extension(format!("{}.tmp", std::process::id()));
    let result = (|| -> std::io::Result<()> {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        file.write_all(&content)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temporary, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result.map_err(ConfigurationError::Read)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("companion-config-{name}-{}", std::process::id()))
    }

    #[test]
    fn migrates_legacy_settings_history_and_logs_without_changing_xampp() {
        let root = fixture("migration");
        fs::create_dir_all(root.join("config/logs/file-transfer")).unwrap();
        fs::create_dir_all(root.join("configs/xampp")).unwrap();
        fs::write(root.join("config/settings.json"), b"settings").unwrap();
        fs::write(
            root.join("config/file-transfer-history-v1.json"),
            b"history",
        )
        .unwrap();
        fs::write(root.join("config/logs/file-transfer/old.log"), b"log").unwrap();
        fs::write(root.join("configs/xampp/settings.json"), b"domains").unwrap();
        migrate_legacy_config(&root).unwrap();
        migrate_legacy_config(&root).unwrap();
        assert!(!root.join("config").exists());
        for (path, bytes) in [
            ("settings.json", "settings"),
            ("file-transfer-history-v1.json", "history"),
            ("logs/file-transfer/old.log", "log"),
            ("xampp/settings.json", "domains"),
        ] {
            assert_eq!(
                fs::read(root.join("configs").join(path)).unwrap(),
                bytes.as_bytes()
            );
        }
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn migration_collision_preserves_both_folders_before_any_move() {
        let root = fixture("collision");
        fs::create_dir_all(root.join("config")).unwrap();
        fs::create_dir_all(root.join("configs")).unwrap();
        fs::write(root.join("config/settings.json"), b"old").unwrap();
        fs::write(root.join("config/history.json"), b"history").unwrap();
        fs::write(root.join("configs/settings.json"), b"new").unwrap();
        assert!(migrate_legacy_config(&root).is_err());
        assert_eq!(fs::read(root.join("config/settings.json")).unwrap(), b"old");
        assert_eq!(
            fs::read(root.join("configs/settings.json")).unwrap(),
            b"new"
        );
        assert!(root.join("config/history.json").is_file());
        assert!(!root.join("configs/history.json").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn migration_rejects_junction_without_moving_settings() {
        let root = fixture("junction");
        fs::create_dir_all(root.join("config")).unwrap();
        fs::create_dir_all(root.join("outside")).unwrap();
        fs::write(root.join("config/settings.json"), b"old").unwrap();
        assert!(std::process::Command::new("cmd.exe")
            .args(["/C", "mklink", "/J"])
            .arg(root.join("config").join("linked"))
            .arg(root.join("outside"))
            .output()
            .unwrap()
            .status
            .success());
        assert!(migrate_legacy_config(&root).is_err());
        assert!(root.join("config/settings.json").is_file());
        fs::remove_dir(root.join("config/linked")).unwrap();
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn default_configuration_is_safe() {
        let configuration = AppConfiguration::default();
        assert!(configuration.create_safety_backups);
        assert!(matches!(configuration.theme, Theme::System));
        assert!(matches!(configuration.language, Language::En));
    }

    #[test]
    fn language_is_persisted_in_the_configuration_dto() {
        let mut configuration = AppConfiguration::default();
        configuration.language = Language::Vi;
        let serialized = serde_json::to_value(configuration).unwrap();
        assert_eq!(serialized["language"], "vi");
        let restored: AppConfiguration = serde_json::from_value(serialized).unwrap();
        assert!(matches!(restored.language, Language::Vi));
    }

    #[test]
    fn save_replaces_the_configuration_with_a_readable_complete_file() {
        let root =
            std::env::temp_dir().join(format!("companion-config-test-{}", std::process::id()));
        let path = root.join("configs").join("settings.json");
        let mut configuration = AppConfiguration::default();
        save_to(&path, &configuration).unwrap();
        configuration.language = Language::Vi;
        save_to(&path, &configuration).unwrap();
        assert!(matches!(load_from(&path).unwrap().language, Language::Vi));
        assert!(serde_json::from_slice::<serde_json::Value>(&fs::read(&path).unwrap()).is_ok());
        fs::remove_dir_all(root).unwrap();
    }
}
