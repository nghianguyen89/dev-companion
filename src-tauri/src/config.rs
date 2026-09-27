use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};
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
pub enum Language { En, Vi }

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

fn config_file() -> PathBuf {
    crate::platform::portable_root().unwrap_or_else(crate::platform::app_data_dir)
        .join("config")
        .join("settings.json")
}
pub fn load() -> Result<AppConfiguration, ConfigurationError> {
    let path = config_file();
    if !path.exists() {
        return Ok(AppConfiguration::default());
    }
    Ok(serde_json::from_slice(&fs::read(path)?)?)
}
pub fn save(configuration: &AppConfiguration) -> Result<(), ConfigurationError> {
    let path = config_file();
    let directory = path.parent().expect("settings parent");
    fs::create_dir_all(&directory)?;
    let content = serde_json::to_vec_pretty(configuration)?;
    fs::write(path, content)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
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
}
