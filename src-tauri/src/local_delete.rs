use std::{collections::HashSet, fs::{self, File, OpenOptions}, io::{Read, Write}, path::{Path, PathBuf}};

use serde::{Deserialize, Serialize};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};
use zip::{write::SimpleFileOptions, CompressionMethod, ZipArchive, ZipWriter};

use crate::{
    backup::BackupSession,
    config::AppConfiguration,
    platform,
    restore_history::{self, RestoreOutcome},
    session_storage::{self, SelectedSession},
};

pub const DELETE_FORMAT_VERSION: u32 = 1;
const DELETE_MANIFEST_NAME: &str = "delete-manifest.json";
const CONFIRMATION: &str = "DELETE";

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeletePreview {
    pub format_version: u32,
    pub session_count: usize,
    pub total_bytes: u64,
    pub quarantine_directory: String,
    pub sessions: Vec<BackupSession>,
    pub local_only: bool,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DeleteResult {
    pub outcome: DeleteOutcome,
    pub deleted_count: usize,
    pub restored_count: usize,
    pub skipped_count: usize,
    pub total_bytes: u64,
    pub safety_archive_path: Option<String>,
}

#[derive(Debug, Serialize, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)] // Reserved for a future explicitly user-approved partial-delete policy.
#[serde(rename_all = "camelCase")]
pub enum DeleteOutcome { Completed, Partial, RolledBack, Failed }

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DeleteManifest {
    format_version: u32,
    created_at: String,
    kind: &'static str,
    sessions: Vec<DeleteManifestSession>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ReadDeleteManifest {
    format_version: u32,
    created_at: String,
    kind: String,
    sessions: Vec<DeleteManifestSession>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DeleteManifestSession { id: String, archive_path: String, bytes: u64 }

#[derive(Clone)]
struct ArchivedSession { selected: SelectedSession, bytes: Vec<u8> }

struct DeleteFailure { code: &'static str, message: &'static str, result: DeleteResult }

pub fn preview(configuration: &AppConfiguration, ids: &[String]) -> Result<DeletePreview, String> {
    let selected = session_storage::selected_sessions(&platform::codex_home(), ids)?;
    let (sessions, total_bytes) = describe(&selected)?;
    Ok(DeletePreview {
        format_version: DELETE_FORMAT_VERSION,
        session_count: sessions.len(),
        total_bytes,
        quarantine_directory: display(platform::delete_quarantine_dir(configuration)),
        sessions,
        local_only: true,
    })
}

/// Performs a legacy-local deletion only after a literal confirmation. IDs are re-resolved at
/// execution time; paths from the frontend are never accepted.
pub fn execute(configuration: &AppConfiguration, ids: &[String], confirmation: &str) -> Result<DeleteResult, String> {
    let home = platform::codex_home();
    match execute_at(configuration, &home, ids, confirmation, None, None) {
        Ok(result) => {
            record(configuration, ids, &result, None);
            Ok(result)
        }
        Err(failure) => {
            record(configuration, ids, &failure.result, Some(failure.code));
            if failure.result.safety_archive_path.is_some() { Ok(failure.result) } else { Err(format!("{}: {}", failure.code, failure.message)) }
        }
    }
}

fn execute_at(
    configuration: &AppConfiguration,
    home: &Path,
    ids: &[String],
    confirmation: &str,
    fail_after: Option<usize>,
    quarantine_override: Option<&Path>,
) -> Result<DeleteResult, DeleteFailure> {
    let empty = || DeleteResult { outcome: DeleteOutcome::Failed, deleted_count: 0, restored_count: 0, skipped_count: 0, total_bytes: 0, safety_archive_path: None };
    if confirmation != CONFIRMATION {
        return Err(DeleteFailure { code: "deletion_confirmation_required", message: "Explicit DELETE confirmation is required.", result: empty() });
    }
    let selected = session_storage::selected_sessions(home, ids)
        .map_err(|_| DeleteFailure { code: "deletion_validation_failed", message: "Selected local legacy sessions are unavailable or unsupported.", result: empty() })?;
    let root = session_storage::canonical_sessions_root(home)
        .map_err(|_| DeleteFailure { code: "deletion_validation_failed", message: "The local legacy sessions directory is unavailable.", result: empty() })?;
    let archived = snapshot_selected(&selected, &root)
        .map_err(|_| DeleteFailure { code: "deletion_validation_failed", message: "Selected local legacy sessions changed before deletion.", result: empty() })?;
    let total_bytes = archived.iter().map(|item| item.bytes.len() as u64).sum();
    let quarantine_directory = quarantine_override.map(Path::to_path_buf).unwrap_or_else(|| platform::delete_quarantine_dir(configuration));
    let archive_path = create_safety_archive(&quarantine_directory, &archived, &root)
        .map_err(|_| DeleteFailure { code: "deletion_archive_failed", message: "The safety archive could not be created and verified; nothing was deleted.", result: empty() })?;
    let safety_archive_path = Some(display(&archive_path));

    let mut deleted = Vec::new();
    for (index, item) in archived.iter().enumerate() {
        if fail_after == Some(index)
            || session_storage::verify_regular_file_within(&item.selected.source_path, &root).is_err()
            || fs::read(&item.selected.source_path).map_or(true, |bytes| bytes != item.bytes)
            || fs::remove_file(&item.selected.source_path).is_err()
        {
            let restored = restore_deleted(&root, &archive_path, &deleted).unwrap_or(0);
            let all_restored = restored == deleted.len();
            return Err(DeleteFailure {
                code: if all_restored { "deletion_rolled_back" } else { "deletion_rollback_failed" },
                message: if all_restored { "Deletion stopped and deleted files were restored from the safety archive." } else { "Deletion stopped and could not fully restore deleted files." },
                result: DeleteResult {
                    outcome: if all_restored { DeleteOutcome::RolledBack } else { DeleteOutcome::Failed },
                    deleted_count: deleted.len(), restored_count: restored, skipped_count: 0, total_bytes,
                    safety_archive_path,
                },
            });
        }
        deleted.push(item.clone());
    }
    Ok(DeleteResult { outcome: DeleteOutcome::Completed, deleted_count: deleted.len(), restored_count: 0, skipped_count: 0, total_bytes, safety_archive_path })
}

fn record(configuration: &AppConfiguration, ids: &[String], result: &DeleteResult, code: Option<&str>) {
    let outcome = match result.outcome {
        DeleteOutcome::Completed => RestoreOutcome::Completed,
        DeleteOutcome::Partial => RestoreOutcome::Partial,
        DeleteOutcome::RolledBack => RestoreOutcome::RolledBack,
        DeleteOutcome::Failed => RestoreOutcome::Failed,
    };
    // Failed requests that never passed the resolver are not persisted: this keeps arbitrary
    // frontend-supplied IDs out of the local audit file.
    if result.safety_archive_path.is_none() && result.deleted_count == 0 { return; }
    if let Err(error) = restore_history::record_delete(configuration, ids.to_vec(), result.deleted_count, result.restored_count, result.skipped_count, result.safety_archive_path.clone(), outcome, code.map(str::to_owned)) {
        tracing::warn!("delete history could not be recorded: {error}");
    }
}

fn describe(selected: &[SelectedSession]) -> Result<(Vec<BackupSession>, u64), String> {
    let root = session_storage::canonical_sessions_root(&platform::codex_home())?;
    let mut total = 0_u64;
    let sessions = selected.iter().map(|item| {
        session_storage::verify_regular_file_within(&item.source_path, &root)?;
        let bytes = fs::metadata(&item.source_path).map_err(|_| "Selected conversation cannot be read.")?.len();
        total = total.checked_add(bytes).ok_or("Selected files are too large.")?;
        Ok(BackupSession { id: item.summary.id.clone(), title: item.summary.title.clone(), created_at: item.summary.created_at.clone(), updated_at: item.summary.updated_at.clone(), archive_path: item.archive_path.clone(), bytes })
    }).collect::<Result<Vec<_>, String>>()?;
    Ok((sessions, total))
}

fn snapshot_selected(selected: &[SelectedSession], root: &Path) -> Result<Vec<ArchivedSession>, String> {
    selected.iter().map(|item| {
        session_storage::verify_regular_file_within(&item.source_path, root)?;
        let bytes = fs::read(&item.source_path).map_err(|_| "Selected session is unavailable.")?;
        session_storage::validate_restorable_session_metadata(&bytes, &item.summary.id)?;
        Ok(ArchivedSession { selected: item.clone(), bytes })
    }).collect()
}

fn create_safety_archive(directory: &Path, sessions: &[ArchivedSession], root: &Path) -> Result<PathBuf, String> {
    fs::create_dir_all(directory).map_err(|_| "Quarantine is unavailable.")?;
    let metadata = fs::symlink_metadata(directory).map_err(|_| "Quarantine is unavailable.")?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() { return Err("Quarantine is unsafe.".into()); }
    let path = create_quarantine_path(directory)?;
    let file = OpenOptions::new().write(true).create_new(true).open(&path).map_err(|_| "Could not reserve a new safety archive.")?;
    let created_at = OffsetDateTime::now_utc().format(&Rfc3339).map_err(|_| "Could not timestamp safety archive.")?;
    let manifest = DeleteManifest {
        format_version: DELETE_FORMAT_VERSION, created_at, kind: "codex-companion-local-delete-safety-archive",
        sessions: sessions.iter().map(|item| DeleteManifestSession { id: item.selected.summary.id.clone(), archive_path: item.selected.archive_path.clone(), bytes: item.bytes.len() as u64 }).collect(),
    };
    let result = (|| -> Result<(), String> {
        let mut archive = ZipWriter::new(file);
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated).unix_permissions(0o600);
        archive.start_file(DELETE_MANIFEST_NAME, options).map_err(|_| "Could not write safety archive.")?;
        archive.write_all(&serde_json::to_vec_pretty(&manifest).map_err(|_| "Could not write safety archive.")?).map_err(|_| "Could not write safety archive.")?;
        for item in sessions {
            session_storage::verify_regular_file_within(&item.selected.source_path, root)?;
            // The archive must represent the exact bytes about to become eligible for removal;
            // a changed regular file is rejected just as a symlink would be.
            if fs::read(&item.selected.source_path).map_err(|_| "Selected session is unavailable.")? != item.bytes {
                return Err("Selected session changed before safety archiving.".into());
            }
            archive.start_file(&item.selected.archive_path, options).map_err(|_| "Could not write safety archive.")?;
            archive.write_all(&item.bytes).map_err(|_| "Could not write safety archive.")?;
        }
        archive.finish().map_err(|_| "Could not finish safety archive.")?.sync_all().map_err(|_| "Could not finish safety archive.")?;
        Ok(())
    })();
    let result = result.and_then(|_| verify_safety_archive(&path, sessions));
    if result.is_err() { let _ = fs::remove_file(&path); }
    result?;
    Ok(path)
}

fn verify_safety_archive(path: &Path, expected: &[ArchivedSession]) -> Result<(), String> {
    let file = File::open(path).map_err(|_| "Safety archive is unavailable.")?;
    let mut archive = ZipArchive::new(file).map_err(|_| "Safety archive is invalid.")?;
    let mut names = HashSet::new();
    for index in 0..archive.len() {
        let entry = archive.by_index(index).map_err(|_| "Safety archive is invalid.")?;
        if entry.is_dir() || !names.insert(entry.name().to_owned()) { return Err("Safety archive is invalid.".into()); }
    }
    let mut manifest_bytes = Vec::new();
    archive.by_name(DELETE_MANIFEST_NAME).map_err(|_| "Safety archive is invalid.")?.read_to_end(&mut manifest_bytes).map_err(|_| "Safety archive is invalid.")?;
    let manifest: ReadDeleteManifest = serde_json::from_slice(&manifest_bytes).map_err(|_| "Safety archive is invalid.")?;
    if manifest.format_version != DELETE_FORMAT_VERSION || manifest.kind != "codex-companion-local-delete-safety-archive" || OffsetDateTime::parse(&manifest.created_at, &Rfc3339).is_err() || manifest.sessions.len() != expected.len() { return Err("Safety archive is invalid.".into()); }
    let expected_names: HashSet<String> = expected.iter().map(|item| item.selected.archive_path.clone()).collect();
    if names.len() != expected_names.len() + 1 || !names.contains(DELETE_MANIFEST_NAME) { return Err("Safety archive is invalid.".into()); }
    for item in expected {
        let listed = manifest.sessions.iter().find(|session| session.id == item.selected.summary.id && session.archive_path == item.selected.archive_path).ok_or("Safety archive is invalid.")?;
        if listed.bytes != item.bytes.len() as u64 || !expected_names.contains(&listed.archive_path) { return Err("Safety archive is invalid.".into()); }
        let mut bytes = Vec::new();
        archive.by_name(&item.selected.archive_path).map_err(|_| "Safety archive is invalid.")?.read_to_end(&mut bytes).map_err(|_| "Safety archive is invalid.")?;
        if bytes != item.bytes || session_storage::validate_restorable_session_metadata(&bytes, &item.selected.summary.id).is_err() { return Err("Safety archive is invalid.".into()); }
    }
    Ok(())
}

fn restore_deleted(root: &Path, archive_path: &Path, deleted: &[ArchivedSession]) -> Result<usize, String> {
    let file = File::open(archive_path).map_err(|_| "Safety archive is unavailable.")?;
    let mut archive = ZipArchive::new(file).map_err(|_| "Safety archive is invalid.")?;
    let mut restored = 0;
    for item in deleted.iter().rev() {
        let attempt = (|| -> Result<(), String> {
            let target = restore_target(root, &item.selected.archive_path)?;
            crate::fs_safety::check(&target)?;
            create_safe_parent(root, &target)?;
            let mut bytes = Vec::new();
            archive.by_name(&item.selected.archive_path).map_err(|e| e.to_string())?.read_to_end(&mut bytes).map_err(|e| e.to_string())?;
            if bytes != item.bytes { return Err("Recovery archive changed.".into()); }
            let mut output = OpenOptions::new().write(true).create_new(true).open(&target).map_err(|e| e.to_string())?;
            let result = output.write_all(&bytes).and_then(|_| output.sync_all());
            drop(output);
            if let Err(error) = result { let _ = fs::remove_file(&target); return Err(error.to_string()); }
            Ok(())
        })();
        if attempt.is_ok() { restored += 1; }
    }
    Ok(restored)
}

fn restore_target(root: &Path, archive_path: &str) -> Result<PathBuf, String> {
    let relative = Path::new(archive_path).strip_prefix("sessions").map_err(|_| "Safety archive is invalid.")?;
    if relative.components().any(|part| matches!(part, std::path::Component::ParentDir | std::path::Component::RootDir | std::path::Component::Prefix(_))) { return Err("Safety archive is invalid.".into()); }
    let target = root.join(relative);
    if !target.starts_with(root) { return Err("Safety archive is invalid.".into()); }
    Ok(target)
}

fn create_safe_parent(root: &Path, target: &Path) -> Result<(), String> {
    crate::fs_safety::check(target)?;
    let relative = target.parent().ok_or("Safety archive is invalid.")?.strip_prefix(root).map_err(|_| "Safety archive is invalid.")?;
    let mut current = root.to_path_buf();
    for part in relative.components() {
        current.push(part);
        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => return Err("Safety archive is invalid.".into()),
            Ok(_) => {},
            Err(_) => fs::create_dir(&current).map_err(|_| "Safety archive is invalid.")?,
        }
    }
    Ok(())
}

fn create_quarantine_path(directory: &Path) -> Result<PathBuf, String> {
    let format = time::format_description::parse_borrowed::<2>("[year]-[month]-[day]_[hour][minute][second]").map_err(|_| "Could not name safety archive.")?;
    let stamp = OffsetDateTime::now_utc().format(&format).map_err(|_| "Could not name safety archive.")?;
    for suffix in 0..10_000_u32 {
        let name = if suffix == 0 { format!("codex-delete-safety-{stamp}.zip") } else { format!("codex-delete-safety-{stamp}-{suffix}.zip") };
        let candidate = directory.join(name);
        if !candidate.exists() { return Ok(candidate); }
    }
    Err("Could not name safety archive.".into())
}

fn display(path: impl AsRef<Path>) -> String { path.as_ref().to_string_lossy().to_string() }

#[cfg(test)]
mod tests {
    use super::*;

    fn root(name: &str) -> PathBuf { std::env::temp_dir().join(format!("codex-companion-delete-{name}-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos())) }
    fn config() -> AppConfiguration { AppConfiguration { theme: crate::config::Theme::System, portable_mode: false, create_safety_backups: true, language: crate::config::Language::En, log_level: crate::config::LogLevel::Info, codex_environments: Vec::new(), file_transfer_profiles: Vec::new() } }
    fn session(home: &Path, name: &str, id: &str) -> PathBuf { let path = home.join("sessions/2026/09/04").join(name); fs::create_dir_all(path.parent().unwrap()).unwrap(); fs::write(&path, format!("{{\"type\":\"session_meta\",\"payload\":{{\"session_id\":\"{id}\"}}}}\n{{\"type\":\"event_msg\",\"payload\":{{\"ignored\":true}}}}" )).unwrap(); path }

    #[test]
    fn creates_versioned_archive_with_exact_selected_files_before_deleting() {
        let home = root("archive"); let first = session(&home, "one.jsonl", "one"); let second = session(&home, "two.jsonl", "two");
        let quarantine = home.join("quarantine");
        let selected = session_storage::selected_sessions(&home, &["one".into()]).unwrap(); let sessions_root = session_storage::canonical_sessions_root(&home).unwrap(); let snapshot = snapshot_selected(&selected, &sessions_root).unwrap();
        let archive = create_safety_archive(&quarantine, &snapshot, &sessions_root).unwrap();
        verify_safety_archive(&archive, &snapshot).unwrap();
        assert!(archive.file_name().unwrap().to_string_lossy().starts_with("codex-delete-safety-"));
        let collision = create_quarantine_path(&quarantine).unwrap(); File::create(&collision).unwrap(); assert_ne!(collision, create_quarantine_path(&quarantine).unwrap());
        assert!(first.exists() && second.exists());
        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn rollback_restores_deleted_files_with_create_new_semantics() {
        let home = root("rollback"); let first = session(&home, "one.jsonl", "one"); let second = session(&home, "two.jsonl", "two");
        let quarantine = home.join("quarantine"); let result = execute_at(&config(), &home, &["one".into(), "two".into()], "DELETE", Some(1), Some(&quarantine)).unwrap_err();
        assert_eq!(result.result.outcome, DeleteOutcome::RolledBack); assert_eq!(result.result.deleted_count, 1); assert_eq!(result.result.restored_count, 1); assert!(first.exists() && second.exists());
        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn rejects_duplicates_and_missing_files_before_mutation() {
        let home = root("invalid"); let path = session(&home, "one.jsonl", "one");
        assert_eq!(execute_at(&config(), &home, &["one".into(), "one".into()], "DELETE", None, None).unwrap_err().code, "deletion_validation_failed");
        fs::remove_file(&path).unwrap(); assert_eq!(execute_at(&config(), &home, &["one".into()], "DELETE", None, None).unwrap_err().code, "deletion_validation_failed");
        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn rejects_traversal_paths_during_recovery() {
        let root = root("traversal"); fs::create_dir_all(&root).unwrap();
        assert!(restore_target(&root, "sessions/../../outside.jsonl").is_err());
        assert!(restore_target(&root, "not-sessions/file.jsonl").is_err());
        let _ = fs::remove_dir_all(root);
    }
}
