//! Multi-account Codex folder migration. Credentials and runtime caches never enter this ZIP.
use crate::{environment, fs_safety, platform};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    cmp::Reverse,
    collections::{BTreeMap, HashMap, HashSet},
    fs::{self, File, OpenOptions},
    io::{Read, Write},
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
#[derive(Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Restored {
    pub restored: usize,
    pub skipped: usize,
    pub errors: Vec<String>,
    pub rollback_remaining: usize,
}

#[derive(Clone)]
enum Plan {
    Snapshot(Vec<Account>, Vec<String>, Vec<Entry>),
    Archive(PathBuf, String),
    Restore(PathBuf, String, Vec<RestoreItem>),
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
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
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

fn protected(path: &str) -> bool {
    matches!(path, "auth.json" | "cap_sid" | "installation_id")
}
fn runtime(path: &str) -> bool {
    path.split('/').next().is_some_and(|part| {
        matches!(
            part,
            "cache" | ".tmp" | ".sandbox" | ".sandbox-bin" | "vendor_imports"
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
    None
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
    for entry in &manifest.entries {
        let account = manifest
            .accounts
            .iter()
            .find(|account| account.id == entry.account_id)
            .ok_or("Unknown account in archive entry.")?;
        fs_safety::relative(&entry.relative_path)?;
        if !GROUPS.contains(&entry.group.as_str())
            || classify(&entry.relative_path) != Some(entry.group.as_str())
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
    }
    Ok(manifest)
}
pub fn inspect(path: PathBuf) -> Result<Preview> {
    let manifest = inspect_at(&path)?;
    let hash = digest(&fs::read(&path).map_err(|e| e.to_string())?);
    Ok(Preview {
        token: put(Plan::Archive(path, hash)),
        accounts: manifest.accounts,
        groups: Vec::new(),
        excluded: Vec::new(),
        entries: manifest.entries,
    })
}
fn restore_items(root: &Path, manifest: &Manifest) -> Result<Vec<RestoreItem>> {
    manifest
        .entries
        .iter()
        .map(|entry| {
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
pub fn preview_restore(token: &str) -> Result<RestorePreview> {
    let Plan::Archive(path, hash) = get(token)? else {
        return Err("Inspect archive first.".into());
    };
    if digest(&fs::read(&path).map_err(|e| e.to_string())?) != hash {
        return Err("Archive changed; inspect again.".into());
    }
    let manifest = inspect_at(&path)?;
    let items = restore_items(&home_dir()?, &manifest)?;
    Ok(RestorePreview {
        token: put(Plan::Restore(path, hash, items.clone())),
        items,
    })
}
pub fn restore(token: &str, confirmation: &str) -> Result<Restored> {
    if confirmation != "RESTORE" {
        return Err("Type RESTORE to confirm.".into());
    }
    environment::require_closed()?;
    let Plan::Restore(path, hash, items) = get(token)? else {
        return Err("Preview restore first.".into());
    };
    if digest(&fs::read(&path).map_err(|e| e.to_string())?) != hash {
        return Err("Archive changed; inspect again.".into());
    }
    restore_at(&home_dir()?, &path, &items, None)
}
fn restore_at(
    root: &Path,
    path: &Path,
    items: &[RestoreItem],
    fail_after: Option<usize>,
) -> Result<Restored> {
    let manifest = inspect_at(path)?;
    let current = restore_items(root, &manifest)?;
    if current != items {
        return Err("Destination changed; inspect again.".into());
    }
    let mut zip = ZipArchive::new(environment::locked_read(path)?).map_err(|e| e.to_string())?;
    let mut result = Restored::default();
    let mut created = Vec::new();
    let attempt = (|| -> Result<()> {
        for item in &current {
            if item.status != "new" {
                result.skipped += 1;
                continue;
            }
            if fail_after == Some(created.len()) {
                return Err("Injected restore failure.".into());
            }
            let entry = manifest
                .entries
                .iter()
                .find(|entry| entry.archive_path == item.archive_path)
                .unwrap();
            let account = manifest
                .accounts
                .iter()
                .find(|account| account.id == entry.account_id)
                .unwrap();
            let target = root.join(&account.folder).join(&entry.relative_path);
            fs_safety::check(&target)?;
            fs::create_dir_all(target.parent().unwrap()).map_err(|e| e.to_string())?;
            fs_safety::check(&target)?;
            let mut output = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&target)
                .map_err(|e| e.to_string())?;
            created.push(target.clone());
            std::io::copy(
                &mut zip.by_name(&item.archive_path).map_err(|e| e.to_string())?,
                &mut output,
            )
            .map_err(|e| e.to_string())?;
            output.sync_all().map_err(|e| e.to_string())?;
            drop(output);
            if hash_file(&target)? != entry.sha256 {
                return Err("Restored file verification failed.".into());
            }
        }
        Ok(())
    })();
    if let Err(error) = attempt {
        result.errors.push(error);
        for target in created.iter().rev() {
            if fs_safety::check(target).is_err() || fs::remove_file(target).is_err() {
                result.rollback_remaining += 1;
            }
        }
    } else {
        result.restored = created.len();
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (PathBuf, Vec<Account>, Vec<String>) {
        let root = std::env::temp_dir().join(format!(
            "companion-migration-test-{}",
            time::OffsetDateTime::now_utc().unix_timestamp_nanos()
        ));
        let personal = root.join(".codex");
        let work = root.join(".codex-cus");
        fs::create_dir_all(personal.join("sessions")).unwrap();
        fs::create_dir_all(work.join("pets/nyx")).unwrap();
        fs::create_dir_all(work.join("worktrees/wip")).unwrap();
        fs::create_dir_all(personal.join("cache")).unwrap();
        fs::write(personal.join("sessions/chat.jsonl"), "chat").unwrap();
        fs::write(personal.join("thread_history_1.sqlite"), "db").unwrap();
        fs::write(personal.join("auth.json"), "secret").unwrap();
        fs::write(personal.join("cache/runtime"), "cache").unwrap();
        fs::write(work.join("pets/nyx/pet.json"), "{}").unwrap();
        fs::write(work.join("worktrees/wip/readme.md"), "uncommitted").unwrap();
        let accounts = vec![
            account(".codex".into(), "Personal".into(), personal).unwrap(),
            account(".codex-cus".into(), "Work".into(), work).unwrap(),
        ];
        (
            root,
            accounts,
            vec!["chat".into(), "pets".into(), "worktrees".into()],
        )
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
            estimates[1]
                .groups
                .iter()
                .find(|group| group.id == "worktrees")
                .unwrap()
                .bytes,
            11
        );
        assert_eq!(entries.len(), 4);
        assert!(!entries
            .iter()
            .any(|entry| entry.relative_path == "auth.json"));
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
        let failed = restore_at(&destination, &archive_path, &items, Some(1)).unwrap();
        assert_eq!(failed.rollback_remaining, 0);
        assert!(!destination.join(".codex/sessions/chat.jsonl").exists());
        let done = restore_at(&destination, &archive_path, &items, None).unwrap();
        assert_eq!(done.restored, 4);
        assert_eq!(
            fs::read(destination.join(".codex-cus/worktrees/wip/readme.md")).unwrap(),
            b"uncommitted"
        );
        fs::write(
            destination.join(".codex/sessions/chat.jsonl"),
            "destination",
        )
        .unwrap();
        let conflicts = restore_items(&destination, &manifest).unwrap();
        assert!(conflicts.iter().any(|item| item.status == "conflict"));
        assert_eq!(
            restore_at(&destination, &archive_path, &conflicts, None)
                .unwrap()
                .restored,
            0
        );
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
