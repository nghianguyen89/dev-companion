use std::{fs, path::Path};

use serde::{Deserialize, Serialize};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

use crate::{config::AppConfiguration, platform};

pub const HISTORY_FORMAT_VERSION: u32 = 1;
const MAX_HISTORY_ENTRIES: usize = 100;

/// This is deliberately an audit summary, not an activity log. In particular, it never has a
/// source archive path, restore token, session contents, native error text, or credentials.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreHistoryEntry {
    pub occurred_at: String,
    #[serde(default)]
    pub action: HistoryAction,
    pub archive_name: String,
    pub session_ids: Vec<String>,
    #[serde(default)]
    pub deleted_count: usize,
    pub restored_count: usize,
    pub skipped_conflicts: usize,
    pub safety_backup_path: Option<String>,
    pub outcome: RestoreOutcome,
    pub error_code: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum HistoryAction {
    Restore,
    Delete,
}

impl Default for HistoryAction {
    fn default() -> Self {
        Self::Restore
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RestoreOutcome {
    Completed,
    Partial,
    RolledBack,
    Failed,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RestoreHistoryFile {
    format_version: u32,
    entries: Vec<RestoreHistoryEntry>,
}

pub fn record(
    configuration: &AppConfiguration,
    archive_name: String,
    session_ids: Vec<String>,
    restored_count: usize,
    skipped_conflicts: usize,
    safety_backup_path: Option<String>,
    outcome: RestoreOutcome,
    error_code: Option<String>,
) -> Result<(), String> {
    record_operation(
        configuration,
        HistoryAction::Restore,
        archive_name,
        session_ids,
        0,
        restored_count,
        skipped_conflicts,
        safety_backup_path,
        outcome,
        error_code,
    )
}

pub fn record_delete(
    configuration: &AppConfiguration,
    session_ids: Vec<String>,
    deleted_count: usize,
    restored_count: usize,
    skipped_count: usize,
    safety_backup_path: Option<String>,
    outcome: RestoreOutcome,
    error_code: Option<String>,
) -> Result<(), String> {
    record_operation(
        configuration,
        HistoryAction::Delete,
        "Local delete safety archive".into(),
        session_ids,
        deleted_count,
        restored_count,
        skipped_count,
        safety_backup_path,
        outcome,
        error_code,
    )
}

fn record_operation(
    configuration: &AppConfiguration,
    action: HistoryAction,
    archive_name: String,
    session_ids: Vec<String>,
    deleted_count: usize,
    restored_count: usize,
    skipped_conflicts: usize,
    safety_backup_path: Option<String>,
    outcome: RestoreOutcome,
    error_code: Option<String>,
) -> Result<(), String> {
    record_at_path(
        &platform::restore_history_file(configuration),
        action,
        archive_name,
        session_ids,
        deleted_count,
        restored_count,
        skipped_conflicts,
        safety_backup_path,
        outcome,
        error_code,
    )
}

fn record_at_path(
    path: &Path,
    action: HistoryAction,
    archive_name: String,
    session_ids: Vec<String>,
    deleted_count: usize,
    restored_count: usize,
    skipped_conflicts: usize,
    safety_backup_path: Option<String>,
    outcome: RestoreOutcome,
    error_code: Option<String>,
) -> Result<(), String> {
    let occurred_at = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .map_err(|_| "Unable to timestamp restore history.")?;
    let mut entries = read_entries(path)?;
    entries.insert(
        0,
        RestoreHistoryEntry {
            occurred_at,
            action,
            archive_name: safe_archive_name(&archive_name),
            session_ids,
            deleted_count,
            restored_count,
            skipped_conflicts,
            safety_backup_path,
            outcome,
            error_code,
        },
    );
    entries.truncate(MAX_HISTORY_ENTRIES);
    write_entries(path, &entries)
}

pub fn list(configuration: &AppConfiguration) -> Result<Vec<RestoreHistoryEntry>, String> {
    read_entries(&platform::restore_history_file(configuration))
}

fn safe_archive_name(value: &str) -> String {
    Path::new(value)
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("Selected archive")
        .to_owned()
}

fn read_entries(path: &Path) -> Result<Vec<RestoreHistoryEntry>, String> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let metadata = fs::symlink_metadata(path).map_err(|_| "Restore history is unavailable.")?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err("Restore history is unavailable.".into());
    }
    let history: RestoreHistoryFile =
        serde_json::from_slice(&fs::read(path).map_err(|_| "Restore history is unavailable.")?)
            .map_err(|_| "Restore history is unavailable.")?;
    if history.format_version != HISTORY_FORMAT_VERSION {
        return Err("Restore history uses an unsupported format.".into());
    }
    Ok(history
        .entries
        .into_iter()
        .take(MAX_HISTORY_ENTRIES)
        .collect())
}

fn write_entries(path: &Path, entries: &[RestoreHistoryEntry]) -> Result<(), String> {
    let parent = path.parent().ok_or("Restore history is unavailable.")?;
    fs::create_dir_all(parent).map_err(|_| "Restore history is unavailable.")?;
    if path.exists() {
        let metadata = fs::symlink_metadata(path).map_err(|_| "Restore history is unavailable.")?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err("Restore history is unavailable.".into());
        }
    }
    let document = RestoreHistoryFile {
        format_version: HISTORY_FORMAT_VERSION,
        entries: entries.to_vec(),
    };
    fs::write(
        path,
        serde_json::to_vec_pretty(&document).map_err(|_| "Restore history is unavailable.")?,
    )
    .map_err(|_| "Restore history is unavailable.".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn history_keeps_a_bounded_sanitized_audit_summary() {
        let root = std::env::temp_dir().join(format!(
            "codex-companion-history-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        let path = root.join("history.json");
        for number in 0..101 {
            record_at_path(
                &path,
                HistoryAction::Restore,
                format!("C:/private/source/archive-{number}.zip"),
                vec![format!("session-{number}")],
                0,
                1,
                0,
                None,
                RestoreOutcome::Completed,
                None,
            )
            .unwrap();
        }
        let loaded = read_entries(&path).unwrap();
        assert_eq!(loaded.len(), MAX_HISTORY_ENTRIES);
        assert_eq!(loaded[0].archive_name, "archive-100.zip");
        assert_eq!(
            safe_archive_name("C:/private/source/archive.zip"),
            "archive.zip"
        );
        let serialized = fs::read_to_string(&path).unwrap();
        assert!(!serialized.contains("C:/private/source"));
        assert!(serialized.contains("session-100"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn corrupt_or_future_history_is_not_silently_interpreted() {
        let path = std::env::temp_dir().join(format!(
            "codex-companion-history-invalid-{}.json",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::write(&path, r#"{"formatVersion":2,"entries":[]}"#).unwrap();
        assert!(read_entries(&path).unwrap_err().contains("unsupported"));
        let _ = fs::remove_file(path);
    }
}
