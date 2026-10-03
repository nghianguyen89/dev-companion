//! Explicit Windows file recovery, not a claim of Desktop database migration.
use crate::{fs_safety, platform};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{HashMap, HashSet},
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        Mutex, OnceLock,
    },
};
use zip::{write::SimpleFileOptions, CompressionMethod, ZipArchive, ZipWriter};

type Result<T> = std::result::Result<T, String>;
const LIMIT: u64 = 1024 * 1024 * 1024;
const GROUPS: &[&str] = &["chat", "settings", "skills", "pets"];
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Entry {
    pub path: String,
    pub group: String,
    pub bytes: u64,
    pub sha256: String,
    pub manual: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Manifest {
    pub format_version: u32,
    pub kind: String,
    pub created_at: String,
    pub platform: String,
    pub cli_version: Option<String>,
    pub entries: Vec<Entry>,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Group {
    pub id: String,
    pub files: usize,
    pub bytes: u64,
    pub reason: String,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Preview {
    pub token: String,
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
pub struct RestoreItem {
    pub path: String,
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
    Backup(PathBuf, Vec<String>, Vec<Entry>),
    Archive(PathBuf, String),
    Restore(PathBuf, String, Vec<RestoreItem>),
}
static PLANS: OnceLock<Mutex<HashMap<String, Plan>>> = OnceLock::new();
static NEXT: AtomicU64 = AtomicU64::new(1);
fn put(plan: Plan) -> String {
    let token = format!("environment-{}", NEXT.fetch_add(1, Ordering::Relaxed));
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
pub fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub fn require_closed() -> Result<()> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let output = std::process::Command::new("tasklist.exe")
            .args(["/FO", "CSV", "/NH"])
            .creation_flags(0x08000000)
            .output()
            .map_err(|_| "Cannot check Codex processes. Close Codex and retry.")?;
        if !output.status.success() {
            return Err("Cannot check Codex processes.".into());
        }
        if String::from_utf8_lossy(&output.stdout).lines().any(|line| {
            let name = line
                .split(',')
                .next()
                .unwrap_or("")
                .trim_matches('"')
                .to_ascii_lowercase();
            matches!(
                name.as_str(),
                "codex.exe" | "codex-cli.exe" | "codex-app-server.exe"
            )
        }) {
            return Err("Close Codex Desktop and all Codex CLI processes, then scan again. Companion never closes them automatically.".into());
        }
        Ok(())
    }
    #[cfg(not(windows))]
    {
        Err("Environment snapshot currently supports Windows only.".into())
    }
}

pub fn locked_read(path: &Path) -> Result<File> {
    fs_safety::check(path)?;
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.share_mode(1);
    }
    let file = options
        .open(path)
        .map_err(|e| format!("Close Codex; cannot lock source for reading: {e}"))?;
    if !file.metadata().map_err(|e| e.to_string())?.is_file() {
        return Err("Not a regular file.".into());
    }
    Ok(file)
}

fn classify(path: &str) -> Option<(&'static str, bool)> {
    if path.split('/').any(|p| {
        matches!(p, "backups" | "quarantine" | ".git")
            || p.starts_with("codex-backup-")
            || p.starts_with("codex-delete-safety-")
            || p.starts_with("codex-environment-")
    }) {
        return None;
    }
    if path.starts_with("sessions/")
        || path.starts_with("archived_sessions/")
        || path.starts_with("attachments/")
    {
        return Some(("chat", false));
    }
    if matches!(path, "session_index.jsonl" | ".codex-global-state.json")
        || [
            "state_5.sqlite",
            "thread_history_1.sqlite",
            "goals_1.sqlite",
            "queue_1.sqlite",
            "memories_1.sqlite",
        ]
        .iter()
        .any(|db| path == *db || path == format!("{db}-wal") || path == format!("{db}-shm"))
        || path.starts_with("sqlite/")
    {
        return Some(("chat", true));
    }
    if matches!(path, "config.toml" | "AGENTS.md" | "hooks.json")
        || path.starts_with("rules/")
        || path.starts_with("prompts/")
    {
        return Some(("settings", true));
    }
    if path.starts_with("skills/") || path.starts_with("plugins/cache/") {
        return Some(("skills", false));
    }
    if path.starts_with("pets/") {
        return Some(("pets", false));
    }
    None
}
fn excluded_reason(path: &str) -> &'static str {
    if matches!(path, "auth.json" | "cap_sid" | "installation_id") || path.contains("credential") {
        "Authentication / machine identity: sign in again"
    } else if path.starts_with("plugins/") {
        "Plugin runtime / registry: reinstall and reconnect; installed cache retained with skills"
    } else if path.starts_with("logs/")
        || path.starts_with("log/")
        || path.starts_with("logs_2.sqlite")
    {
        "Diagnostic logs, not recovery data"
    } else if path.starts_with("tmp/") || path == "models_cache.json" {
        "Regenerable runtime data"
    } else {
        "Outside supported scope / recovery artifacts / machine-specific data; source unchanged"
    }
}
fn walk(root: &Path, dir: &Path, files: &mut Vec<String>) -> Result<()> {
    fs_safety::check(dir)?;
    for entry in fs::read_dir(dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        let m = fs::symlink_metadata(&path).map_err(|e| e.to_string())?;
        if fs_safety::linked(&m) {
            return Err(format!(
                "Link or junction excluded; resolve before snapshot: {}",
                path.display()
            ));
        }
        if m.is_dir() {
            walk(root, &path, files)?;
        } else if m.is_file() {
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
fn inventory(home: &Path, groups: &[String]) -> Result<(Vec<Entry>, Vec<Group>)> {
    if groups.is_empty()
        || groups.iter().any(|g| !GROUPS.contains(&g.as_str()))
        || groups.iter().collect::<HashSet<_>>().len() != groups.len()
    {
        return Err("Select valid component groups.".into());
    }
    let mut files = Vec::new();
    walk(home, home, &mut files)?;
    files.sort();
    let mut entries = Vec::new();
    let mut excluded: HashMap<String, Group> = HashMap::new();
    for path in files {
        fs_safety::relative(&path)?;
        let size = fs::metadata(home.join(&path))
            .map_err(|e| e.to_string())?
            .len();
        if let Some((group, manual)) =
            classify(&path).filter(|(g, _)| groups.iter().any(|s| s == g))
        {
            // Credential-bearing settings are never silently copied or redacted. Review separately.
            let mut file = locked_read(&home.join(&path))?;
            if size > LIMIT {
                return Err(format!("File exceeds 1 GiB snapshot limit: {path}"));
            }
            let mut bytes = Vec::new();
            file.read_to_end(&mut bytes).map_err(|e| e.to_string())?;
            if group == "settings" {
                let lower = String::from_utf8_lossy(&bytes).to_ascii_lowercase();
                if [
                    "api_key",
                    "bearer_token",
                    "authorization",
                    "password",
                    "secret",
                    "access_token",
                ]
                .iter()
                .any(|s| lower.contains(s))
                {
                    let reason = "Settings may contain credentials: omitted intact; review and configure manually".to_string();
                    let g = excluded.entry(reason.clone()).or_insert(Group {
                        id: reason.clone(),
                        files: 0,
                        bytes: 0,
                        reason,
                    });
                    g.files += 1;
                    g.bytes += size;
                    continue;
                }
            }
            entries.push(Entry {
                path,
                group: group.into(),
                bytes: bytes.len() as u64,
                sha256: digest(&bytes),
                manual,
            });
        } else {
            let reason = if classify(&path).is_some() {
                "Component not selected"
            } else {
                excluded_reason(&path)
            }
            .to_owned();
            let g = excluded.entry(reason.clone()).or_insert(Group {
                id: reason.clone(),
                files: 0,
                bytes: 0,
                reason,
            });
            g.files += 1;
            g.bytes += size;
        }
    }
    let mut excluded: Vec<_> = excluded.into_values().collect();
    excluded.sort_by(|a, b| a.id.cmp(&b.id));
    Ok((entries, excluded))
}
pub fn preview(groups: Vec<String>) -> Result<Preview> {
    let home = platform::codex_home();
    let (entries, excluded) = inventory(&home, &groups)?;
    let summaries = groups
        .iter()
        .map(|id| Group {
            id: id.clone(),
            files: entries.iter().filter(|e| &e.group == id).count(),
            bytes: entries
                .iter()
                .filter(|e| &e.group == id)
                .map(|e| e.bytes)
                .sum(),
            reason: "Selected recovery files; database/settings require manual migration".into(),
        })
        .collect();
    let token = put(Plan::Backup(home, groups, entries.clone()));
    Ok(Preview {
        token,
        groups: summaries,
        excluded,
        entries,
    })
}
pub fn create(token: &str) -> Result<Created> {
    require_closed()?;
    let Plan::Backup(home, groups, expected) = get(token)? else {
        return Err("Invalid backup preview.".into());
    };
    let (entries, _) = inventory(&home, &groups)?;
    if entries != expected {
        return Err("Source changed; preview again.".into());
    }
    let config = crate::config::load().map_err(|e| e.to_string())?;
    create_at(&home, &platform::backup_dir(&config), entries, &groups)
}
fn create_at(
    home: &Path,
    output: &Path,
    entries: Vec<Entry>,
    groups: &[String],
) -> Result<Created> {
    fs_safety::check(output)?;
    fs::create_dir_all(output).map_err(|e| e.to_string())?;
    if output.starts_with(home) {
        return Err("Backup output must be outside CODEX_HOME.".into());
    }
    // Hold deny-write/delete handles for the complete selected set, including SQLite WAL/SHM.
    let mut inputs = entries
        .iter()
        .map(|e| locked_read(&home.join(&e.path)))
        .collect::<Result<Vec<_>>>()?;
    let stamp = time::OffsetDateTime::now_utc().unix_timestamp_nanos();
    let path = output.join(format!("codex-environment-{stamp}.zip"));
    let file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&path)
        .map_err(|e| e.to_string())?;
    let result = (|| -> Result<()> {
        let manifest = Manifest {
            format_version: 2,
            kind: "codex-companion-environment".into(),
            created_at: time::OffsetDateTime::now_utc()
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap(),
            platform: "windows".into(),
            cli_version: crate::codex::cli_version(),
            entries: entries.clone(),
        };
        let mut zip = ZipWriter::new(file);
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        zip.start_file("environment-manifest.json", options)
            .map_err(|e| e.to_string())?;
        zip.write_all(&serde_json::to_vec_pretty(&manifest).unwrap())
            .map_err(|e| e.to_string())?;
        for (entry, input) in entries.iter().zip(inputs.iter_mut()) {
            let mut bytes = Vec::new();
            input.read_to_end(&mut bytes).map_err(|e| e.to_string())?;
            if digest(&bytes) != entry.sha256 {
                return Err("Source changed; close Codex and preview again.".into());
            }
            zip.start_file(&entry.path, options)
                .map_err(|e| e.to_string())?;
            zip.write_all(&bytes).map_err(|e| e.to_string())?;
        }
        zip.finish()
            .map_err(|e| e.to_string())?
            .sync_all()
            .map_err(|e| e.to_string())?;
        // Detect newly created/deleted selected files while all original files remain locked.
        if inventory(home, groups)?.0 != entries {
            return Err("Source inventory changed; close Codex and preview again.".into());
        }
        inspect_at(&path)?;
        Ok(())
    })();
    if let Err(e) = result {
        let _ = fs::remove_file(&path);
        return Err(e);
    }
    Ok(Created {
        archive_bytes: fs::metadata(&path).map_err(|e| e.to_string())?.len(),
        archive_path: path.to_string_lossy().into(),
        files: entries.len(),
    })
}
fn inspect_at(path: &Path) -> Result<Manifest> {
    let mut archive = ZipArchive::new(locked_read(path)?).map_err(|e| e.to_string())?;
    let mut names = HashSet::new();
    for i in 0..archive.len() {
        let entry = archive.by_index(i).map_err(|e| e.to_string())?;
        fs_safety::relative(entry.name())?;
        if entry.is_dir()
            || entry.size() > LIMIT
            || !names.insert(entry.name().to_lowercase())
            || entry.unix_mode().is_some_and(|m| m & 0o170000 == 0o120000)
        {
            return Err("Invalid archive entry.".into());
        }
    }
    let mut bytes = Vec::new();
    archive
        .by_name("environment-manifest.json")
        .map_err(|e| e.to_string())?
        .take(4 * 1024 * 1024)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    let manifest: Manifest = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    if manifest.format_version != 2
        || manifest.kind != "codex-companion-environment"
        || manifest.platform != "windows"
        || manifest.entries.len() + 1 != names.len()
    {
        return Err("Unsupported environment manifest.".into());
    }
    let mut listed = HashSet::new();
    for entry in &manifest.entries {
        fs_safety::relative(&entry.path)?;
        let Some((group, manual)) = classify(&entry.path) else {
            return Err("Protected or unsupported archive path.".into());
        };
        if group != entry.group
            || manual != entry.manual
            || !listed.insert(entry.path.to_lowercase())
        {
            return Err("Invalid component policy.".into());
        }
        let mut file = archive.by_name(&entry.path).map_err(|e| e.to_string())?;
        if file.size() != entry.bytes {
            return Err("Archive size mismatch.".into());
        }
        let mut hash = Sha256::new();
        let mut buffer = [0u8; 65536];
        loop {
            let n = file.read(&mut buffer).map_err(|e| e.to_string())?;
            if n == 0 {
                break;
            }
            hash.update(&buffer[..n]);
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
    let token = put(Plan::Archive(path, hash));
    Ok(Preview {
        token,
        groups: Vec::new(),
        excluded: Vec::new(),
        entries: manifest.entries,
    })
}
fn restore_items(home: &Path, manifest: &Manifest) -> Result<Vec<RestoreItem>> {
    manifest
        .entries
        .iter()
        .map(|e| {
            let target = home.join(&e.path);
            fs_safety::check(&target)?;
            let status = match fs::symlink_metadata(&target) {
                Ok(m) if m.is_file() => {
                    if digest(&fs::read(&target).map_err(|e| e.to_string())?) == e.sha256 {
                        "identical"
                    } else {
                        "conflict"
                    }
                }
                Ok(_) => "conflict",
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                    if e.manual {
                        "manual"
                    } else {
                        "new"
                    }
                }
                Err(err) => return Err(err.to_string()),
            };
            Ok(RestoreItem {
                path: e.path.clone(),
                status: status.into(),
                bytes: e.bytes,
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
    let items = restore_items(&platform::codex_home(), &manifest)?;
    let token = put(Plan::Restore(path, hash, items.clone()));
    Ok(RestorePreview { token, items })
}
pub fn restore(token: &str, confirmation: &str) -> Result<Restored> {
    if confirmation != "RESTORE" {
        return Err("Type RESTORE to confirm.".into());
    }
    require_closed()?;
    let Plan::Restore(path, hash, items) = get(token)? else {
        return Err("Preview restore first.".into());
    };
    if digest(&fs::read(&path).map_err(|e| e.to_string())?) != hash {
        return Err("Archive changed; inspect again.".into());
    }
    restore_at(&platform::codex_home(), &path, &items, None)
}
fn restore_at(
    home: &Path,
    path: &Path,
    items: &[RestoreItem],
    fail_after: Option<usize>,
) -> Result<Restored> {
    let manifest = inspect_at(path)?;
    let current = restore_items(home, &manifest)?;
    if current
        .iter()
        .zip(items)
        .any(|(a, b)| a.path != b.path || a.status != b.status)
        || current.len() != items.len()
    {
        return Err("Destination changed; preview again.".into());
    }
    let mut zip = ZipArchive::new(locked_read(path)?).map_err(|e| e.to_string())?;
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
            let target = home.join(&item.path);
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
                &mut zip.by_name(&item.path).map_err(|e| e.to_string())?,
                &mut output,
            )
            .map_err(|e| e.to_string())?;
            output.sync_all().map_err(|e| e.to_string())?;
            drop(output);
            let expected = manifest
                .entries
                .iter()
                .find(|e| e.path == item.path)
                .unwrap();
            if digest(&fs::read(&target).map_err(|e| e.to_string())?) != expected.sha256 {
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
    fn fixture() -> (PathBuf, PathBuf, Vec<String>) {
        let root = std::env::temp_dir().join(format!(
            "companion-env-test-{}",
            time::OffsetDateTime::now_utc().unix_timestamp_nanos()
        ));
        let home = root.join("source");
        fs::create_dir_all(home.join("skills/demo")).unwrap();
        fs::create_dir_all(home.join("pets/demo")).unwrap();
        fs::write(home.join("skills/demo/SKILL.md"), "synthetic skill").unwrap();
        fs::write(home.join("pets/demo/pet.json"), "{}").unwrap();
        fs::write(home.join("auth.json"), "secret fixture").unwrap();
        fs::write(home.join("config.toml"), "api_key='synthetic'").unwrap();
        fs::write(home.join("state_5.sqlite"), "synthetic db bytes").unwrap();
        fs::write(home.join("state_5.sqlite-wal"), "synthetic WAL").unwrap();
        (root, home, GROUPS.iter().map(|s| s.to_string()).collect())
    }
    #[test]
    fn environment_roundtrip_conflicts_manual_components_and_rollback() {
        let (root, home, groups) = fixture();
        let (entries, excluded) = inventory(&home, &groups).unwrap();
        assert!(!entries
            .iter()
            .any(|e| e.path == "auth.json" || e.path == "config.toml"));
        assert!(excluded.iter().any(|g| g.reason.contains("credentials")));
        assert!(entries.iter().any(|e| e.path.ends_with("-wal") && e.manual));
        let archive = create_at(&home, &root.join("out"), entries.clone(), &groups).unwrap();
        let archive = PathBuf::from(archive.archive_path);
        let manifest = inspect_at(&archive).unwrap();
        assert_eq!(manifest.entries, entries);
        let dest = root.join("destination");
        let items = restore_items(&dest, &manifest).unwrap();
        let failed = restore_at(&dest, &archive, &items, Some(1)).unwrap();
        assert_eq!(failed.errors.len(), 1);
        assert_eq!(failed.rollback_remaining, 0);
        assert!(!dest.join("pets/demo/pet.json").exists());
        assert!(!dest.join("skills/demo/SKILL.md").exists());
        let done = restore_at(&dest, &archive, &items, None).unwrap();
        assert_eq!(done.restored, 2);
        assert_eq!(done.skipped, 2);
        assert!(!dest.join("state_5.sqlite").exists());
        assert_eq!(
            fs::read(dest.join("skills/demo/SKILL.md")).unwrap(),
            b"synthetic skill"
        );
        let identical = restore_items(&dest, &manifest).unwrap();
        assert_eq!(
            identical.iter().filter(|e| e.status == "identical").count(),
            2
        );
        fs::write(dest.join("skills/demo/SKILL.md"), "keep destination").unwrap();
        assert!(restore_at(&dest, &archive, &identical, None).is_err());
        let conflicts = restore_items(&dest, &manifest).unwrap();
        assert!(conflicts.iter().any(|e| e.status == "conflict"));
        assert_eq!(
            restore_at(&dest, &archive, &conflicts, None)
                .unwrap()
                .restored,
            0
        );
        assert_eq!(
            fs::read(dest.join("skills/demo/SKILL.md")).unwrap(),
            b"keep destination"
        );
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn archive_corruption_and_unsafe_components_are_rejected() {
        let (root, home, groups) = fixture();
        let entries = inventory(&home, &groups).unwrap().0;
        let archive = create_at(&home, &root.join("out"), entries, &groups).unwrap();
        fs::write(&archive.archive_path, b"corrupt ZIP").unwrap();
        assert!(inspect_at(Path::new(&archive.archive_path)).is_err());
        assert!(inventory(&home, &["../auth".into()]).is_err());
        assert!(classify("skills/demo/backups/old.zip").is_none());
        for path in [
            "../auth.json",
            "C:/auth.json",
            "skills/NUL",
            "pets/a.",
            "skills/a:stream",
        ] {
            assert!(fs_safety::relative(path).is_err());
        }
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn locked_source_denies_windows_writer() {
        let (root, home, _) = fixture();
        let target = home.join("state_5.sqlite");
        let input = locked_read(&target).unwrap();
        #[cfg(windows)]
        assert!(OpenOptions::new().write(true).open(&target).is_err());
        drop(input);
        fs::remove_dir_all(root).unwrap();
    }
}
