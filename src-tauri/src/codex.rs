use crate::{config::AppConfiguration, platform};
use serde::Serialize;
use std::{
    cmp::Reverse,
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::UNIX_EPOCH,
};

const RECENT_BACKUP_FILES: usize = 8;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexPaths {
    pub codex_home: String,
    pub config_dir: String,
    pub backup_dir: String,
    pub portable_mode: bool,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticsSnapshot {
    pub operating_system: String,
    pub architecture: String,
    pub codex_home: String,
    pub codex_home_exists: bool,
    pub config_dir: String,
    pub backup_dir: String,
    pub codex_cli_version: Option<String>,
    pub skills_count: usize,
    pub pets_count: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupStorageEntry {
    pub name: String,
    pub bytes: u64,
    pub modified_at: Option<u64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupStorageSummary {
    pub directory: String,
    pub file_count: usize,
    pub total_bytes: u64,
    pub recent_files: Vec<BackupStorageEntry>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupStorageOverview {
    pub session_backups: BackupStorageSummary,
    pub personal_bundles: BackupStorageSummary,
}

pub fn paths(configuration: &AppConfiguration) -> CodexPaths {
    CodexPaths {
        codex_home: display(platform::codex_home()),
        config_dir: display(platform::config_dir(configuration)),
        backup_dir: display(platform::backup_dir(configuration)),
        portable_mode: configuration.portable_mode && platform::portable_root().is_some(),
    }
}
pub fn diagnostics(configuration: &AppConfiguration) -> DiagnosticsSnapshot {
    let paths = paths(configuration);
    let home = platform::codex_home();
    DiagnosticsSnapshot {
        operating_system: platform::operating_system().to_string(),
        architecture: std::env::consts::ARCH.to_string(),
        codex_home: paths.codex_home,
        codex_home_exists: home.is_dir(),
        config_dir: paths.config_dir,
        backup_dir: paths.backup_dir,
        codex_cli_version: cli_version(),
        skills_count: directory_entry_count(&home.join("skills")),
        pets_count: directory_entry_count(&home.join("pets")),
    }
}
pub fn backup_storage(configuration: &AppConfiguration) -> BackupStorageOverview {
    BackupStorageOverview {
        session_backups: summarize_backup_directory(platform::backup_dir(configuration)),
        personal_bundles: summarize_backup_directory(platform::personal_bundle_dir()),
    }
}

fn summarize_backup_directory(directory: PathBuf) -> BackupStorageSummary {
    let mut files = Vec::new();
    let mut total_bytes: u64 = 0;
    if let Ok(entries) = fs::read_dir(&directory) {
        for entry in entries.flatten() {
            let Ok(metadata) = entry.metadata() else {
                continue;
            };
            if !metadata.is_file() {
                continue;
            }
            total_bytes = total_bytes.saturating_add(metadata.len());
            files.push(BackupStorageEntry {
                name: entry.file_name().to_string_lossy().into_owned(),
                bytes: metadata.len(),
                modified_at: metadata
                    .modified()
                    .ok()
                    .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                    .map(|duration| duration.as_secs()),
            });
        }
    }
    files.sort_by_key(|file| Reverse(file.modified_at.unwrap_or_default()));
    let file_count = files.len();
    files.truncate(RECENT_BACKUP_FILES);
    BackupStorageSummary {
        directory: display(directory),
        file_count,
        total_bytes,
        recent_files: files,
    }
}
fn display(path: impl AsRef<Path>) -> String {
    path.as_ref().to_string_lossy().to_string()
}
fn directory_entry_count(path: &Path) -> usize {
    fs::read_dir(path)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .filter(|entry| entry.path().is_dir())
                .count()
        })
        .unwrap_or(0)
}
pub(crate) fn cli_version() -> Option<String> {
    Command::new("codex")
        .arg("--version")
        .output()
        .ok()
        .filter(|result| result.status.success())
        .map(|result| String::from_utf8_lossy(&result.stdout).trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};
    #[test]
    fn missing_directory_has_zero_entries() {
        assert_eq!(
            directory_entry_count(Path::new("not-a-real-codex-companion-path")),
            0
        );
    }

    #[test]
    fn backup_storage_lists_only_direct_regular_files() {
        let root = std::env::temp_dir().join(format!(
            "dev-companion-backup-storage-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(root.join("nested")).unwrap();
        fs::write(root.join("one.zip"), b"one").unwrap();
        fs::write(root.join("two.zip"), b"two!").unwrap();

        let summary = summarize_backup_directory(root.clone());

        assert_eq!(summary.file_count, 2);
        assert_eq!(summary.total_bytes, 7);
        assert_eq!(summary.recent_files.len(), 2);
        let _ = fs::remove_dir_all(root);
    }
}
