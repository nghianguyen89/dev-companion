use std::path::PathBuf;

use crate::config::AppConfiguration;

pub fn operating_system() -> &'static str {
    match std::env::consts::OS {
        "windows" => "windows",
        "macos" => "macos",
        "linux" => "linux",
        _ => "unknown",
    }
}

pub fn codex_home() -> PathBuf {
    std::env::var_os("CODEX_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| dirs::home_dir().unwrap_or_default().join(".codex"))
}

pub fn app_data_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| dirs::home_dir().unwrap_or_default().join(".config"))
        .join("codex-companion")
}

pub fn portable_root() -> Option<PathBuf> {
    let executable = std::env::current_exe().ok()?;
    let root = executable.parent()?.to_path_buf();
    root.join("portable-mode").is_file().then_some(root)
}

pub fn config_dir(configuration: &AppConfiguration) -> PathBuf {
    if configuration.portable_mode {
        if let Some(root) = portable_root() {
            return root.join("config");
        }
    }
    app_data_dir().join("config")
}

pub fn backup_dir(configuration: &AppConfiguration) -> PathBuf {
    if configuration.portable_mode {
        if let Some(root) = portable_root() {
            return root.join("backups");
        }
    }
    app_data_dir().join("backups")
}

fn personal_bundle_dir_at(portable: Option<PathBuf>) -> PathBuf {
    portable
        .map(|root| root.join("backup"))
        .unwrap_or_else(|| app_data_dir().join("personal-bundles"))
}

/// Personal bundles stay beside the portable executable when its marker is present.
pub fn personal_bundle_dir() -> PathBuf {
    personal_bundle_dir_at(portable_root())
}
pub fn personal_staging_dir() -> PathBuf {
    app_data_dir().join("staging")
}

/// Safety archives for local legacy-session deletion live in Companion data, never in
/// CODEX_HOME. They are intentionally retained until the user removes them manually.
pub fn delete_quarantine_dir(configuration: &AppConfiguration) -> PathBuf {
    if configuration.portable_mode {
        if let Some(root) = portable_root() {
            return root.join("quarantine");
        }
    }
    app_data_dir().join("quarantine")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn portable_personal_bundles_stay_with_the_executable() {
        assert_eq!(
            personal_bundle_dir_at(Some(PathBuf::from(r"D:\Portable\Dev Companion"))),
            PathBuf::from(r"D:\Portable\Dev Companion\backup")
        );
    }
}
