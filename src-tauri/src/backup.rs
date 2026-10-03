use std::{
    collections::{HashMap, HashSet},
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        Mutex, OnceLock,
    },
};

use serde::{Deserialize, Serialize};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};
use zip::{write::SimpleFileOptions, CompressionMethod, ZipArchive, ZipWriter};

use crate::{
    codex,
    config::AppConfiguration,
    platform,
    restore_history::{self, RestoreOutcome},
    session_storage::{self, SelectedSession},
};

const FORMAT_VERSION: u32 = 1;
const SAFETY_BACKUP_FORMAT_VERSION: u32 = 1;
const MAX_MANIFEST_BYTES: u64 = 1024 * 1024;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupPreview {
    pub format_version: u32,
    pub session_count: usize,
    pub total_bytes: u64,
    pub backup_directory: String,
    pub sessions: Vec<BackupSession>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupResult {
    pub archive_path: String,
    pub session_count: usize,
    pub total_bytes: u64,
}

/// A read-only summary of a selected archive. The source path deliberately never crosses the
/// Tauri boundary; the UI gets only its file name and validated manifest metadata.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveInspection {
    pub archive_name: String,
    pub created_at: Option<String>,
    pub platform: Option<String>,
    pub codex_cli_version: Option<String>,
    pub session_count: usize,
    pub total_bytes: u64,
    pub validation: ArchiveValidation,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
    pub restore_token: Option<String>,
    pub sessions: Vec<BackupSession>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestorePreview {
    pub session_count: usize,
    pub total_bytes: u64,
    pub destination_root: String,
    pub sessions: Vec<RestoreSession>,
    pub conflict_count: usize,
    pub planned_creates: usize,
    pub safety_backup_will_be_created: bool,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreSession {
    pub id: String,
    pub archive_path: String,
    pub destination_path: String,
    pub bytes: u64,
    pub conflict: bool,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreResult {
    pub restored_count: usize,
    pub skipped_conflicts: usize,
    pub total_bytes: u64,
    pub safety_backup_path: Option<String>,
}

/// Stable, serializable error data returned by backup/restore commands. Messages are diagnostic
/// text and intentionally remain native (they are never treated as UI translation keys).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupError {
    pub code: &'static str,
    pub message: String,
}
impl std::fmt::Display for BackupError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}
impl BackupError {
    pub fn from_message(message: String) -> Self {
        let code = error_code(&message);
        Self { code, message }
    }
}

fn error_code(message: &str) -> &'static str {
    if message.contains("rollback incomplete") {
        return "restore_rollback_failed";
    }
    if message.contains("Unsupported backup format") || message.contains("unsupported format") {
        "unsupported_format"
    } else if message.contains("deletion_confirmation_required") {
        "deletion_confirmation_required"
    } else if message.contains("deletion_validation_failed") {
        "deletion_validation_failed"
    } else if message.contains("deletion_archive_failed") {
        "deletion_archive_failed"
    } else if message.contains("deletion_rolled_back") {
        "deletion_rolled_back"
    } else if message.contains("deletion_rollback_failed") {
        "deletion_rollback_failed"
    } else if message.contains("symbolic link")
        || message.contains("outside")
        || message.contains("unsafe")
    {
        "unsafe_path"
    } else if message.contains("rolled back") {
        "restore_rolled_back"
    } else if message.contains("safety backup") {
        "safety_backup_failed"
    } else if message.contains("archive") || message.contains("ZIP") || message.contains("manifest")
    {
        "invalid_archive"
    } else if message.contains("metadata") || message.contains("selected archive entry") {
        "restore_validation_failed"
    } else {
        "backup_operation_failed"
    }
}

static RESTORE_ARCHIVES: OnceLock<Mutex<HashMap<String, PathBuf>>> = OnceLock::new();
static RESTORE_TOKEN: AtomicU64 = AtomicU64::new(1);
pub fn register_restore_archive(path: PathBuf) -> String {
    let token = format!("restore-{}", RESTORE_TOKEN.fetch_add(1, Ordering::Relaxed));
    RESTORE_ARCHIVES
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .expect("restore archive registry lock")
        .insert(token.clone(), path);
    token
}
fn archive_for_token(token: &str) -> Result<PathBuf, String> {
    RESTORE_ARCHIVES
        .get()
        .and_then(|items| items.lock().ok()?.get(token).cloned())
        .ok_or_else(|| "This archive must be inspected successfully before restore.".into())
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveValidation {
    pub valid: bool,
    pub format_version: Option<u32>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupSession {
    pub id: String,
    pub title: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub archive_path: String,
    pub bytes: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Manifest {
    format_version: u32,
    created_at: String,
    platform: String,
    codex_cli_version: Option<String>,
    sessions: Vec<ManifestSession>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ManifestSession {
    id: String,
    title: Option<String>,
    created_at: Option<String>,
    updated_at: Option<String>,
    archive_path: String,
    bytes: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SafetyBackupManifest {
    format_version: u32,
    created_at: String,
    kind: &'static str,
    reason: &'static str,
    conflicts: Vec<SafetyBackupConflict>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SafetyBackupConflict {
    id: String,
    archive_path: String,
    bytes: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ArchiveManifest {
    format_version: u32,
    created_at: String,
    platform: String,
    codex_cli_version: Option<String>,
    sessions: Vec<ArchiveManifestSession>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ArchiveManifestSession {
    id: String,
    title: Option<String>,
    created_at: Option<String>,
    updated_at: Option<String>,
    archive_path: String,
    bytes: u64,
}

pub fn preview(configuration: &AppConfiguration, ids: &[String]) -> Result<BackupPreview, String> {
    let selected = session_storage::selected_sessions(&platform::codex_home(), ids)?;
    let (sessions, total_bytes) = describe(&selected)?;
    Ok(BackupPreview {
        format_version: FORMAT_VERSION,
        session_count: sessions.len(),
        total_bytes,
        backup_directory: display(platform::backup_dir(configuration)),
        sessions,
    })
}

pub fn create(configuration: &AppConfiguration, ids: &[String]) -> Result<BackupResult, String> {
    let selected = session_storage::selected_sessions(&platform::codex_home(), ids)?;
    let (sessions, total_bytes) = describe(&selected)?;
    let directory = platform::backup_dir(configuration);
    fs::create_dir_all(&directory)
        .map_err(|error| format!("Unable to create the backup directory: {error}"))?;
    let archive_path = create_archive_path(&directory)?;
    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&archive_path)
        .map_err(|error| format!("Unable to reserve a new backup archive: {error}"))?;
    if let Err(error) = write_archive(file, &selected, &sessions).and_then(|_| {
        let verified = inspect(&archive_path)?;
        if !verified.validation.valid {
            return Err("Backup archive verification failed.".into());
        }
        Ok(())
    }) {
        let _ = fs::remove_file(&archive_path);
        return Err(error);
    }
    Ok(BackupResult {
        archive_path: display(archive_path),
        session_count: sessions.len(),
        total_bytes,
    })
}

/// Opens a ZIP for inspection only. It never extracts an entry or writes to either the archive
/// location or CODEX_HOME.
pub fn inspect(path: &Path) -> Result<ArchiveInspection, String> {
    let archive_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("Selected archive")
        .to_owned();
    let file = File::open(path)
        .map_err(|error| format!("Unable to open the selected archive: {error}"))?;
    let mut archive = ZipArchive::new(file)
        .map_err(|error| format!("The selected file is not a readable ZIP archive: {error}"))?;
    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    let mut entries = HashMap::new();

    for index in 0..archive.len() {
        let entry = archive
            .by_index(index)
            .map_err(|error| format!("Unable to inspect ZIP entries: {error}"))?;
        let name = entry.name().to_owned();
        if let Err(error) = validate_zip_path(&name) {
            errors.push(error);
            continue;
        }
        if entry.unix_mode().is_some_and(|m| m & 0o170000 == 0o120000) {
            errors.push("ZIP symbolic link rejected.".into());
        }
        if entry.is_dir() {
            errors.push(format!("ZIP entry must be a file, not a directory: {name}"));
            continue;
        }
        if entries.insert(name.clone(), entry.size()).is_some() {
            errors.push(format!("ZIP contains a duplicate entry: {name}"));
        }
    }

    let Some(manifest_bytes) = read_manifest(&mut archive, &entries, &mut errors) else {
        return Ok(inspection_from_errors(archive_name, warnings, errors));
    };
    let manifest = match serde_json::from_slice::<ArchiveManifest>(&manifest_bytes) {
        Ok(manifest) => manifest,
        Err(error) => {
            errors.push(format!(
                "manifest.json does not match the supported schema: {error}"
            ));
            return Ok(inspection_from_errors(archive_name, warnings, errors));
        }
    };

    if manifest.format_version != FORMAT_VERSION {
        errors.push(format!(
            "Unsupported backup format version {} (this app supports {}).",
            manifest.format_version, FORMAT_VERSION
        ));
    }
    if OffsetDateTime::parse(&manifest.created_at, &Rfc3339).is_err() {
        errors.push("manifest.json createdAt must be an RFC 3339 timestamp.".into());
    }
    if manifest.platform.trim().is_empty() {
        errors.push("manifest.json platform must not be empty.".into());
    }
    if manifest.sessions.is_empty() {
        errors.push("manifest.json must list at least one selected session.".into());
    }
    if !matches!(manifest.codex_cli_version.as_deref(), Some(value) if !value.is_empty()) {
        warnings.push("The archive does not record a Codex CLI version.".into());
    }

    let mut paths = HashSet::new();
    let mut ids = HashSet::new();
    let mut total_bytes = 0_u64;
    for session in &manifest.sessions {
        if session.id.trim().is_empty() {
            errors.push("A manifest session has an empty id.".into());
        } else if !ids.insert(&session.id) {
            errors.push(format!(
                "manifest.json contains a duplicate session id: {}",
                session.id
            ));
        }
        if let Some(value) = &session.title {
            if value.is_empty() {
                errors.push("A manifest session title must be a string when present.".into());
            }
        }
        for (label, value) in [
            ("createdAt", &session.created_at),
            ("updatedAt", &session.updated_at),
        ] {
            if let Some(value) = value {
                if OffsetDateTime::parse(value, &Rfc3339).is_err() {
                    errors.push(format!(
                        "A session {label} value must be an RFC 3339 timestamp."
                    ));
                }
            }
        }
        if let Err(error) = validate_session_archive_path(&session.archive_path) {
            errors.push(error);
            continue;
        }
        if !paths.insert(&session.archive_path) {
            errors.push(format!(
                "manifest.json contains a duplicate archivePath: {}",
                session.archive_path
            ));
        }
        match entries.get(&session.archive_path) {
            Some(size) if *size == session.bytes => {
                if let Ok(mut entry) = archive.by_name(&session.archive_path) {
                    if io::copy(&mut entry, &mut io::sink()).is_err() {
                        errors.push("Archive payload integrity check failed.".into());
                    }
                }
            }
            Some(size) => errors.push(format!(
                "Byte count differs for {} (manifest: {}, ZIP: {size}).",
                session.archive_path, session.bytes
            )),
            None => errors.push(format!(
                "Selected session file is missing from the ZIP: {}",
                session.archive_path
            )),
        }
        total_bytes = match total_bytes.checked_add(session.bytes) {
            Some(total) => total,
            None => {
                errors.push("Manifest session byte counts overflow the supported total.".into());
                total_bytes
            }
        };
    }
    let expected: HashSet<&str> = manifest
        .sessions
        .iter()
        .map(|session| session.archive_path.as_str())
        .collect();
    for name in entries.keys() {
        if name != "manifest.json"
            && name != "delete-manifest.json"
            && !expected.contains(name.as_str())
        {
            errors.push(format!("ZIP contains an unlisted entry: {name}"));
        }
    }

    Ok(ArchiveInspection {
        archive_name,
        created_at: Some(manifest.created_at.clone()),
        platform: Some(manifest.platform.clone()),
        codex_cli_version: manifest.codex_cli_version.clone(),
        session_count: manifest.sessions.len(),
        total_bytes,
        validation: ArchiveValidation {
            valid: errors.is_empty(),
            format_version: Some(manifest.format_version),
        },
        warnings,
        errors,
        restore_token: None,
        sessions: manifest
            .sessions
            .iter()
            .map(|s| BackupSession {
                id: s.id.clone(),
                title: s.title.clone(),
                created_at: s.created_at.clone(),
                updated_at: s.updated_at.clone(),
                archive_path: s.archive_path.clone(),
                bytes: s.bytes,
            })
            .collect(),
    })
}

fn inspection_from_errors(
    archive_name: String,
    warnings: Vec<String>,
    errors: Vec<String>,
) -> ArchiveInspection {
    ArchiveInspection {
        archive_name,
        created_at: None,
        platform: None,
        codex_cli_version: None,
        session_count: 0,
        total_bytes: 0,
        validation: ArchiveValidation {
            valid: false,
            format_version: None,
        },
        warnings,
        errors,
        restore_token: None,
        sessions: Vec::new(),
    }
}

/// Restore is deliberately a two-step operation. Both preview and execution re-run the archive
/// validator; execution never trusts metadata retained from a previous UI render.
pub fn preview_restore(
    configuration: &AppConfiguration,
    token: &str,
    ids: &[String],
) -> Result<RestorePreview, String> {
    let path = archive_for_token(token)?;
    let manifest = validated_manifest(&path)?;
    restore_preview_for(configuration, &manifest, ids)
}

pub fn restore(
    configuration: &AppConfiguration,
    token: &str,
    ids: &[String],
) -> Result<RestoreResult, String> {
    let path = archive_for_token(token)?;
    let archive_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("Selected archive")
        .to_owned();
    let operation = (|| {
        let manifest = validated_manifest(&path)?;
        let preview = restore_preview_for(configuration, &manifest, ids)?;
        restore_from_path(configuration, &path, &manifest, &preview, None)
    })();
    match &operation {
        Ok(result) => {
            let outcome = if result.skipped_conflicts > 0 {
                RestoreOutcome::Partial
            } else {
                RestoreOutcome::Completed
            };
            record_restore_history(configuration, archive_name, ids, result, outcome, None);
        }
        Err(error) => {
            let code = error_code(error);
            let outcome = if code == "restore_rolled_back" {
                RestoreOutcome::RolledBack
            } else {
                RestoreOutcome::Failed
            };
            record_restore_history(
                configuration,
                archive_name,
                ids,
                &RestoreResult {
                    restored_count: 0,
                    skipped_conflicts: 0,
                    total_bytes: 0,
                    safety_backup_path: None,
                },
                outcome,
                Some(code.to_owned()),
            );
        }
    }
    operation
}

fn record_restore_history(
    configuration: &AppConfiguration,
    archive_name: String,
    ids: &[String],
    result: &RestoreResult,
    outcome: RestoreOutcome,
    error_code: Option<String>,
) {
    if let Err(error) = restore_history::record(
        configuration,
        archive_name,
        ids.to_vec(),
        result.restored_count,
        result.skipped_conflicts,
        result.safety_backup_path.clone(),
        outcome,
        error_code,
    ) {
        tracing::warn!("restore history could not be recorded: {error}");
    }
}

fn validated_manifest(path: &Path) -> Result<ArchiveManifest, String> {
    let inspection = inspect(path)?;
    if !inspection.validation.valid {
        return Err("The archive is no longer valid and cannot be restored.".into());
    }
    let file =
        File::open(path).map_err(|_| "The inspected archive is no longer readable.".to_string())?;
    let mut archive = ZipArchive::new(file)
        .map_err(|_| "The inspected archive is no longer a readable ZIP.".to_string())?;
    let entries = (0..archive.len())
        .map(|i| {
            let e = archive.by_index(i).unwrap();
            (e.name().to_owned(), e.size())
        })
        .collect();
    let bytes = read_manifest(&mut archive, &entries, &mut Vec::new())
        .ok_or("Invalid archive manifest.")?;
    serde_json::from_slice(&bytes).map_err(|_| "The inspected manifest is invalid.".to_string())
}

fn selected_manifest_sessions<'a>(
    manifest: &'a ArchiveManifest,
    ids: &[String],
) -> Result<Vec<&'a ArchiveManifestSession>, String> {
    if ids.is_empty() {
        return Err("Select at least one archive session to restore.".into());
    }
    let requested: HashSet<&str> = ids.iter().map(String::as_str).collect();
    if requested.len() != ids.len() {
        return Err("Duplicate restore selections are not allowed.".into());
    }
    let selected: Vec<_> = manifest
        .sessions
        .iter()
        .filter(|session| requested.contains(session.id.as_str()))
        .collect();
    if selected.len() != ids.len() {
        return Err("One or more selected archive sessions are unavailable.".into());
    }
    Ok(selected)
}

fn restore_preview_for(
    configuration: &AppConfiguration,
    manifest: &ArchiveManifest,
    ids: &[String],
) -> Result<RestorePreview, String> {
    let root = destination_root()?;
    let selected = selected_manifest_sessions(manifest, ids)?;
    let mut total_bytes = 0_u64;
    let mut conflict_count = 0;
    let sessions = selected
        .into_iter()
        .map(|session| {
            let destination = destination_for(&root, &session.archive_path)?;
            let conflict = fs::symlink_metadata(&destination).is_ok();
            if conflict {
                conflict_count += 1;
            }
            total_bytes = total_bytes
                .checked_add(session.bytes)
                .ok_or("Selected restore files are too large.")?;
            Ok(RestoreSession {
                id: session.id.clone(),
                archive_path: session.archive_path.clone(),
                destination_path: display(destination),
                bytes: session.bytes,
                conflict,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let session_count = sessions.len();
    Ok(RestorePreview {
        session_count,
        total_bytes,
        destination_root: display(root),
        sessions,
        conflict_count,
        planned_creates: session_count - conflict_count,
        safety_backup_will_be_created: configuration.create_safety_backups && conflict_count > 0,
    })
}

fn destination_root() -> Result<PathBuf, String> {
    let root = platform::codex_home().join("sessions");
    crate::fs_safety::check(&root)?;
    Ok(root)
}

fn destination_for(root: &Path, archive_path: &str) -> Result<PathBuf, String> {
    validate_session_archive_path(archive_path)?;
    let relative = Path::new(archive_path)
        .strip_prefix("sessions")
        .map_err(|_| "Invalid restore session path.".to_string())?;
    let destination = root.join(relative);
    if !destination.starts_with(root) {
        return Err("Rejected a restore path outside the Codex sessions directory.".into());
    }
    Ok(destination)
}

fn assert_no_symlink_escape(root: &Path, target: &Path) -> Result<(), String> {
    crate::fs_safety::check(target)?;
    fs::create_dir_all(root).map_err(|e| e.to_string())?;
    let parent = target.parent().ok_or("Invalid restore destination.")?;
    let relative = parent
        .strip_prefix(root)
        .map_err(|_| "Rejected a destination outside the sessions directory.".to_string())?;
    let mut current = root.to_path_buf();
    for part in relative.components() {
        current.push(part);
        if fs::symlink_metadata(&current).is_ok() {
            if fs::symlink_metadata(&current)
                .map_err(|_| "Unable to inspect restore destination.")?
                .file_type()
                .is_symlink()
            {
                return Err("Rejected restore destination through a symbolic link.".into());
            }
        } else {
            match fs::create_dir(&current) {
                Ok(()) => {}
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                    if fs::symlink_metadata(&current)
                        .map_err(|_| "Unable to inspect restore destination.")?
                        .file_type()
                        .is_symlink()
                    {
                        return Err("Rejected restore destination through a symbolic link.".into());
                    }
                }
                Err(_) => return Err("Unable to create a restore destination directory.".into()),
            }
        }
    }
    Ok(())
}

fn validate_session_payload(bytes: &[u8], id: &str) -> Result<(), String> {
    session_storage::validate_restorable_session_metadata(bytes, id).map_err(str::to_owned)
}

fn restore_from_path(
    configuration: &AppConfiguration,
    path: &Path,
    manifest: &ArchiveManifest,
    preview: &RestorePreview,
    fail_after: Option<usize>,
) -> Result<RestoreResult, String> {
    restore_from_path_at_root(
        configuration,
        path,
        manifest,
        preview,
        &destination_root()?,
        fail_after,
    )
}

fn restore_from_path_at_root(
    configuration: &AppConfiguration,
    path: &Path,
    manifest: &ArchiveManifest,
    preview: &RestorePreview,
    root: &Path,
    fail_after: Option<usize>,
) -> Result<RestoreResult, String> {
    let selected = selected_manifest_sessions(
        manifest,
        &preview
            .sessions
            .iter()
            .map(|item| item.id.clone())
            .collect::<Vec<_>>(),
    )?;
    let file =
        File::open(path).map_err(|_| "The inspected archive is no longer readable.".to_string())?;
    let mut archive = ZipArchive::new(file)
        .map_err(|_| "The inspected archive is no longer a readable ZIP.".to_string())?;
    let mut contents = Vec::new();
    for session in &selected {
        let mut entry = archive
            .by_name(&session.archive_path)
            .map_err(|_| "A selected archive entry is unavailable.".to_string())?;
        let mut bytes = Vec::new();
        entry
            .read_to_end(&mut bytes)
            .map_err(|_| "A selected archive entry could not be read.".to_string())?;
        if bytes.len() as u64 != session.bytes {
            return Err("A selected archive entry changed size during restore.".into());
        }
        validate_session_payload(&bytes, &session.id)?;
        contents.push((session, bytes));
    }
    // This is a fresh execution-time snapshot; preview conflicts are advisory only and never
    // authorize a write. Every target is inspected again before any directory/file creation.
    let conflicts: Vec<_> = selected
        .iter()
        .map(|session| {
            let destination = destination_for(root, &session.archive_path)?;
            Ok(fs::symlink_metadata(&destination)
                .ok()
                .map(|metadata| (session, destination, metadata)))
        })
        .collect::<Result<Vec<_>, String>>()?
        .into_iter()
        .flatten()
        .collect();
    for (_, destination, metadata) in &conflicts {
        if metadata.file_type().is_symlink() || !metadata.file_type().is_file() {
            return Err(format!(
                "Restore conflict is not a regular file and was not followed: {}",
                display(destination)
            ));
        }
    }
    let safety_backup_path = if configuration.create_safety_backups && !conflicts.is_empty() {
        Some(create_conflict_safety_backup(
            configuration,
            root,
            &conflicts,
        )?)
    } else {
        None
    };
    let mut created = Vec::new();
    let mut skipped = 0;
    let mut total = 0;
    let result = (|| -> Result<(), String> {
        for (index, (session, bytes)) in contents.iter().enumerate() {
            let destination = destination_for(root, &session.archive_path)?;
            assert_no_symlink_escape(root, &destination)?;
            if fs::symlink_metadata(&destination).is_ok() {
                skipped += 1;
                continue;
            }
            if fail_after == Some(index) {
                return Err("Injected copy failure.".into());
            }
            match OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&destination)
            {
                Ok(mut output) => {
                    created.push(destination.clone());
                    output
                        .write_all(bytes)
                        .and_then(|_| output.sync_all())
                        .map_err(|e| e.to_string())?;
                    drop(output);
                    if fs::read(&destination).map_err(|e| e.to_string())? != *bytes {
                        return Err("Restore verification failed.".into());
                    }
                    total += bytes.len() as u64;
                }
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => skipped += 1,
                Err(e) => return Err(e.to_string()),
            }
        }
        Ok(())
    })();
    if let Err(error) = result {
        let remaining = rollback(&created);
        return Err(if remaining == 0 {
            format!("Restore failed; created files were rolled back: {error}")
        } else {
            format!("Restore failed; rollback incomplete, {remaining} files remain: {error}")
        });
    }
    Ok(RestoreResult {
        restored_count: created.len(),
        skipped_conflicts: skipped,
        total_bytes: total,
        safety_backup_path,
    })
}
fn rollback(created: &[PathBuf]) -> usize {
    created
        .iter()
        .rev()
        .filter(|file| crate::fs_safety::check(file).is_err() || fs::remove_file(file).is_err())
        .count()
}
fn create_conflict_safety_backup(
    configuration: &AppConfiguration,
    root: &Path,
    conflicts: &[(&&ArchiveManifestSession, PathBuf, fs::Metadata)],
) -> Result<String, String> {
    let directory = platform::backup_dir(configuration);
    fs::create_dir_all(&directory).map_err(|_| "Unable to create the safety backup directory.")?;
    let path = create_safety_backup_path(&directory)?;
    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|_| "Unable to reserve a safety backup archive.")?;
    let created_at = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .map_err(|_| "Unable to timestamp safety backup.")?;
    let manifest = SafetyBackupManifest {
        format_version: SAFETY_BACKUP_FORMAT_VERSION,
        created_at,
        kind: "codex-companion-safety-backup",
        reason: "pre-restore-conflicts",
        conflicts: conflicts
            .iter()
            .map(|(session, _, metadata)| SafetyBackupConflict {
                id: session.id.clone(),
                archive_path: session.archive_path.clone(),
                bytes: metadata.len(),
            })
            .collect(),
    };
    let result = (|| -> Result<(), String> {
        let mut archive = ZipWriter::new(file);
        let options = SimpleFileOptions::default()
            .compression_method(CompressionMethod::Deflated)
            .unix_permissions(0o600);
        archive
            .start_file("safety-manifest.json", options)
            .map_err(zip_error)?;
        archive
            .write_all(
                &serde_json::to_vec_pretty(&manifest)
                    .map_err(|_| "Unable to serialize safety backup manifest.")?,
            )
            .map_err(io_error)?;
        for (session, destination, _) in conflicts {
            assert_no_symlink_escape(root, destination)?;
            let metadata = fs::symlink_metadata(destination).map_err(|_| {
                "A restore conflict disappeared before its safety backup was created."
            })?;
            if metadata.file_type().is_symlink() || !metadata.file_type().is_file() {
                return Err("A restore conflict changed to an unsafe file type before its safety backup was created.".into());
            }
            archive
                .start_file(format!("conflicts/{}", session.archive_path), options)
                .map_err(zip_error)?;
            let mut input = File::open(destination)
                .map_err(|_| "Unable to read an existing conflict for safety backup.")?;
            io::copy(&mut input, &mut archive).map_err(io_error)?;
        }
        archive.finish().map_err(zip_error)?;
        Ok(())
    })();
    if let Err(error) = result {
        let _ = fs::remove_file(&path);
        return Err(error);
    }
    Ok(display(path))
}

fn read_manifest(
    archive: &mut ZipArchive<File>,
    entries: &HashMap<String, u64>,
    errors: &mut Vec<String>,
) -> Option<Vec<u8>> {
    let name = if entries.contains_key("manifest.json") {
        "manifest.json"
    } else {
        "delete-manifest.json"
    };
    let Some(size) = entries.get(name) else {
        errors.push("ZIP archive is missing manifest.json.".into());
        return None;
    };
    if *size > MAX_MANIFEST_BYTES {
        errors.push("manifest.json exceeds the 1 MiB inspection limit.".into());
        return None;
    }
    let mut manifest = match archive.by_name(name) {
        Ok(file) => file,
        Err(error) => {
            errors.push(format!("Unable to read manifest.json: {error}"));
            return None;
        }
    };
    let mut bytes = Vec::with_capacity(*size as usize);
    if let Err(error) = manifest.read_to_end(&mut bytes) {
        errors.push(format!("Unable to read manifest.json: {error}"));
        return None;
    }
    if name == "delete-manifest.json" {
        let mut value: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
        if value["kind"] != "codex-companion-local-delete-safety-archive" {
            errors.push("Invalid delete archive kind.".into());
            return None;
        }
        value.as_object_mut()?.remove("kind");
        value["platform"] = "unknown".into();
        return serde_json::to_vec(&value).ok();
    }
    Some(bytes)
}

fn validate_zip_path(name: &str) -> Result<(), String> {
    crate::fs_safety::relative(name).map_err(|_| format!("Rejected unsafe ZIP path: {name}"))?;
    if name.is_empty()
        || name.contains('\\')
        || name.contains('\0')
        || name.starts_with('/')
        || name.starts_with("//")
        || name.contains(':')
    {
        return Err(format!("Rejected unsafe ZIP path: {name}"));
    }
    if name
        .split('/')
        .any(|part| part.is_empty() || part == "." || part == "..")
    {
        return Err(format!("Rejected unsafe ZIP path: {name}"));
    }
    Ok(())
}

fn validate_session_archive_path(path: &str) -> Result<(), String> {
    validate_zip_path(path)?;
    if !path.starts_with("sessions/") || !path.ends_with(".jsonl") {
        return Err(format!("Invalid session archivePath: {path}"));
    }
    Ok(())
}

fn describe(selected: &[SelectedSession]) -> Result<(Vec<BackupSession>, u64), String> {
    let mut total_bytes = 0_u64;
    let mut sessions = Vec::with_capacity(selected.len());
    for item in selected {
        let bytes = fs::metadata(&item.source_path)
            .map_err(|_| format!("Selected conversation cannot be read: {}", item.summary.id))?
            .len();
        total_bytes = total_bytes
            .checked_add(bytes)
            .ok_or("Selected files are too large to archive.")?;
        sessions.push(BackupSession {
            id: item.summary.id.clone(),
            title: item.summary.title.clone(),
            created_at: item.summary.created_at.clone(),
            updated_at: item.summary.updated_at.clone(),
            archive_path: item.archive_path.clone(),
            bytes,
        });
    }
    Ok((sessions, total_bytes))
}

fn write_archive(
    file: File,
    selected: &[SelectedSession],
    sessions: &[BackupSession],
) -> Result<(), String> {
    let mut inputs = selected
        .iter()
        .map(|item| crate::environment::locked_read(&item.source_path))
        .collect::<Result<Vec<_>, _>>()?;
    let created_at = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .map_err(|error| error.to_string())?;
    let manifest = Manifest {
        format_version: FORMAT_VERSION,
        created_at,
        platform: platform::operating_system().to_string(),
        codex_cli_version: codex::cli_version(),
        sessions: sessions
            .iter()
            .map(|session| ManifestSession {
                id: session.id.clone(),
                title: session.title.clone(),
                created_at: session.created_at.clone(),
                updated_at: session.updated_at.clone(),
                archive_path: session.archive_path.clone(),
                bytes: session.bytes,
            })
            .collect(),
    };
    let mut archive = ZipWriter::new(file);
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .unix_permissions(0o600);
    archive
        .start_file("manifest.json", options)
        .map_err(zip_error)?;
    archive
        .write_all(&serde_json::to_vec_pretty(&manifest).map_err(|error| error.to_string())?)
        .map_err(io_error)?;
    for (index, item) in selected.iter().enumerate() {
        archive
            .start_file(&item.archive_path, options)
            .map_err(zip_error)?;
        let copied = io::copy(&mut inputs[index], &mut archive).map_err(io_error)?;
        if copied != sessions[index].bytes {
            return Err("Source changed; close Codex and preview again.".into());
        }
    }
    archive
        .finish()
        .map_err(zip_error)?
        .sync_all()
        .map_err(io_error)?;
    Ok(())
}

fn create_archive_path(directory: &Path) -> Result<PathBuf, String> {
    let format = time::format_description::parse_borrowed::<2>(
        "[year]-[month]-[day]_[hour][minute][second]",
    )
    .map_err(|error| error.to_string())?;
    let stamp = OffsetDateTime::now_utc()
        .format(&format)
        .map_err(|error| error.to_string())?;
    for suffix in 0..10_000_u32 {
        let name = if suffix == 0 {
            format!("codex-backup-{stamp}.zip")
        } else {
            format!("codex-backup-{stamp}-{suffix}.zip")
        };
        let candidate = directory.join(name);
        if !candidate.exists() {
            return Ok(candidate);
        }
    }
    Err("Could not find an unused backup archive name.".into())
}

fn create_safety_backup_path(directory: &Path) -> Result<PathBuf, String> {
    let format = time::format_description::parse_borrowed::<2>(
        "[year]-[month]-[day]_[hour][minute][second]",
    )
    .map_err(|error| error.to_string())?;
    let stamp = OffsetDateTime::now_utc()
        .format(&format)
        .map_err(|error| error.to_string())?;
    for suffix in 0..10_000_u32 {
        let name = if suffix == 0 {
            format!("codex-safety-backup-{stamp}.zip")
        } else {
            format!("codex-safety-backup-{stamp}-{suffix}.zip")
        };
        let candidate = directory.join(name);
        if !candidate.exists() {
            return Ok(candidate);
        }
    }
    Err("Could not find an unused safety backup archive name.".into())
}

fn display(path: impl AsRef<Path>) -> String {
    path.as_ref().to_string_lossy().to_string()
}
fn io_error(error: io::Error) -> String {
    format!("Backup archive write failed: {error}")
}
fn zip_error(error: zip::result::ZipError) -> String {
    format!("Backup archive write failed: {error}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn existing_delete_safety_archive_is_inspectable_and_restorable() {
        let root = restore_fixture_root("delete-recovery");
        let path = root.join("delete.zip");
        let payload = valid_jsonl("one");
        let mut zip = ZipWriter::new(File::create(&path).unwrap());
        let options = SimpleFileOptions::default();
        zip.start_file("delete-manifest.json", options).unwrap();
        zip.write_all(serde_json::json!({"formatVersion":1,"kind":"codex-companion-local-delete-safety-archive","createdAt":"2026-09-07T00:00:00Z","sessions":[{"id":"one","archivePath":"sessions/one.jsonl","bytes":payload.len()}]}).to_string().as_bytes()).unwrap();
        zip.start_file("sessions/one.jsonl", options).unwrap();
        zip.write_all(&payload).unwrap();
        zip.finish().unwrap();
        assert!(inspect(&path).unwrap().validation.valid);
        let manifest = validated_manifest(&path).unwrap();
        let target = root.join("empty/sessions");
        let result = restore_from_path_at_root(
            &no_safety_configuration(),
            &path,
            &manifest,
            &restore_preview(&manifest, &target),
            &target,
            None,
        )
        .unwrap();
        assert_eq!(result.restored_count, 1);
        assert_eq!(fs::read(target.join("one.jsonl")).unwrap(), payload);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rollback_reports_locked_file_instead_of_claiming_success() {
        let root = restore_fixture_root("rollback-denied");
        let path = root.join("locked");
        fs::write(&path, "fixture").unwrap();
        let handle = crate::environment::locked_read(&path).unwrap();
        #[cfg(windows)]
        assert_eq!(rollback(&[path.clone()]), 1);
        drop(handle);
        assert_eq!(rollback(&[path]), 0);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn manifest_is_versioned_and_contains_only_selected_synthetic_sessions() {
        let root = std::env::temp_dir().join(format!(
            "codex-companion-backup-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let sessions = root.join("sessions/2026/09/04");
        fs::create_dir_all(&sessions).unwrap();
        let selected_path = sessions.join("rollout-selected.jsonl");
        fs::write(&selected_path, "synthetic selected fixture").unwrap();
        fs::write(
            sessions.join("rollout-unselected.jsonl"),
            "synthetic unselected fixture",
        )
        .unwrap();
        let selected = SelectedSession {
            summary: crate::session_storage::ConversationSummary {
                id: "selected".into(),
                title: Some("Synthetic".into()),
                created_at: None,
                updated_at: None,
                project_path: None,
                project_name: None,
                source: None,
            },
            source_path: selected_path,
            archive_path: "sessions/2026/09/04/rollout-selected.jsonl".into(),
        };
        let (described, _) = describe(&[selected.clone()]).unwrap();
        let output = root.join("backup.zip");
        write_archive(File::create(&output).unwrap(), &[selected], &described).unwrap();
        let mut archive = zip::ZipArchive::new(File::open(&output).unwrap()).unwrap();
        assert!(archive.by_name("manifest.json").is_ok());
        assert!(archive
            .by_name("sessions/2026/09/04/rollout-selected.jsonl")
            .is_ok());
        assert!(archive
            .by_name("sessions/2026/09/04/rollout-unselected.jsonl")
            .is_err());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn archive_name_does_not_overwrite_existing_file() {
        let root = std::env::temp_dir().join(format!(
            "codex-companion-name-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        let first = create_archive_path(&root).unwrap();
        File::create(&first).unwrap();
        let second = create_archive_path(&root).unwrap();
        assert_ne!(first, second);
        let _ = fs::remove_dir_all(root);
    }

    fn synthetic_manifest() -> String {
        r#"{"formatVersion":1,"createdAt":"2026-09-04T10:00:00Z","platform":"windows","codexCliVersion":"codex-cli 0.153.0","sessions":[{"id":"synthetic-a","title":null,"createdAt":null,"updatedAt":null,"archivePath":"sessions/2026/09/04/rollout-a.jsonl","bytes":17}]}"#.into()
    }

    fn write_fixture(path: &Path, manifest: &str, entries: &[(&str, &[u8])]) {
        let file = File::create(path).unwrap();
        let mut archive = ZipWriter::new(file);
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
        archive.start_file("manifest.json", options).unwrap();
        archive.write_all(manifest.as_bytes()).unwrap();
        for (name, content) in entries {
            archive.start_file(*name, options).unwrap();
            archive.write_all(content).unwrap();
        }
        archive.finish().unwrap();
    }

    fn fixture_path(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "codex-companion-inspection-{label}-{}.zip",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[test]
    fn inspects_valid_synthetic_archive() {
        let path = fixture_path("valid");
        write_fixture(
            &path,
            &synthetic_manifest(),
            &[("sessions/2026/09/04/rollout-a.jsonl", b"synthetic payload")],
        );
        let result = inspect(&path).unwrap();
        assert!(result.validation.valid, "{:?}", result.errors);
        assert_eq!(result.session_count, 1);
        assert_eq!(result.total_bytes, 17);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn rejects_unsupported_format_version() {
        let path = fixture_path("version");
        let manifest = synthetic_manifest().replace("\"formatVersion\":1", "\"formatVersion\":999");
        write_fixture(
            &path,
            &manifest,
            &[("sessions/2026/09/04/rollout-a.jsonl", b"synthetic payload")],
        );
        let result = inspect(&path).unwrap();
        assert!(!result.validation.valid);
        assert!(result
            .errors
            .iter()
            .any(|error| error.contains("Unsupported backup format version")));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn backup_format_v1_remains_the_only_backward_compatible_restore_format() {
        let path = fixture_path("v1-compatibility");
        let payload = valid_jsonl("synthetic-a");
        let manifest =
            synthetic_manifest().replace("\"bytes\":17", &format!("\"bytes\":{}", payload.len()));
        write_fixture(
            &path,
            &manifest,
            &[("sessions/2026/09/04/rollout-a.jsonl", payload.as_slice())],
        );
        let inspection = inspect(&path).unwrap();
        assert!(inspection.validation.valid, "{:?}", inspection.errors);
        assert_eq!(inspection.validation.format_version, Some(FORMAT_VERSION));
        assert!(validate_session_payload(&payload, "synthetic-a").is_ok());
        let _ = fs::remove_file(path);
    }

    #[test]
    fn rejects_malformed_manifest() {
        let path = fixture_path("malformed");
        write_fixture(&path, "{", &[]);
        let result = inspect(&path).unwrap();
        assert!(!result.validation.valid);
        assert!(result
            .errors
            .iter()
            .any(|error| error.contains("does not match")));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn rejects_zip_traversal_path() {
        let path = fixture_path("traversal");
        write_fixture(
            &path,
            &synthetic_manifest(),
            &[(
                "../sessions/2026/09/04/rollout-a.jsonl",
                b"synthetic payload",
            )],
        );
        let result = inspect(&path).unwrap();
        assert!(!result.validation.valid);
        assert!(result
            .errors
            .iter()
            .any(|error| error.contains("unsafe ZIP path")));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn rejects_missing_selected_session_file() {
        let path = fixture_path("missing");
        write_fixture(&path, &synthetic_manifest(), &[]);
        let result = inspect(&path).unwrap();
        assert!(!result.validation.valid);
        assert!(result
            .errors
            .iter()
            .any(|error| error.contains("session file is missing")));
        let _ = fs::remove_file(path);
    }

    fn valid_jsonl(id: &str) -> Vec<u8> {
        format!("{{\"type\":\"session_meta\",\"payload\":{{\"session_id\":\"{id}\"}}}}\n")
            .into_bytes()
    }

    fn restore_manifest(entries: &[(&str, &[u8])]) -> String {
        let sessions = entries.iter().map(|(id, bytes)| format!("{{\"id\":\"{id}\",\"title\":null,\"createdAt\":null,\"updatedAt\":null,\"archivePath\":\"sessions/2026/09/04/rollout-{id}.jsonl\",\"bytes\":{}}}", bytes.len())).collect::<Vec<_>>().join(",");
        format!("{{\"formatVersion\":1,\"createdAt\":\"2026-09-04T10:00:00Z\",\"platform\":\"windows\",\"codexCliVersion\":\"test\",\"sessions\":[{sessions}]}}")
    }

    fn restore_preview(manifest: &ArchiveManifest, root: &Path) -> RestorePreview {
        let sessions = manifest
            .sessions
            .iter()
            .map(|session| RestoreSession {
                id: session.id.clone(),
                archive_path: session.archive_path.clone(),
                destination_path: display(destination_for(root, &session.archive_path).unwrap()),
                bytes: session.bytes,
                conflict: false,
            })
            .collect::<Vec<_>>();
        RestorePreview {
            session_count: sessions.len(),
            total_bytes: sessions.iter().map(|session| session.bytes).sum(),
            destination_root: display(root),
            conflict_count: 0,
            planned_creates: sessions.len(),
            safety_backup_will_be_created: false,
            sessions,
        }
    }

    fn restore_fixture_root(label: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "codex-companion-restore-{label}-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        root
    }

    fn no_safety_configuration() -> AppConfiguration {
        let mut configuration = AppConfiguration::default();
        configuration.create_safety_backups = false;
        configuration
    }

    #[test]
    fn restores_valid_selected_synthetic_session_with_create_new_semantics() {
        let root = restore_fixture_root("success");
        let archive_path = root.join("fixture.zip");
        let payload = valid_jsonl("one");
        let entries = [("one", payload.as_slice())];
        let manifest_text = restore_manifest(&entries);
        write_fixture(
            &archive_path,
            &manifest_text,
            &[("sessions/2026/09/04/rollout-one.jsonl", payload.as_slice())],
        );
        let manifest: ArchiveManifest = serde_json::from_str(&manifest_text).unwrap();
        let preview = restore_preview(&manifest, &root);
        let result = restore_from_path_at_root(
            &no_safety_configuration(),
            &archive_path,
            &manifest,
            &preview,
            &root,
            None,
        )
        .unwrap();
        assert_eq!(result.restored_count, 1);
        assert_eq!(
            fs::read(root.join("2026/09/04/rollout-one.jsonl")).unwrap(),
            payload
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn skips_existing_destination_without_overwrite() {
        let root = restore_fixture_root("conflict");
        let archive_path = root.join("fixture.zip");
        let payload = valid_jsonl("one");
        let entries = [("one", payload.as_slice())];
        let manifest_text = restore_manifest(&entries);
        write_fixture(
            &archive_path,
            &manifest_text,
            &[("sessions/2026/09/04/rollout-one.jsonl", payload.as_slice())],
        );
        let manifest: ArchiveManifest = serde_json::from_str(&manifest_text).unwrap();
        let destination = root.join("2026/09/04/rollout-one.jsonl");
        fs::create_dir_all(destination.parent().unwrap()).unwrap();
        fs::write(&destination, b"existing").unwrap();
        let result = restore_from_path_at_root(
            &no_safety_configuration(),
            &archive_path,
            &manifest,
            &restore_preview(&manifest, &root),
            &root,
            None,
        )
        .unwrap();
        assert_eq!((result.restored_count, result.skipped_conflicts), (0, 1));
        assert_eq!(fs::read(destination).unwrap(), b"existing");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn invalid_archive_cannot_reach_restore() {
        let path = fixture_path("invalid-restore");
        write_fixture(&path, "{", &[]);
        assert!(!inspect(&path).unwrap().validation.valid);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn restore_rolls_back_all_created_files_after_mid_copy_failure() {
        let root = restore_fixture_root("rollback");
        let archive_path = root.join("fixture.zip");
        let one = valid_jsonl("one");
        let two = valid_jsonl("two");
        let entries = [("one", one.as_slice()), ("two", two.as_slice())];
        let manifest_text = restore_manifest(&entries);
        write_fixture(
            &archive_path,
            &manifest_text,
            &[
                ("sessions/2026/09/04/rollout-one.jsonl", one.as_slice()),
                ("sessions/2026/09/04/rollout-two.jsonl", two.as_slice()),
            ],
        );
        let manifest: ArchiveManifest = serde_json::from_str(&manifest_text).unwrap();
        let result = restore_from_path_at_root(
            &no_safety_configuration(),
            &archive_path,
            &manifest,
            &restore_preview(&manifest, &root),
            &root,
            Some(1),
        );
        assert!(result.is_err());
        assert!(!root.join("2026/09/04/rollout-one.jsonl").exists());
        assert!(!root.join("2026/09/04/rollout-two.jsonl").exists());
        let _ = fs::remove_dir_all(root);
    }
}
