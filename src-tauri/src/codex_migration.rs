//! Multi-account Codex folder migration. Credentials and runtime caches never enter this ZIP.
use crate::{environment, fs_safety, platform};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    cmp::Reverse,
    collections::{BTreeMap, HashMap, HashSet},
    fs::{self, File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    process::Command,
    sync::{
        atomic::{AtomicU64, Ordering},
        Mutex, OnceLock,
    },
    time::UNIX_EPOCH,
};
use zip::{write::SimpleFileOptions, CompressionMethod, ZipArchive, ZipWriter};

type Result<T> = std::result::Result<T, String>;
const LIMIT: u64 = 1024 * 1024 * 1024;
const GROUPS: &[&str] = &[
    "chat",
    "settings",
    "skills",
    "pets",
    "projects",
    "state",
    "worktrees",
    "plugins",
    "visualizations",
];

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Account {
    pub id: String,
    pub label: String,
    pub folder: String,
    #[serde(skip)]
    pub home: PathBuf,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Entry {
    pub account_id: String,
    pub relative_path: String,
    pub archive_path: String,
    pub group: String,
    pub bytes: u64,
    pub sha256: String,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Group {
    pub id: String,
    pub files: usize,
    pub bytes: u64,
    pub reason: String,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountEstimate {
    pub account_id: String,
    pub groups: Vec<Group>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Manifest {
    format_version: u32,
    kind: String,
    created_at: String,
    platform: String,
    accounts: Vec<Account>,
    entries: Vec<Entry>,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Overview {
    pub accounts: Vec<Account>,
    pub estimates: Vec<AccountEstimate>,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Preview {
    pub token: String,
    pub accounts: Vec<Account>,
    pub groups: Vec<Group>,
    pub excluded: Vec<Group>,
    pub entries: Vec<Entry>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Created {
    pub archive_path: String,
    pub archive_bytes: u64,
    pub files: usize,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Archive {
    pub name: String,
    pub bytes: u64,
    pub modified_at: Option<u64>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Archives {
    pub directory: String,
    pub archives: Vec<Archive>,
    pub total_bytes: u64,
}
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RestoreItem {
    pub account_label: String,
    pub path: String,
    pub archive_path: String,
    pub status: String,
    pub bytes: u64,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestorePreview {
    pub token: String,
    pub items: Vec<RestoreItem>,
}
#[derive(Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RestoreProgress {
    pub stage: String,
    pub completed: usize,
    pub total: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bytes_copied: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_bytes: Option<u64>,
}
fn report(progress: &dyn Fn(RestoreProgress), stage: &str, completed: usize, total: usize) {
    progress(RestoreProgress {
        stage: stage.into(),
        completed,
        total,
        file: None,
        operation: None,
        ..Default::default()
    });
}
fn report_file(
    progress: &dyn Fn(RestoreProgress),
    completed: usize,
    total: usize,
    file: &str,
    operation: &str,
) {
    report_file_bytes(progress, completed, total, file, operation, None);
}
fn report_file_bytes(
    progress: &dyn Fn(RestoreProgress),
    completed: usize,
    total: usize,
    file: &str,
    operation: &str,
    bytes: Option<(u64, u64)>,
) {
    progress(RestoreProgress {
        stage: "restoring".into(),
        completed,
        total,
        file: Some(file.into()),
        operation: Some(operation.into()),
        bytes_copied: bytes.map(|value| value.0),
        file_bytes: bytes.map(|value| value.1),
    });
}
fn copy_with_progress(
    source: &mut impl Read,
    destination: &mut impl Write,
    progress: &dyn Fn(&str, u64),
) -> Result<u64> {
    let mut buffer = [0u8; 65_536];
    let mut copied = 0;
    loop {
        progress("copy-read", copied);
        let count = match source.read(&mut buffer) {
            Ok(count) => count,
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(format!("Backup read failed: {error}")),
        };
        if count == 0 {
            return Ok(copied);
        }
        progress("copy-write", copied);
        destination
            .write_all(&buffer[..count])
            .map_err(|error| format!("Destination write failed: {error}"))?;
        copied += count as u64;
        progress("copy-file", copied);
    }
}
struct RestoreTrace {
    sender: std::sync::mpsc::SyncSender<Vec<u8>>,
    dropped: std::cell::Cell<usize>,
    // Dropping a JoinHandle detaches only this diagnostic writer, never restore I/O.
    _worker: std::thread::JoinHandle<()>,
}
fn start_trace_writer(
    writer: impl Write + Send + 'static,
    capacity: usize,
) -> Result<RestoreTrace> {
    let (sender, receiver) = std::sync::mpsc::sync_channel::<Vec<u8>>(capacity);
    let worker = std::thread::Builder::new()
        .name("codex-restore-log".into())
        .spawn(move || {
            let mut writer = writer;
            for record in receiver {
                if writer.write_all(&record).is_err() {
                    break;
                }
            }
        })
        .map_err(|error| format!("Cannot start restore diagnostic writer: {error}"))?;
    Ok(RestoreTrace {
        sender,
        dropped: std::cell::Cell::new(0),
        _worker: worker,
    })
}
fn create_restore_trace(directory: &Path) -> Result<RestoreTrace> {
    fs_safety::check(directory)?;
    fs::create_dir_all(directory).map_err(|error| error.to_string())?;
    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(directory.join(format!(
            "codex-restore-{}.jsonl",
            time::OffsetDateTime::now_utc().unix_timestamp_nanos()
        )))
        .map_err(|error| format!("Cannot create restore diagnostic log: {error}"))?;
    // ponytail: one bounded best-effort queue per restore; log gaps instead of delaying data.
    start_trace_writer(file, 1024)
}
fn trace_progress(
    trace: &RestoreTrace,
    event: &RestoreProgress,
    callback_returned: bool,
) -> Result<()> {
    let mut record = serde_json::to_vec(&serde_json::json!({
        "timestampMs": time::OffsetDateTime::now_utc().unix_timestamp_nanos() / 1_000_000,
        "progress": event,
        "notificationCallbackReturned": callback_returned,
        "droppedRecords": trace.dropped.get(),
    }))
    .map_err(|error| error.to_string())?;
    record.push(b'\n');
    match trace.sender.try_send(record) {
        Ok(()) => trace.dropped.set(0),
        Err(_) => trace.dropped.set(trace.dropped.get().saturating_add(1)),
    }
    Ok(())
}
fn notify_progress(
    trace: &RestoreTrace,
    event: RestoreProgress,
    progress: &dyn Fn(RestoreProgress),
) {
    // A last pre-notification record alone cannot attribute a stall to filesystem I/O.
    let _ = trace_progress(trace, &event, false);
    progress(event.clone());
    let _ = trace_progress(trace, &event, true);
}
#[derive(Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Restored {
    pub restored: usize,
    pub skipped: usize,
    pub errors: Vec<String>,
    pub rollback_remaining: usize,
    pub safety_archive: Option<String>,
}

#[derive(Clone)]
struct SafetyEntry {
    target: PathBuf,
    entry: Entry,
}

#[derive(Clone)]
enum Plan {
    Snapshot(Vec<Account>, Vec<String>, Vec<Entry>),
    Archive(PathBuf, String, Manifest),
    Restore(PathBuf, String, Vec<RestoreItem>, Manifest),
}
static PLANS: OnceLock<Mutex<HashMap<String, Plan>>> = OnceLock::new();
static NEXT: AtomicU64 = AtomicU64::new(1);
fn put(plan: Plan) -> String {
    let token = format!("codex-migration-{}", NEXT.fetch_add(1, Ordering::Relaxed));
    let mut plans = PLANS.get_or_init(Default::default).lock().unwrap();
    if plans.len() >= 32 {
        plans.clear();
    }
    plans.insert(token.clone(), plan);
    token
}
fn get(token: &str) -> Result<Plan> {
    PLANS
        .get_or_init(Default::default)
        .lock()
        .unwrap()
        .get(token)
        .cloned()
        .ok_or("Preview expired; scan again.".into())
}
fn hash_file(path: &Path) -> Result<String> {
    let mut file = environment::locked_read(path)?;
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 65_536];
    loop {
        let n = file.read(&mut buffer).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        hash.update(&buffer[..n]);
    }
    Ok(format!("{:x}", hash.finalize()))
}
fn create_output(path: &Path) -> Result<File> {
    let mut options = OpenOptions::new();
    options.read(true).write(true).create_new(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.share_mode(1); // Pin this new file against competing writes/deletes until verified.
    }
    options.open(path).map_err(|error| error.to_string())
}
fn verify_output(file: &mut File, entry: &Entry, progress: &dyn Fn(&str)) -> Result<()> {
    progress("verify-size");
    if file.metadata().map_err(|error| error.to_string())?.len() != entry.bytes {
        return Err("Restored file size mismatch.".into());
    }
    progress("verify-seek");
    file.seek(SeekFrom::Start(0))
        .map_err(|error| error.to_string())?;
    progress("verify-read");
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 65_536];
    let mut remaining = entry.bytes;
    while remaining > 0 {
        let amount = remaining.min(buffer.len() as u64) as usize;
        let read = file
            .read(&mut buffer[..amount])
            .map_err(|error| error.to_string())?;
        if read == 0 {
            return Err("Restored file ended before the expected size.".into());
        }
        hash.update(&buffer[..read]);
        remaining -= read as u64;
    }
    progress("verify-digest");
    if format!("{:x}", hash.finalize()) != entry.sha256 {
        return Err("Restored file verification failed.".into());
    }
    Ok(())
}
fn home_dir() -> Result<PathBuf> {
    dirs::home_dir().ok_or("Could not resolve the current user home directory.".into())
}
fn normal(path: &Path) -> String {
    path.to_string_lossy()
        .trim_end_matches(['\\', '/'])
        .to_ascii_lowercase()
}
fn valid_folder(folder: &str) -> bool {
    folder == ".codex"
        || folder.strip_prefix(".codex-").is_some_and(|suffix| {
            !suffix.is_empty()
                && suffix
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
        })
}
fn account(folder: String, label: String, home: PathBuf) -> Result<Account> {
    if !valid_folder(&folder) {
        return Err("Only direct .codex or .codex-* folders are supported.".into());
    }
    fs_safety::check(&home)?;
    Ok(Account {
        id: folder.to_ascii_lowercase(),
        folder,
        label,
        home,
    })
}

pub fn overview() -> Result<Overview> {
    let user_home = home_dir()?;
    let mut candidates: BTreeMap<String, (PathBuf, String)> = BTreeMap::new();
    let mut add = |path: PathBuf, label: String| {
        let Some(folder) = path.file_name().and_then(|name| name.to_str()) else {
            return;
        };
        if valid_folder(folder) && path.is_dir() && fs_safety::check(&path).is_ok() {
            candidates.entry(normal(&path)).or_insert((path, label));
        }
    };
    add(platform::codex_home(), "Active CODEX_HOME".into());
    if let Ok(read) = fs::read_dir(&user_home) {
        for entry in read.flatten() {
            let path = entry.path();
            let folder = path
                .file_name()
                .and_then(|name| name.to_str())
                .map(str::to_owned);
            if let Some(folder) = folder {
                if valid_folder(&folder) {
                    add(path, format!("Detected {folder}"));
                }
            }
        }
    }
    if let Ok(config) = crate::config::load() {
        for environment in config.codex_environments {
            add(
                PathBuf::from(&environment.codex_home),
                environment.display_name,
            );
        }
    }
    let mut accounts = candidates
        .into_values()
        .filter_map(|(path, label)| {
            let folder = path.file_name()?.to_str()?.to_owned();
            account(folder, label, path).ok()
        })
        .collect::<Vec<_>>();
    accounts.sort_by_key(|account| (account.folder != ".codex", account.folder.clone()));
    let estimates = estimate_accounts(&accounts)?;
    Ok(Overview {
        accounts,
        estimates,
    })
}
fn backup_directory() -> Result<PathBuf> {
    let config = crate::config::load().map_err(|error| error.to_string())?;
    let directory = platform::backup_dir(&config);
    fs_safety::check(&directory)?;
    Ok(directory)
}
fn archive_name(name: &str) -> bool {
    name.strip_prefix("codex-migration-")
        .and_then(|value| value.strip_suffix(".zip"))
        .is_some_and(|stamp| !stamp.is_empty() && stamp.bytes().all(|byte| byte.is_ascii_digit()))
}
fn archive_path_at(directory: &Path, name: &str) -> Result<PathBuf> {
    if !archive_name(name) {
        return Err("Unknown Codex migration backup.".into());
    }
    fs_safety::check(directory)?;
    let path = directory.join(name);
    fs_safety::check(&path)?;
    let metadata = fs::symlink_metadata(&path).map_err(|_| "Backup is unavailable.")?;
    if fs_safety::linked(&metadata) || !metadata.is_file() {
        return Err("Backup is not a regular file.".into());
    }
    Ok(path)
}
fn list_archives_at(directory: &Path) -> Result<Archives> {
    fs_safety::check(directory)?;
    if !directory.exists() {
        return Ok(Archives {
            directory: directory.to_string_lossy().into(),
            archives: Vec::new(),
            total_bytes: 0,
        });
    }
    let mut archives = Vec::new();
    for entry in fs::read_dir(directory).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if !archive_name(&name) {
            continue;
        }
        let metadata = fs::symlink_metadata(entry.path()).map_err(|error| error.to_string())?;
        if fs_safety::linked(&metadata) || !metadata.is_file() {
            continue;
        }
        archives.push(Archive {
            name,
            bytes: metadata.len(),
            modified_at: metadata
                .modified()
                .ok()
                .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                .map(|duration| duration.as_secs()),
        });
    }
    archives.sort_by_key(|archive| Reverse(archive.modified_at.unwrap_or_default()));
    let total_bytes = archives.iter().map(|archive| archive.bytes).sum();
    Ok(Archives {
        directory: directory.to_string_lossy().into(),
        archives,
        total_bytes,
    })
}
pub fn list_archives() -> Result<Archives> {
    list_archives_at(&backup_directory()?)
}
pub fn open_archive(name: &str) -> Result<()> {
    let path = archive_path_at(&backup_directory()?, name)?;
    Command::new("explorer.exe")
        .arg(format!("/select,{}", path.display()))
        .spawn()
        .map(|_| ())
        .map_err(|_| "Cannot open the migration backup folder.".into())
}
fn delete_archive_at(directory: &Path, name: &str) -> Result<()> {
    let path = archive_path_at(directory, name)?;
    fs::remove_file(path).map_err(|_| "Cannot remove the migration backup.")?;
    Ok(())
}
pub fn delete_archive(name: &str) -> Result<()> {
    delete_archive_at(&backup_directory()?, name)
}
pub fn inspect_archive(name: &str) -> Result<Preview> {
    inspect(archive_path_at(&backup_directory()?, name)?)
}

fn protected(path: &str) -> bool {
    path.split('/').any(|part| {
        matches!(
            part.to_ascii_lowercase().as_str(),
            "auth.json" | "cap_sid" | "installation_id"
        )
    })
}
fn runtime(path: &str) -> bool {
    path.split('/').any(|part| {
        matches!(
            part.to_ascii_lowercase().as_str(),
            "cache"
                | ".tmp"
                | "tmp"
                | ".sandbox"
                | ".sandbox-bin"
                | ".sandbox-secrets"
                | "vendor_imports"
                | "node_modules"
                | "thread-writer-locks"
        )
    }) || path.split('/').next().is_some_and(|root| {
        matches!(
            root,
            "app-server-control"
                | "app-server-daemon"
                | "ambient-suggestions"
                | "packages"
                | "tui-thread-reference-capabilities"
                | ".sandbox_migration"
        )
    }) || path.split('/').next_back().is_some_and(|name| {
        let name = name.to_ascii_lowercase();
        name.ends_with(".tmp")
            || name.contains(".tmp-")
            || matches!(
                name.as_str(),
                ".sqlite-maintenance.lock"
                    | "models_cache.json"
                    | "version.json"
                    | ".sandbox_migration"
            )
    })
}
fn classify(path: &str) -> Option<&'static str> {
    if protected(path)
        || runtime(path)
        || path
            .split('/')
            .any(|part| matches!(part, "backups" | "quarantine" | ".git"))
    {
        return None;
    }
    if path.starts_with(".chatgpt-projects/") {
        return Some("projects");
    }
    if path.starts_with("sessions/")
        || path.starts_with("archived_sessions/")
        || path.starts_with("attachments/")
        || path == "session_index.jsonl"
        || (path.split('/').count() == 1
            && (path.ends_with(".sqlite") || path.contains(".sqlite-"))
            && path != "logs_2.sqlite")
    {
        return Some("chat");
    }
    if matches!(path, "config.toml" | "AGENTS.md" | "hooks.json")
        || path.starts_with("rules/")
        || path.starts_with("prompts/")
    {
        return Some("settings");
    }
    if path.starts_with("skills/") {
        return Some("skills");
    }
    if path.starts_with("pets/") {
        return Some("pets");
    }
    if path.starts_with("worktrees/") {
        return Some("worktrees");
    }
    if path.starts_with("plugins/") {
        return Some("plugins");
    }
    if path.starts_with("visualizations/") {
        return Some("visualizations");
    }
    Some("state")
}
fn excluded_reason(path: &str) -> &'static str {
    if protected(path) {
        "Authentication or machine identity: sign in again"
    } else if runtime(path) {
        "Runtime/cache data: regenerated on the new machine"
    } else if path
        .split('/')
        .any(|part| matches!(part, "backups" | "quarantine" | ".git"))
    {
        "Recovery artifacts or repository metadata are not copied"
    } else {
        "Outside supported Codex migration scope"
    }
}
fn safe_settings(path: &Path) -> Result<bool> {
    let bytes = fs::read(path).map_err(|e| e.to_string())?;
    let lower = String::from_utf8_lossy(&bytes).to_ascii_lowercase();
    Ok(![
        "api_key",
        "bearer_token",
        "authorization",
        "password",
        "secret",
        "access_token",
    ]
    .iter()
    .any(|needle| lower.contains(needle)))
}
fn walk(root: &Path, dir: &Path, files: &mut Vec<String>, skipped_links: &mut usize) -> Result<()> {
    fs_safety::check(dir)?;
    for entry in fs::read_dir(dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path).map_err(|e| e.to_string())?;
        if fs_safety::linked(&metadata) {
            *skipped_links += 1;
            continue;
        }
        if metadata.is_dir() {
            walk(root, &path, files, skipped_links)?;
        } else if metadata.is_file() {
            files.push(
                path.strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/"),
            );
        }
    }
    Ok(())
}
fn walk_for_estimate(root: &Path, dir: &Path, files: &mut Vec<String>) -> Result<()> {
    fs_safety::check(dir)?;
    for entry in fs::read_dir(dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path).map_err(|e| e.to_string())?;
        if fs_safety::linked(&metadata) {
            continue;
        }
        if metadata.is_dir() {
            walk_for_estimate(root, &path, files)?;
        } else if metadata.is_file() {
            files.push(
                path.strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/"),
            );
        }
    }
    Ok(())
}
fn validate_selection(
    accounts: &[Account],
    account_ids: &[String],
    groups: &[String],
) -> Result<Vec<Account>> {
    if groups.is_empty()
        || groups.iter().any(|group| !GROUPS.contains(&group.as_str()))
        || groups.iter().collect::<HashSet<_>>().len() != groups.len()
    {
        return Err("Select valid component groups.".into());
    }
    if account_ids.is_empty()
        || account_ids.iter().collect::<HashSet<_>>().len() != account_ids.len()
    {
        return Err("Select one or more detected Codex accounts.".into());
    }
    let selected = account_ids
        .iter()
        .map(|id| {
            accounts
                .iter()
                .find(|account| &account.id == id)
                .cloned()
                .ok_or_else(|| "Selected Codex account is unavailable.".to_owned())
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(selected)
}
fn estimate_accounts(accounts: &[Account]) -> Result<Vec<AccountEstimate>> {
    accounts
        .iter()
        .map(|account| {
            let mut totals: HashMap<&str, (usize, u64)> = HashMap::new();
            let mut files = Vec::new();
            walk_for_estimate(&account.home, &account.home, &mut files)?;
            for relative_path in files {
                let Some(group) = classify(&relative_path) else {
                    continue;
                };
                let source = account.home.join(&relative_path);
                if group == "settings" && !safe_settings(&source)? {
                    continue;
                }
                let bytes = fs::metadata(source).map_err(|e| e.to_string())?.len();
                let total = totals.entry(group).or_default();
                total.0 += 1;
                total.1 += bytes;
            }
            Ok(AccountEstimate {
                account_id: account.id.clone(),
                groups: GROUPS
                    .iter()
                    .map(|id| {
                        let (files, bytes) = totals.get(*id).copied().unwrap_or_default();
                        Group {
                            id: (*id).into(),
                            files,
                            bytes,
                            reason: "Estimated current source size".into(),
                        }
                    })
                    .collect(),
            })
        })
        .collect()
}
fn inventory(accounts: &[Account], groups: &[String]) -> Result<(Vec<Entry>, Vec<Group>)> {
    let mut entries = Vec::new();
    let mut excluded: HashMap<String, Group> = HashMap::new();
    let mut skipped_links = 0;
    for account in accounts {
        let mut files = Vec::new();
        walk(&account.home, &account.home, &mut files, &mut skipped_links)?;
        files.sort();
        for relative_path in files {
            fs_safety::relative(&relative_path)?;
            let source = account.home.join(&relative_path);
            let bytes = fs::metadata(&source).map_err(|e| e.to_string())?.len();
            if bytes > LIMIT {
                return Err(format!("File exceeds 1 GiB backup limit: {relative_path}"));
            }
            let Some(group) = classify(&relative_path) else {
                let reason = excluded_reason(&relative_path).to_owned();
                let item = excluded.entry(reason.clone()).or_insert(Group {
                    id: reason.clone(),
                    files: 0,
                    bytes: 0,
                    reason,
                });
                item.files += 1;
                item.bytes += bytes;
                continue;
            };
            if !groups.iter().any(|selected| selected == group) {
                let reason = "Component not selected".to_owned();
                let item = excluded.entry(reason.clone()).or_insert(Group {
                    id: reason.clone(),
                    files: 0,
                    bytes: 0,
                    reason,
                });
                item.files += 1;
                item.bytes += bytes;
                continue;
            }
            if group == "settings" && !safe_settings(&source)? {
                let reason = "Settings may contain credentials: omitted intact; configure manually"
                    .to_owned();
                let item = excluded.entry(reason.clone()).or_insert(Group {
                    id: reason.clone(),
                    files: 0,
                    bytes: 0,
                    reason,
                });
                item.files += 1;
                item.bytes += bytes;
                continue;
            }
            entries.push(Entry {
                account_id: account.id.clone(),
                archive_path: format!("accounts/{}/{relative_path}", account.folder),
                relative_path,
                group: group.into(),
                bytes,
                sha256: hash_file(&source)?,
            });
        }
    }
    if skipped_links > 0 {
        let reason = "Link or junction excluded for safe backup".to_owned();
        excluded.insert(
            reason.clone(),
            Group {
                id: reason.clone(),
                files: skipped_links,
                bytes: 0,
                reason,
            },
        );
    }
    entries.sort_by(|a, b| a.archive_path.cmp(&b.archive_path));
    let mut excluded = excluded.into_values().collect::<Vec<_>>();
    excluded.sort_by(|a, b| a.id.cmp(&b.id));
    Ok((entries, excluded))
}
pub fn preview(account_ids: Vec<String>, groups: Vec<String>) -> Result<Preview> {
    environment::require_closed()?;
    let all = overview()?.accounts;
    let accounts = validate_selection(&all, &account_ids, &groups)?;
    let (entries, excluded) = inventory(&accounts, &groups)?;
    let summaries = groups
        .iter()
        .map(|id| Group {
            id: id.clone(),
            files: entries.iter().filter(|entry| &entry.group == id).count(),
            bytes: entries
                .iter()
                .filter(|entry| &entry.group == id)
                .map(|entry| entry.bytes)
                .sum(),
            reason: "Selected component".into(),
        })
        .collect();
    Ok(Preview {
        token: put(Plan::Snapshot(accounts.clone(), groups, entries.clone())),
        accounts,
        groups: summaries,
        excluded,
        entries,
    })
}
pub fn create(token: &str) -> Result<Created> {
    environment::require_closed()?;
    let Plan::Snapshot(accounts, groups, expected) = get(token)? else {
        return Err("Invalid backup preview.".into());
    };
    let (entries, _) = inventory(&accounts, &groups)?;
    if entries != expected {
        return Err("Source changed; review the backup again.".into());
    }
    let config = crate::config::load().map_err(|error| error.to_string())?;
    create_at(&accounts, &platform::backup_dir(&config), entries, &groups)
}
fn copy_and_hash(input: &mut File, output: &mut ZipWriter<File>) -> Result<String> {
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 65_536];
    loop {
        let count = input.read(&mut buffer).map_err(|e| e.to_string())?;
        if count == 0 {
            break;
        }
        hash.update(&buffer[..count]);
        output
            .write_all(&buffer[..count])
            .map_err(|e| e.to_string())?;
    }
    Ok(format!("{:x}", hash.finalize()))
}
fn create_at(
    accounts: &[Account],
    output: &Path,
    entries: Vec<Entry>,
    groups: &[String],
) -> Result<Created> {
    fs_safety::check(output)?;
    fs::create_dir_all(output).map_err(|e| e.to_string())?;
    if accounts
        .iter()
        .any(|account| output.starts_with(&account.home))
    {
        return Err("Backup output must be outside every selected Codex account.".into());
    }
    let stamp = time::OffsetDateTime::now_utc().unix_timestamp_nanos();
    let path = output.join(format!("codex-migration-{stamp}.zip"));
    let file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&path)
        .map_err(|e| e.to_string())?;
    let result = (|| -> Result<()> {
        let manifest = Manifest {
            format_version: 1,
            kind: "dev-companion-codex-migration".into(),
            created_at: time::OffsetDateTime::now_utc()
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap(),
            platform: "windows".into(),
            accounts: accounts.to_vec(),
            entries: entries.clone(),
        };
        let mut zip = ZipWriter::new(file);
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        zip.start_file("codex-migration-manifest.json", options)
            .map_err(|e| e.to_string())?;
        zip.write_all(&serde_json::to_vec_pretty(&manifest).unwrap())
            .map_err(|e| e.to_string())?;
        for entry in &entries {
            let account = accounts
                .iter()
                .find(|account| account.id == entry.account_id)
                .ok_or("Backup account changed.")?;
            let mut input = environment::locked_read(&account.home.join(&entry.relative_path))?;
            zip.start_file(&entry.archive_path, options)
                .map_err(|e| e.to_string())?;
            if copy_and_hash(&mut input, &mut zip)? != entry.sha256 {
                return Err("Source changed; close Codex and review the backup again.".into());
            }
        }
        zip.finish()
            .map_err(|e| e.to_string())?
            .sync_all()
            .map_err(|e| e.to_string())?;
        if inventory(accounts, groups)?.0 != entries {
            return Err(
                "Source inventory changed; close Codex and review the backup again.".into(),
            );
        }
        inspect_at(&path)?;
        Ok(())
    })();
    if let Err(error) = result {
        let _ = fs::remove_file(&path);
        return Err(error);
    }
    Ok(Created {
        archive_bytes: fs::metadata(&path).map_err(|e| e.to_string())?.len(),
        archive_path: path.to_string_lossy().into(),
        files: entries.len(),
    })
}
fn inspect_at(path: &Path) -> Result<Manifest> {
    inspect_at_with_progress(path, &|_, _| {})
}
fn inspect_at_with_progress(path: &Path, progress: &dyn Fn(usize, usize)) -> Result<Manifest> {
    progress(0, 0);
    let mut zip = ZipArchive::new(environment::locked_read(path)?).map_err(|e| e.to_string())?;
    let mut names = HashSet::new();
    for index in 0..zip.len() {
        let file = zip.by_index(index).map_err(|e| e.to_string())?;
        fs_safety::relative(file.name())?;
        if file.is_dir()
            || file.size() > LIMIT
            || !names.insert(file.name().to_ascii_lowercase())
            || file
                .unix_mode()
                .is_some_and(|mode| mode & 0o170000 == 0o120000)
        {
            return Err("Invalid archive entry.".into());
        }
    }
    let mut raw = Vec::new();
    zip.by_name("codex-migration-manifest.json")
        .map_err(|e| e.to_string())?
        .take(4 * 1024 * 1024)
        .read_to_end(&mut raw)
        .map_err(|e| e.to_string())?;
    let manifest: Manifest = serde_json::from_slice(&raw).map_err(|e| e.to_string())?;
    if manifest.format_version != 1
        || manifest.kind != "dev-companion-codex-migration"
        || manifest.platform != "windows"
        || manifest.entries.len() + 1 != names.len()
        || manifest.accounts.is_empty()
    {
        return Err("Unsupported Codex migration manifest.".into());
    }
    let mut accounts = HashSet::new();
    for account in &manifest.accounts {
        if !valid_folder(&account.folder)
            || account.id != account.folder.to_ascii_lowercase()
            || !accounts.insert(account.id.clone())
        {
            return Err("Invalid Codex account manifest.".into());
        }
    }
    let mut listed = HashSet::new();
    progress(0, manifest.entries.len());
    for (index, entry) in manifest.entries.iter().enumerate() {
        let account = manifest
            .accounts
            .iter()
            .find(|account| account.id == entry.account_id)
            .ok_or("Unknown account in archive entry.")?;
        fs_safety::relative(&entry.relative_path)?;
        if !GROUPS.contains(&entry.group.as_str())
            || (classify(&entry.relative_path) != Some(entry.group.as_str())
                && !(entry.group == "state" && runtime(&entry.relative_path)))
            || entry.archive_path
                != format!("accounts/{}/{},", account.folder, entry.relative_path)
                    .trim_end_matches(',')
            || !listed.insert(entry.archive_path.to_ascii_lowercase())
        {
            return Err("Invalid migration component policy.".into());
        }
        let mut file = zip
            .by_name(&entry.archive_path)
            .map_err(|e| e.to_string())?;
        if file.size() != entry.bytes {
            return Err("Archive size mismatch.".into());
        }
        let mut hash = Sha256::new();
        let mut buffer = [0u8; 65_536];
        loop {
            let count = file.read(&mut buffer).map_err(|e| e.to_string())?;
            if count == 0 {
                break;
            }
            hash.update(&buffer[..count]);
        }
        if format!("{:x}", hash.finalize()) != entry.sha256 {
            return Err("Archive SHA-256 mismatch.".into());
        }
        progress(index + 1, manifest.entries.len());
    }
    Ok(manifest)
}
fn archive_groups(entries: &[Entry]) -> Vec<Group> {
    GROUPS
        .iter()
        .map(|id| Group {
            id: (*id).into(),
            files: entries.iter().filter(|entry| entry.group == *id).count(),
            bytes: entries
                .iter()
                .filter(|entry| entry.group == *id)
                .map(|entry| entry.bytes)
                .sum(),
            reason: "Archived component".into(),
        })
        .collect()
}
pub fn inspect(path: PathBuf) -> Result<Preview> {
    let _archive_lock = environment::locked_read(&path)?;
    let manifest = inspect_at(&path)?;
    let hash = hash_file(&path)?;
    Ok(Preview {
        token: put(Plan::Archive(path, hash, manifest.clone())),
        accounts: manifest.accounts,
        groups: archive_groups(&manifest.entries),
        excluded: Vec::new(),
        entries: manifest.entries,
    })
}
fn restore_items(root: &Path, manifest: &Manifest) -> Result<Vec<RestoreItem>> {
    restore_items_with_progress(root, manifest, &|_, _| {})
}
fn restore_items_with_progress(
    root: &Path,
    manifest: &Manifest,
    progress: &dyn Fn(usize, usize),
) -> Result<Vec<RestoreItem>> {
    progress(0, manifest.entries.len());
    manifest
        .entries
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            let account = manifest
                .accounts
                .iter()
                .find(|account| account.id == entry.account_id)
                .unwrap();
            let target = root.join(&account.folder).join(&entry.relative_path);
            fs_safety::check(&target)?;
            let status = match fs::symlink_metadata(&target) {
                Ok(metadata) if metadata.is_file() => {
                    if hash_file(&target)? == entry.sha256 {
                        "identical"
                    } else {
                        "conflict"
                    }
                }
                Ok(_) => "conflict",
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => "new",
                Err(error) => return Err(error.to_string()),
            };
            progress(index + 1, manifest.entries.len());
            Ok(RestoreItem {
                account_label: account.label.clone(),
                path: format!(
                    "{}\\{}",
                    account.folder,
                    entry.relative_path.replace('/', "\\")
                ),
                archive_path: entry.archive_path.clone(),
                status: status.into(),
                bytes: entry.bytes,
            })
        })
        .collect()
}
fn walk_chat_files(root: &Path, dir: &Path, files: &mut Vec<String>) -> Result<()> {
    fs_safety::check(dir)?;
    for entry in fs::read_dir(dir).map_err(|error| error.to_string())? {
        let path = entry.map_err(|error| error.to_string())?.path();
        let relative = path
            .strip_prefix(root)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        // Prune excluded subtrees before touching their contents.
        if classify(&relative) != Some("chat") {
            continue;
        }
        let metadata = fs::symlink_metadata(&path).map_err(|error| error.to_string())?;
        if fs_safety::linked(&metadata) {
            continue;
        }
        if metadata.is_dir() {
            walk_chat_files(root, &path, files)?;
        } else if metadata.is_file() {
            files.push(relative);
        }
    }
    Ok(())
}
#[cfg(test)]
fn chat_safety_entries(root: &Path, manifest: &Manifest) -> Result<Vec<SafetyEntry>> {
    chat_safety_entries_with_progress(root, manifest, &|_| {})
}
fn chat_safety_entries_with_progress(
    root: &Path,
    manifest: &Manifest,
    progress: &dyn Fn(RestoreProgress),
) -> Result<Vec<SafetyEntry>> {
    report(progress, "safety-scan", 0, 0);
    let mut entries = Vec::new();
    for account in &manifest.accounts {
        let account_home = root.join(&account.folder);
        if !account_home.exists() {
            continue;
        }
        fs_safety::check(&account_home)?;
        let archived: Vec<_> = manifest
            .entries
            .iter()
            .filter(|entry| entry.account_id == account.id && !runtime(&entry.relative_path))
            .collect();
        let mut files: BTreeMap<String, String> = BTreeMap::new();
        // Archive targets are known already; never walk unrelated skills/plugins/cache trees.
        for entry in &archived {
            let target = account_home.join(&entry.relative_path);
            fs_safety::check(&target)?;
            if target.is_file() {
                files.insert(
                    entry.relative_path.to_ascii_lowercase(),
                    entry.relative_path.clone(),
                );
            }
        }
        if archived.iter().any(|entry| entry.group == "chat") {
            for item in fs::read_dir(&account_home).map_err(|error| error.to_string())? {
                let item = item.map_err(|error| error.to_string())?;
                let name = item.file_name().to_string_lossy().into_owned();
                let chat_directory = matches!(
                    name.as_str(),
                    "sessions" | "archived_sessions" | "attachments"
                );
                if !chat_directory && classify(&name) != Some("chat") {
                    continue;
                }
                let path = item.path();
                let metadata = fs::symlink_metadata(&path).map_err(|error| error.to_string())?;
                if fs_safety::linked(&metadata) {
                    continue;
                }
                if metadata.is_dir() && chat_directory {
                    let mut chats = Vec::new();
                    walk_chat_files(&account_home, &path, &mut chats)?;
                    for relative in chats {
                        files
                            .entry(relative.to_ascii_lowercase())
                            .or_insert(relative);
                    }
                } else if metadata.is_file() {
                    files.entry(name.to_ascii_lowercase()).or_insert(name);
                }
            }
        }
        for relative_path in files.into_values() {
            let group = classify(&relative_path).ok_or("Unsafe safety snapshot file.")?;
            let target = account_home.join(&relative_path);
            let bytes = fs::metadata(&target)
                .map_err(|error| error.to_string())?
                .len();
            entries.push(SafetyEntry {
                target: target.clone(),
                entry: Entry {
                    account_id: account.id.clone(),
                    relative_path: relative_path.clone(),
                    archive_path: format!("accounts/{}/{relative_path}", account.folder),
                    group: group.into(),
                    bytes,
                    sha256: hash_file(&target)?,
                },
            });
            report(progress, "safety-scan", entries.len(), 0);
        }
    }
    entries.sort_by(|left, right| left.entry.archive_path.cmp(&right.entry.archive_path));
    Ok(entries)
}
fn verify_chat_safety_archive_with_progress(
    path: &Path,
    expected: &[SafetyEntry],
    progress: &dyn Fn(RestoreProgress),
) -> Result<()> {
    let manifest = inspect_at_with_progress(path, &|completed, total| {
        report(progress, "safety-validation", completed, total)
    })?;
    if manifest.entries
        != expected
            .iter()
            .map(|item| item.entry.clone())
            .collect::<Vec<_>>()
    {
        return Err("Safety archive manifest mismatch.".into());
    }
    Ok(())
}
#[cfg(test)]
fn create_chat_safety_archive(
    root: &Path,
    manifest: &Manifest,
    directory: &Path,
) -> Result<(Option<PathBuf>, Vec<SafetyEntry>)> {
    create_chat_safety_archive_with_progress(root, manifest, directory, &|_| {})
}
fn create_chat_safety_archive_with_progress(
    root: &Path,
    manifest: &Manifest,
    directory: &Path,
    progress: &dyn Fn(RestoreProgress),
) -> Result<(Option<PathBuf>, Vec<SafetyEntry>)> {
    let entries = chat_safety_entries_with_progress(root, manifest, progress)?;
    if entries.is_empty() {
        return Ok((None, entries));
    }
    fs_safety::check(directory)?;
    fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
    let path = directory.join(format!(
        "codex-safety-{}.zip",
        time::OffsetDateTime::now_utc().unix_timestamp_nanos()
    ));
    let file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&path)
        .map_err(|e| e.to_string())?;
    let result = (|| -> Result<()> {
        let mut zip = ZipWriter::new(file);
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
        let safety_manifest = Manifest {
            format_version: 1,
            kind: "dev-companion-codex-migration".into(),
            created_at: time::OffsetDateTime::now_utc()
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap(),
            platform: "windows".into(),
            accounts: manifest.accounts.clone(),
            entries: entries.iter().map(|item| item.entry.clone()).collect(),
        };
        zip.start_file("codex-migration-manifest.json", options)
            .map_err(|e| e.to_string())?;
        zip.write_all(&serde_json::to_vec_pretty(&safety_manifest).unwrap())
            .map_err(|e| e.to_string())?;
        report(progress, "safety-backup", 0, entries.len());
        for (index, item) in entries.iter().enumerate() {
            let mut input = environment::locked_read(&item.target)?;
            zip.start_file(&item.entry.archive_path, options)
                .map_err(|e| e.to_string())?;
            if copy_and_hash(&mut input, &mut zip)? != item.entry.sha256 {
                return Err("Destination changed; inspect again.".into());
            }
            report(progress, "safety-backup", index + 1, entries.len());
        }
        zip.finish()
            .map_err(|e| e.to_string())?
            .sync_all()
            .map_err(|e| e.to_string())?;
        verify_chat_safety_archive_with_progress(&path, &entries, progress)
    })();
    if let Err(error) = result {
        let _ = fs::remove_file(&path);
        return Err(error);
    }
    Ok((Some(path), entries))
}
fn restore_chat_safety(path: &Path, entries: &[SafetyEntry]) -> Result<usize> {
    let mut zip = ZipArchive::new(environment::locked_read(path)?).map_err(|e| e.to_string())?;
    let mut failed = 0;
    for item in entries {
        let attempt = (|| -> Result<()> {
            fs_safety::check(&item.target)?;
            fs::create_dir_all(item.target.parent().unwrap()).map_err(|e| e.to_string())?;
            let mut output = create_output(&item.target)?;
            copy_with_progress(
                &mut zip
                    .by_name(&item.entry.archive_path)
                    .map_err(|e| e.to_string())?,
                &mut output,
                &|_, _| {},
            )?;
            // Use normal writeback for destination files, as in restore below.
            // The verified safety ZIP remains the durable recovery copy.
            verify_output(&mut output, &item.entry, &|_| {})?;
            drop(output);
            Ok(())
        })();
        if attempt.is_err() {
            failed += 1;
        }
    }
    Ok(failed)
}
pub fn preview_restore(token: &str) -> Result<RestorePreview> {
    let Plan::Archive(path, hash, manifest) = get(token)? else {
        return Err("Inspect archive first.".into());
    };
    let _archive_lock = environment::locked_read(&path)?;
    if hash_file(&path)? != hash {
        return Err("Archive changed; inspect again.".into());
    }
    let items = restore_items(&home_dir()?, &manifest)?;
    Ok(RestorePreview {
        token: put(Plan::Restore(path, hash, items.clone(), manifest)),
        items,
    })
}
pub fn restore(
    token: &str,
    confirmation: &str,
    replace_chat: bool,
    progress: &dyn Fn(RestoreProgress),
) -> Result<Restored> {
    let expected_confirmation = if replace_chat {
        "REPLACE CODEX"
    } else {
        "RESTORE"
    };
    if confirmation != expected_confirmation {
        return Err(format!("Type {expected_confirmation} to confirm."));
    }
    report(progress, "checking-processes", 0, 0);
    environment::require_closed()?;
    let Plan::Restore(path, hash, items, manifest) = get(token)? else {
        return Err("Preview restore first.".into());
    };
    let trace = create_restore_trace(&platform::app_data_dir().join("logs"))?;
    let traced_progress = |event: RestoreProgress| {
        // Diagnostics must not interrupt an in-flight restore or rollback.
        notify_progress(&trace, event, progress);
    };
    let progress: &dyn Fn(RestoreProgress) = &traced_progress;
    report(progress, "archive-validation", 0, 0);
    let _archive_lock = environment::locked_read(&path)?;
    if hash_file(&path)? != hash {
        return Err("Archive changed; inspect again.".into());
    }
    report(progress, "archive-validation", 1, 1);
    restore_manifest_at(
        &home_dir()?,
        &path,
        &items,
        replace_chat,
        &backup_directory()?,
        None,
        progress,
        &manifest,
    )
}
#[cfg(test)]
fn restore_at(
    root: &Path,
    path: &Path,
    items: &[RestoreItem],
    replace_chat: bool,
    safety_directory: &Path,
    fail_after: Option<usize>,
) -> Result<Restored> {
    restore_at_with_progress(
        root,
        path,
        items,
        replace_chat,
        safety_directory,
        fail_after,
        &|_| {},
    )
}
#[cfg(test)]
fn restore_at_with_progress(
    root: &Path,
    path: &Path,
    items: &[RestoreItem],
    replace_chat: bool,
    safety_directory: &Path,
    fail_after: Option<usize>,
    progress: &dyn Fn(RestoreProgress),
) -> Result<Restored> {
    let manifest = inspect_at_with_progress(path, &|completed, total| {
        report(progress, "archive-validation", completed, total)
    })?;
    restore_manifest_at(
        root,
        path,
        items,
        replace_chat,
        safety_directory,
        fail_after,
        progress,
        &manifest,
    )
}
fn restore_manifest_at(
    root: &Path,
    path: &Path,
    items: &[RestoreItem],
    replace_chat: bool,
    safety_directory: &Path,
    fail_after: Option<usize>,
    progress: &dyn Fn(RestoreProgress),
    manifest: &Manifest,
) -> Result<Restored> {
    let current = restore_items_with_progress(root, &manifest, &|completed, total| {
        report(progress, "destination-check", completed, total)
    })?;
    if current != items {
        return Err("Destination changed; inspect again.".into());
    }
    if replace_chat && manifest.entries.is_empty() {
        return Err("This backup contains no Codex data to replace.".into());
    }
    let mut zip = ZipArchive::new(environment::locked_read(path)?).map_err(|e| e.to_string())?;
    let mut result = Restored::default();
    let mut created = Vec::new();
    let mut removed = Vec::new();
    let (safety_archive, safety_entries) = if replace_chat {
        create_chat_safety_archive_with_progress(root, &manifest, safety_directory, progress)?
    } else {
        (None, Vec::new())
    };
    result.safety_archive = safety_archive
        .as_ref()
        .map(|archive| archive.to_string_lossy().into_owned());
    let attempt = (|| -> Result<()> {
        if replace_chat {
            report(progress, "removing", 0, safety_entries.len());
            for (index, item) in safety_entries.iter().enumerate() {
                fs_safety::check(&item.target)?;
                fs::remove_file(&item.target).map_err(|e| e.to_string())?;
                removed.push(item.clone());
                report(progress, "removing", index + 1, safety_entries.len());
            }
        }
        report(progress, "restoring", 0, current.len());
        for (index, item) in current.iter().enumerate() {
            let entry = manifest
                .entries
                .iter()
                .find(|entry| entry.archive_path == item.archive_path)
                .unwrap();
            if runtime(&entry.relative_path) {
                result.skipped += 1;
                continue;
            }
            if !replace_chat && item.status != "new" {
                result.skipped += 1;
                continue;
            }
            if fail_after == Some(created.len()) {
                return Err("Injected restore failure.".into());
            }
            let account = manifest
                .accounts
                .iter()
                .find(|account| account.id == entry.account_id)
                .unwrap();
            let target = root.join(&account.folder).join(&entry.relative_path);
            report_file(progress, index, current.len(), &item.path, "prepare-target");
            let checked_path = |ancestor: &Path| {
                report_file(
                    progress,
                    index,
                    current.len(),
                    &ancestor.to_string_lossy(),
                    "check-path",
                );
            };
            report_file(progress, index, current.len(), &item.path, "check-target");
            fs_safety::check_with_progress(&target, &checked_path)?;
            report_file(progress, index, current.len(), &item.path, "create-parent");
            fs::create_dir_all(target.parent().unwrap()).map_err(|e| e.to_string())?;
            report_file(progress, index, current.len(), &item.path, "recheck-target");
            fs_safety::check_with_progress(&target, &checked_path)?;
            report_file(progress, index, current.len(), &item.path, "open-target");
            let mut output = create_output(&target)?;
            created.push(target.clone());
            report_file(progress, index, current.len(), &item.path, "read-archive");
            let mut source = zip.by_name(&item.archive_path).map_err(|e| e.to_string())?;
            report_file(progress, index, current.len(), &item.path, "copy-file");
            copy_with_progress(&mut source, &mut output, &|operation, copied| {
                report_file_bytes(
                    progress,
                    index,
                    current.len(),
                    &item.path,
                    operation,
                    Some((copied, entry.bytes)),
                );
            })?;
            // Per-file forced disk sync stalled on the target machine after copy.
            // Keep normal OS writeback and verify the pinned file without reopening.
            // Source/safety ZIPs stay retained and safety is synced before deletion.
            report_file(progress, index, current.len(), &item.path, "verify-file");
            verify_output(&mut output, entry, &|operation| {
                report_file(progress, index, current.len(), &item.path, operation);
            })?;
            report_file(progress, index, current.len(), &item.path, "close-file");
            drop(output);
            report(progress, "restoring", index + 1, current.len());
        }
        Ok(())
    })();
    if let Err(error) = attempt {
        report(progress, "rollback", 0, 0);
        result.errors.push(error);
        for target in created.iter().rev() {
            if fs_safety::check(target).is_err() || fs::remove_file(target).is_err() {
                result.rollback_remaining += 1;
            }
        }
        if let (Some(archive), false) = (safety_archive.as_ref(), removed.is_empty()) {
            result.rollback_remaining +=
                restore_chat_safety(archive, &removed).unwrap_or(removed.len());
        }
    } else {
        result.restored = created.len();
        report(progress, "completed", result.restored, current.len());
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn finish_trace(trace: RestoreTrace) {
        drop(trace.sender);
        trace._worker.join().unwrap(); // Tests drain their log; production never waits for it.
    }
    fn fixture() -> (PathBuf, Vec<Account>, Vec<String>) {
        let root = std::env::temp_dir().join(format!(
            "companion-migration-test-{}",
            time::OffsetDateTime::now_utc().unix_timestamp_nanos()
        ));
        let personal = root.join(".codex");
        let projects = personal.join(".chatgpt-projects");
        let work = root.join(".codex-cus");
        fs::create_dir_all(personal.join("sessions")).unwrap();
        fs::create_dir_all(projects.join(".metadata")).unwrap();
        fs::create_dir_all(projects.join("g-p-example/sources")).unwrap();
        fs::create_dir_all(work.join("pets/nyx")).unwrap();
        fs::create_dir_all(work.join("worktrees/wip")).unwrap();
        fs::create_dir_all(personal.join("cache")).unwrap();
        fs::write(personal.join("sessions/chat.jsonl"), "chat").unwrap();
        fs::write(personal.join("thread_history_1.sqlite"), "db").unwrap();
        fs::write(personal.join("auth.json"), "secret").unwrap();
        fs::write(personal.join("AUTH.JSON"), "secret").unwrap();
        fs::write(projects.join(".metadata/g-p-example.json"), "metadata").unwrap();
        fs::write(projects.join("g-p-example/sources/note.txt"), "project").unwrap();
        fs::write(projects.join("g-p-example/sources/Cargo.lock"), "lock").unwrap();
        fs::write(projects.join("g-p-example/sources/AUTH.JSON"), "secret").unwrap();
        fs::write(personal.join(".codex-global-state.json"), "state").unwrap();
        fs::create_dir_all(personal.join("app-server-daemon")).unwrap();
        fs::write(personal.join("app-server-daemon/runtime.exe"), "runtime").unwrap();
        fs::write(personal.join("state.json.tmp-123"), "temporary").unwrap();
        fs::write(personal.join(".sqlite-maintenance.lock"), "temporary").unwrap();
        fs::write(personal.join("cache/runtime"), "cache").unwrap();
        fs::create_dir_all(personal.join("CACHE")).unwrap();
        fs::write(personal.join("CACHE/runtime"), "cache").unwrap();
        fs::create_dir_all(personal.join("tmp")).unwrap();
        fs::write(personal.join("tmp/recreated"), "temporary").unwrap();
        fs::write(work.join("pets/nyx/pet.json"), "{}").unwrap();
        fs::write(work.join("worktrees/wip/readme.md"), "uncommitted").unwrap();
        let accounts = vec![
            account(".codex".into(), "Personal".into(), personal).unwrap(),
            account(".codex-cus".into(), "Work".into(), work).unwrap(),
        ];
        (
            root,
            accounts,
            vec![
                "chat".into(),
                "pets".into(),
                "projects".into(),
                "state".into(),
                "worktrees".into(),
            ],
        )
    }

    #[test]
    fn archive_groups_report_local_project_entries() {
        let entries = vec![Entry {
            account_id: ".codex".into(),
            relative_path: ".chatgpt-projects/.metadata/example.json".into(),
            archive_path: "accounts/.codex/.chatgpt-projects/.metadata/example.json".into(),
            group: "projects".into(),
            bytes: 42,
            sha256: "0".repeat(64),
        }];
        let projects = archive_groups(&entries)
            .into_iter()
            .find(|group| group.id == "projects")
            .unwrap();
        assert_eq!((projects.files, projects.bytes), (1, 42));
    }

    #[test]
    fn multi_account_roundtrip_excludes_credentials_and_caches() {
        let (root, accounts, groups) = fixture();
        #[cfg(windows)]
        {
            let link = root.join(".codex/plugins/cache/openai-bundled/chrome/latest");
            let target = root.join("plugin-runtime");
            fs::create_dir_all(link.parent().unwrap()).unwrap();
            fs::create_dir_all(&target).unwrap();
            fs::write(target.join("runtime.exe"), "not backed up").unwrap();
            let status = std::process::Command::new("cmd")
                .args(["/c", "mklink", "/J"])
                .arg(link.to_string_lossy().replace('/', "\\"))
                .arg(target.to_string_lossy().replace('/', "\\"))
                .status()
                .unwrap();
            assert!(status.success());
        }
        let (entries, excluded) = inventory(&accounts, &groups).unwrap();
        let estimates = estimate_accounts(&accounts).unwrap();
        assert_eq!(
            estimates[0]
                .groups
                .iter()
                .find(|group| group.id == "chat")
                .unwrap()
                .files,
            2
        );
        assert_eq!(
            estimates[0]
                .groups
                .iter()
                .find(|group| group.id == "projects")
                .unwrap()
                .bytes,
            19
        );
        assert_eq!(
            estimates[1]
                .groups
                .iter()
                .find(|group| group.id == "worktrees")
                .unwrap()
                .bytes,
            11
        );
        assert_eq!(entries.len(), 8);
        assert!(!entries.iter().any(|entry| entry
            .relative_path
            .to_ascii_lowercase()
            .contains("auth.json")));
        assert!(!entries
            .iter()
            .any(|entry| entry.relative_path.starts_with("tmp/")
                || entry.relative_path.starts_with("cache/")
                || entry.relative_path.starts_with("CACHE/")
                || entry.relative_path.contains(".tmp-")
                || entry.relative_path == ".sqlite-maintenance.lock"));
        assert!(!entries
            .iter()
            .any(|entry| entry.relative_path.starts_with("app-server-daemon/")));
        assert!(entries
            .iter()
            .any(|entry| entry.relative_path.ends_with("Cargo.lock")));
        assert!(excluded
            .iter()
            .any(|group| group.reason.contains("Authentication")));
        #[cfg(windows)]
        assert!(excluded
            .iter()
            .any(|group| group.reason == "Link or junction excluded for safe backup"));
        let archive = create_at(&accounts, &root.join("out"), entries.clone(), &groups).unwrap();
        let archive_path = PathBuf::from(archive.archive_path);
        let manifest = inspect_at(&archive_path).unwrap();
        assert_eq!(manifest.entries, entries);
        let destination = root.join("destination");
        let items = restore_items(&destination, &manifest).unwrap();
        let failed = restore_at(
            &destination,
            &archive_path,
            &items,
            false,
            &root.join("safety"),
            Some(1),
        )
        .unwrap();
        assert_eq!(failed.rollback_remaining, 0);
        assert!(!destination.join(".codex/sessions/chat.jsonl").exists());
        let done = restore_at(
            &destination,
            &archive_path,
            &items,
            false,
            &root.join("safety"),
            None,
        )
        .unwrap();
        assert_eq!(done.restored, 8);
        assert_eq!(
            fs::read(destination.join(".codex/.chatgpt-projects/g-p-example/sources/note.txt"))
                .unwrap(),
            b"project"
        );
        assert_eq!(
            fs::read(destination.join(".codex-cus/worktrees/wip/readme.md")).unwrap(),
            b"uncommitted"
        );
        fs::write(
            destination.join(".codex/sessions/chat.jsonl"),
            "destination",
        )
        .unwrap();
        fs::write(
            destination.join(".codex/thread_history_1.sqlite"),
            "target-db",
        )
        .unwrap();
        let project_registry = destination.join(".codex/.codex-global-state.json");
        let project_metadata =
            destination.join(".codex/.chatgpt-projects/.metadata/g-p-example.json");
        fs::write(&project_registry, "empty-target-project-registry").unwrap();
        fs::write(&project_metadata, "empty-target-project-metadata").unwrap();
        let extra_chat = destination.join(".codex/state_5.sqlite-wal");
        fs::write(&extra_chat, "stale-target-wal").unwrap();
        let target_skill = destination.join(".codex/skills/target-only/SKILL.md");
        fs::create_dir_all(target_skill.parent().unwrap()).unwrap();
        fs::write(&target_skill, "keep-target-only-skill").unwrap();
        let conflicts = restore_items(&destination, &manifest).unwrap();
        assert!(conflicts.iter().any(|item| item.status == "conflict"));
        let kept = restore_at(
            &destination,
            &archive_path,
            &conflicts,
            false,
            &root.join("safety"),
            None,
        )
        .unwrap();
        assert_eq!(kept.restored, 0);
        assert!(kept.errors.is_empty());
        assert_eq!(
            fs::read(&project_registry).unwrap(),
            b"empty-target-project-registry"
        );
        let failed_replacement = restore_at(
            &destination,
            &archive_path,
            &conflicts,
            true,
            &root.join("safety"),
            Some(1),
        )
        .unwrap();
        assert_eq!(failed_replacement.rollback_remaining, 0);
        assert_eq!(
            fs::read(destination.join(".codex/sessions/chat.jsonl")).unwrap(),
            b"destination"
        );
        assert_eq!(
            fs::read(destination.join(".codex/thread_history_1.sqlite")).unwrap(),
            b"target-db"
        );
        assert_eq!(
            fs::read(&project_registry).unwrap(),
            b"empty-target-project-registry"
        );
        assert_eq!(
            fs::read(&project_metadata).unwrap(),
            b"empty-target-project-metadata"
        );
        assert_eq!(fs::read(&extra_chat).unwrap(), b"stale-target-wal");
        assert_eq!(fs::read(&target_skill).unwrap(), b"keep-target-only-skill");
        fs::write(destination.join(".codex/auth.json"), "target-secret").unwrap();
        let replacement = restore_at(
            &destination,
            &archive_path,
            &conflicts,
            true,
            &root.join("safety"),
            None,
        )
        .unwrap();
        assert_eq!(replacement.restored, 8);
        let safety = PathBuf::from(replacement.safety_archive.unwrap());
        let safety_entries =
            chat_safety_entries(&destination, &inspect_at(&archive_path).unwrap()).unwrap();
        assert!(safety.exists());
        assert!(inspect_at(&safety).is_ok());
        assert_eq!(
            fs::read(destination.join(".codex/sessions/chat.jsonl")).unwrap(),
            b"chat"
        );
        assert_eq!(
            fs::read(destination.join(".codex/thread_history_1.sqlite")).unwrap(),
            b"db"
        );
        assert_eq!(fs::read(&project_registry).unwrap(), b"state");
        assert_eq!(fs::read(&project_metadata).unwrap(), b"metadata");
        assert!(!extra_chat.exists());
        assert_eq!(fs::read(&target_skill).unwrap(), b"keep-target-only-skill");
        assert!(!ZipArchive::new(File::open(safety).unwrap())
            .unwrap()
            .file_names()
            .any(|name| name.ends_with("auth.json")));
        assert_eq!(safety_entries.len(), 8);
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn rejects_invalid_account_folder_and_archive_path() {
        assert!(!valid_folder(".codex-cus/escape"));
        assert!(valid_folder(".codex-cus"));
        let (root, accounts, groups) = fixture();
        let entries = inventory(&accounts, &groups).unwrap().0;
        let archive = create_at(&accounts, &root.join("out"), entries, &groups).unwrap();
        fs::write(&archive.archive_path, b"not a zip").unwrap();
        assert!(inspect_at(Path::new(&archive.archive_path)).is_err());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn safety_rollback_continues_after_one_target_is_blocked() {
        let (root, accounts, groups) = fixture();
        let entries = inventory(&accounts, &groups).unwrap().0;
        let archive = create_at(&accounts, &root.join("out"), entries, &groups).unwrap();
        let manifest = inspect_at(Path::new(&archive.archive_path)).unwrap();
        let (safety, originals) =
            create_chat_safety_archive(&root, &manifest, &root.join("safety")).unwrap();
        // Keep the first file to simulate a concurrent writer blocking create_new.
        fs::remove_file(&originals[1].target).unwrap();
        assert_eq!(
            restore_chat_safety(&safety.unwrap(), &originals[..2]).unwrap(),
            1
        );
        assert_eq!(
            hash_file(&originals[1].target).unwrap(),
            originals[1].entry.sha256
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn output_handle_verifies_bytes_and_blocks_competing_mutations() {
        let (root, accounts, groups) = fixture();
        let mut entry = inventory(&accounts, &groups).unwrap().0.remove(0);
        let source = accounts
            .iter()
            .find(|account| account.id == entry.account_id)
            .unwrap();
        let bytes = fs::read(Path::new(&source.home).join(&entry.relative_path)).unwrap();
        let target = root.join("verified-output");
        let mut output = create_output(&target).unwrap();
        output.write_all(&bytes).unwrap();
        #[cfg(windows)]
        {
            assert!(OpenOptions::new().write(true).open(&target).is_err());
            assert!(fs::remove_file(&target).is_err());
        }
        verify_output(&mut output, &entry, &|_| {}).unwrap();
        entry.bytes += 1;
        assert!(verify_output(&mut output, &entry, &|_| {})
            .unwrap_err()
            .contains("size mismatch"));
        entry.bytes -= 1;
        let expected = entry.sha256.clone();
        entry.sha256 = "0".repeat(64);
        assert!(verify_output(&mut output, &entry, &|_| {})
            .unwrap_err()
            .contains("verification failed"));
        drop(output);
        assert_eq!(hash_file(&target).unwrap(), expected);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn output_digest_failure_rolls_back_all_original_files() {
        let (root, accounts, groups) = fixture();
        let entries = inventory(&accounts, &groups).unwrap().0;
        let archive = create_at(&accounts, &root.join("out"), entries, &groups).unwrap();
        let path = Path::new(&archive.archive_path);
        let mut manifest = inspect_at(path).unwrap();
        // Inject a verifier failure after strict archive validation, without modifying the ZIP.
        manifest.entries[0].sha256 = "0".repeat(64);
        let items = restore_items(&root, &manifest).unwrap();
        let originals: Vec<_> = manifest
            .entries
            .iter()
            .map(|entry| {
                let account = manifest
                    .accounts
                    .iter()
                    .find(|account| account.id == entry.account_id)
                    .unwrap();
                let target = root.join(&account.folder).join(&entry.relative_path);
                let bytes = fs::read(&target).unwrap();
                (target, bytes)
            })
            .collect();
        let result = restore_manifest_at(
            &root,
            path,
            &items,
            true,
            &root.join("safety"),
            None,
            &|_| {},
            &manifest,
        )
        .unwrap();
        assert_eq!(result.errors, ["Restored file verification failed."]);
        assert_eq!(result.restored, 0);
        assert_eq!(result.rollback_remaining, 0);
        for (target, bytes) in originals {
            assert_eq!(fs::read(target).unwrap(), bytes);
        }
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn stalled_full_diagnostic_queue_does_not_hold_restore() {
        use std::{
            sync::{mpsc, Arc},
            time::Duration,
        };
        struct PausedWriter {
            entered: mpsc::Sender<()>,
            release: Option<mpsc::Receiver<()>>,
            written: mpsc::Sender<()>,
            bytes: Arc<Mutex<Vec<u8>>>,
        }
        impl Write for PausedWriter {
            fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
                if let Some(release) = self.release.take() {
                    self.entered.send(()).unwrap();
                    release.recv().unwrap();
                }
                self.bytes.lock().unwrap().extend_from_slice(buffer);
                self.written.send(()).unwrap();
                Ok(buffer.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let (entered_tx, entered_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let (written_tx, written_rx) = mpsc::channel();
        let bytes = Arc::new(Mutex::new(Vec::new()));
        let trace = start_trace_writer(
            PausedWriter {
                entered: entered_tx,
                release: Some(release_rx),
                written: written_tx,
                bytes: Arc::clone(&bytes),
            },
            1,
        )
        .unwrap();
        let event = RestoreProgress {
            stage: "restoring".into(),
            completed: 0,
            total: 8,
            file: None,
            operation: None,
            ..Default::default()
        };
        trace_progress(&trace, &event, false).unwrap();
        entered_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        trace_progress(&trace, &event, true).unwrap(); // Fill the one-record queue behind the paused write.
        let (root, accounts, groups) = fixture();
        let entries = inventory(&accounts, &groups).unwrap().0;
        let archive = create_at(&accounts, &root.join("out"), entries, &groups).unwrap();
        let path = PathBuf::from(archive.archive_path);
        let manifest = inspect_at(&path).unwrap();
        let destination = root.join("destination");
        let items = restore_items(&destination, &manifest).unwrap();
        let safety = root.join("safety");
        let (completed_tx, completed_rx) = mpsc::channel();
        let job = std::thread::spawn(move || {
            let result = restore_manifest_at(
                &destination,
                &path,
                &items,
                true,
                &safety,
                None,
                &|event| notify_progress(&trace, event, &|_| {}),
                &manifest,
            )
            .unwrap();
            for entry in &manifest.entries {
                let account = manifest
                    .accounts
                    .iter()
                    .find(|account| account.id == entry.account_id)
                    .unwrap();
                assert_eq!(
                    hash_file(&destination.join(&account.folder).join(&entry.relative_path))
                        .unwrap(),
                    entry.sha256
                );
            }
            completed_tx.send(()).unwrap();
            (trace, result)
        });
        let completed_before_release = completed_rx.recv_timeout(Duration::from_secs(5));
        release_tx.send(()).unwrap(); // Always release the test writer before joining/raising an assertion.
        let (trace, result) = job.join().unwrap();
        assert!(
            completed_before_release.is_ok(),
            "Diagnostic backpressure held the restore."
        );
        assert!(result.errors.is_empty());
        assert_eq!(result.restored, 8);
        assert_eq!(result.rollback_remaining, 0);
        assert!(trace.dropped.get() > 0);
        for _ in 0..2 {
            written_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        }
        trace_progress(&trace, &event, true).unwrap();
        finish_trace(trace);
        let bytes = bytes.lock().unwrap();
        let text = std::str::from_utf8(&bytes).unwrap();
        assert!(text.ends_with('\n'));
        let records: Vec<serde_json::Value> = text
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert_eq!(records.len(), 3);
        assert!(records.last().unwrap()["droppedRecords"].as_u64().unwrap() > 0);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn disconnected_diagnostics_do_not_interrupt_notifications() {
        let (sender, receiver) = std::sync::mpsc::sync_channel(1);
        drop(receiver);
        let trace = RestoreTrace {
            sender,
            dropped: std::cell::Cell::new(0),
            _worker: std::thread::spawn(|| {}),
        };
        let called = std::cell::Cell::new(false);
        notify_progress(
            &trace,
            RestoreProgress {
                stage: "rollback".into(),
                completed: 0,
                total: 0,
                file: None,
                operation: None,
                ..Default::default()
            },
            &|_| called.set(true),
        );
        assert!(called.get());
        assert_eq!(trace.dropped.get(), 2);
        finish_trace(trace);
    }

    #[test]
    fn chunked_copy_retries_reads_and_handles_partial_writes() {
        struct InterruptedOnce {
            bytes: std::io::Cursor<Vec<u8>>,
            interrupted: bool,
        }
        impl Read for InterruptedOnce {
            fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
                if !self.interrupted {
                    self.interrupted = true;
                    return Err(std::io::ErrorKind::Interrupted.into());
                }
                self.bytes.read(buffer)
            }
        }
        struct ShortWriter(Vec<u8>);
        impl Write for ShortWriter {
            fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
                let count = bytes.len().min(777);
                self.0.extend_from_slice(&bytes[..count]);
                Ok(count)
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let bytes = vec![42; 2 * 65_536 + 17];
        let mut source = InterruptedOnce {
            bytes: std::io::Cursor::new(bytes.clone()),
            interrupted: false,
        };
        let mut output = ShortWriter(Vec::new());
        let events = std::cell::RefCell::new(Vec::new());
        assert_eq!(
            copy_with_progress(&mut source, &mut output, &|operation, copied| events
                .borrow_mut()
                .push((operation.to_owned(), copied)))
            .unwrap(),
            bytes.len() as u64
        );
        assert_eq!(output.0, bytes);
        let confirmed: Vec<_> = events
            .borrow()
            .iter()
            .filter(|event| event.0 == "copy-file")
            .map(|event| event.1)
            .collect();
        assert_eq!(confirmed, [65_536, 131_072, 131_089]);
        assert_eq!(
            copy_with_progress(&mut std::io::empty(), &mut Vec::new(), &|_, _| {}).unwrap(),
            0
        );
        let mut full = std::io::Cursor::new([0u8; 1]);
        let events = std::cell::RefCell::new(Vec::new());
        let error = copy_with_progress(
            &mut std::io::Cursor::new([1, 2]),
            &mut full,
            &|operation, copied| events.borrow_mut().push((operation.to_owned(), copied)),
        )
        .unwrap_err();
        assert!(error.contains("Destination write failed"));
        assert!(!events.borrow().iter().any(|event| event.0 == "copy-file"));
    }

    #[test]
    fn trace_distinguishes_callback_entry_from_return() {
        let (root, _, _) = fixture();
        let trace = create_restore_trace(&root.join("logs")).unwrap();
        let path = fs::read_dir(root.join("logs"))
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path();
        let event = RestoreProgress {
            stage: "restoring".into(),
            completed: 595,
            total: 618,
            file: Some("test.py".into()),
            operation: Some("check-target".into()),
            ..Default::default()
        };
        let called = std::cell::Cell::new(false);
        notify_progress(&trace, event, &|_| called.set(true));
        assert!(called.get());
        finish_trace(trace);
        let written = fs::read_to_string(&path).unwrap();
        let records: Vec<serde_json::Value> = written
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert_eq!(records.len(), 2);
        assert_eq!(records[0]["notificationCallbackReturned"], false);
        assert_eq!(records[1]["notificationCallbackReturned"], true);
        assert_eq!(records[1]["droppedRecords"], 0);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn restore_file_substeps_are_recorded_in_metadata_trace() {
        let (root, accounts, groups) = fixture();
        let entries = inventory(&accounts, &groups).unwrap().0;
        let archive = create_at(&accounts, &root.join("out"), entries, &groups).unwrap();
        let path = Path::new(&archive.archive_path);
        let manifest = inspect_at(path).unwrap();
        let destination = root.join("destination");
        let items = restore_items(&destination, &manifest).unwrap();
        let log = create_restore_trace(&root.join("logs")).unwrap();
        let events = std::cell::RefCell::new(Vec::new());
        let done = restore_manifest_at(
            &destination,
            path,
            &items,
            true,
            &root.join("safety"),
            None,
            &|event| {
                notify_progress(&log, event, &|event| events.borrow_mut().push(event));
            },
            &manifest,
        )
        .unwrap();
        assert!(done.errors.is_empty());
        assert_eq!(done.restored, manifest.entries.len());
        let events = events.into_inner();
        let checked: Vec<_> = events
            .iter()
            .filter(|event| event.operation.as_deref() == Some("check-path"))
            .map(|event| event.file.as_ref().unwrap())
            .collect();
        let first_entry = &manifest.entries[0];
        let first_account = manifest
            .accounts
            .iter()
            .find(|account| account.id == first_entry.account_id)
            .unwrap();
        let first = destination
            .join(&first_account.folder)
            .join(&first_entry.relative_path);
        let expected: Vec<_> = first
            .ancestors()
            .map(|path| path.to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            &checked[..expected.len()],
            &expected.iter().collect::<Vec<_>>()
        );
        let operations: Vec<_> = events
            .iter()
            .filter(|event| {
                event.file.as_deref() == Some(&items[0].path)
                    && event.operation.as_deref() != Some("check-path")
            })
            .map(|event| event.operation.as_deref().unwrap())
            .collect();
        assert_eq!(
            operations,
            [
                "prepare-target",
                "check-target",
                "create-parent",
                "recheck-target",
                "open-target",
                "read-archive",
                "copy-file",
                "copy-read",
                "copy-write",
                "copy-file",
                "copy-read",
                "verify-file",
                "verify-size",
                "verify-seek",
                "verify-read",
                "verify-digest",
                "close-file"
            ]
        );
        assert_eq!(events.last().unwrap().stage, "completed");
        finish_trace(log);
        let log_path = fs::read_dir(root.join("logs"))
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path();
        let logged = fs::read_to_string(log_path).unwrap();
        assert_eq!(logged.lines().count(), events.len() * 2);
        for (index, line) in logged.lines().enumerate() {
            let record: serde_json::Value = serde_json::from_str(line).unwrap();
            assert!(record["timestampMs"].as_i64().unwrap() > 0);
            assert_eq!(
                record["progress"],
                serde_json::to_value(&events[index / 2]).unwrap()
            );
            assert_eq!(record["notificationCallbackReturned"], index % 2 == 1);
        }
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn safety_scan_prunes_runtime_and_reports_verified_stored_snapshot() {
        let (root, accounts, groups) = fixture();
        let entries = inventory(&accounts, &groups).unwrap().0;
        let archive = create_at(&accounts, &root.join("out"), entries, &groups).unwrap();
        let manifest = inspect_at(Path::new(&archive.archive_path)).unwrap();
        fs::write(root.join(".codex/sessions/chat.jsonl.tmp"), "stale").unwrap();
        fs::create_dir_all(root.join(".codex/sessions/cache")).unwrap();
        fs::write(root.join(".codex/sessions/cache/unused.jsonl"), "cache").unwrap();
        let events = std::cell::RefCell::new(Vec::new());
        let (safety, originals) = create_chat_safety_archive_with_progress(
            &root,
            &manifest,
            &root.join("safety"),
            &|event| events.borrow_mut().push(event),
        )
        .unwrap();
        assert_eq!(originals.len(), manifest.entries.len());
        let mut zip = ZipArchive::new(File::open(safety.unwrap()).unwrap()).unwrap();
        for index in 0..zip.len() {
            assert_eq!(
                zip.by_index(index).unwrap().compression(),
                CompressionMethod::Stored
            );
        }
        for stage in ["safety-backup", "safety-validation"] {
            assert!(events.borrow().iter().any(|event| event.stage == stage
                && event.completed == originals.len()
                && event.total == originals.len()));
        }
        drop(zip);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    #[ignore = "Set DEV_COMPANION_VALIDATION_ZIP to validate a real migration ZIP in an isolated temp target"]
    fn real_archive_overwrites_fresh_install_and_verifies_every_file() {
        let path = PathBuf::from(std::env::var_os("DEV_COMPANION_VALIDATION_ZIP").unwrap());
        let _archive_lock = environment::locked_read(&path).unwrap();
        let manifest = inspect_at(&path).unwrap();
        let archive_hash = hash_file(&path).unwrap();
        let root = std::env::temp_dir().join(format!(
            "companion-migration-real-{}",
            time::OffsetDateTime::now_utc().unix_timestamp_nanos()
        ));
        for account in &manifest.accounts {
            let home = root.join(&account.folder);
            fs::create_dir_all(&home).unwrap();
            fs::write(home.join(".codex-global-state.json"), "{}").unwrap();
        }
        for scenario in ["fresh install", "populated target"] {
            let items = restore_items(&root, &manifest).unwrap();
            let started = std::time::Instant::now();
            let stage = std::cell::RefCell::new(String::new());
            let trace = create_restore_trace(&root.join("logs")).unwrap();
            assert_eq!(hash_file(&path).unwrap(), archive_hash);
            let restored = restore_manifest_at(
                &root,
                &path,
                &items,
                true,
                &root.join("safety"),
                None,
                &|event| {
                    notify_progress(&trace, event, &|event| {
                        if *stage.borrow() != event.stage {
                            println!(
                                "{scenario}: {} at {:.2}s",
                                event.stage,
                                started.elapsed().as_secs_f64()
                            );
                            *stage.borrow_mut() = event.stage;
                        }
                    });
                },
                &manifest,
            )
            .unwrap();
            assert!(restored.errors.is_empty(), "{:?}", restored.errors);
            assert_eq!(restored.rollback_remaining, 0);
            assert_eq!(restored.restored, manifest.entries.len());
            finish_trace(trace);
            for entry in &manifest.entries {
                let account = manifest
                    .accounts
                    .iter()
                    .find(|account| account.id == entry.account_id)
                    .unwrap();
                assert_eq!(
                    hash_file(&root.join(&account.folder).join(&entry.relative_path)).unwrap(),
                    entry.sha256
                );
            }
            println!(
            "{scenario}: restored {} files in {:.2}s; every SHA-256 verified in isolated target.",
            restored.restored, started.elapsed().as_secs_f64()
        );
        }
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn archive_list_and_delete_are_scoped_to_migration_zips() {
        let root = std::env::temp_dir().join(format!(
            "companion-migration-archives-{}",
            time::OffsetDateTime::now_utc().unix_timestamp_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        let migration = "codex-migration-123.zip";
        fs::write(root.join(migration), "migration").unwrap();
        fs::write(root.join("session-backup.zip"), "other").unwrap();
        let archives = list_archives_at(&root).unwrap();
        assert_eq!(archives.archives.len(), 1);
        assert_eq!(archives.archives[0].name, migration);
        assert_eq!(archives.total_bytes, 9);
        assert!(delete_archive_at(&root, migration).is_ok());
        assert!(!root.join(migration).exists());
        assert!(root.join("session-backup.zip").exists());
        assert!(delete_archive_at(&root, "../session-backup.zip").is_err());
        fs::remove_dir_all(root).unwrap();
    }
}
