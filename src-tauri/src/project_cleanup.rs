use crate::fs_safety;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{HashSet, VecDeque},
    fs::{self, Metadata},
    io::Read,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex, OnceLock,
    },
    time::SystemTime,
};

#[path = "recycle.rs"]
mod recycle;

const DELETE_CONFIRMATION: &str = "DELETE GENERATED";
const RECYCLE_CONFIRMATION: &str = "RECYCLE GENERATED";
const MAX_DEPTH: usize = 64;
const MAX_ENTRIES: usize = 100_000;
const MAX_DIRECTORIES: usize = 20_000;
const MAX_CANDIDATES: usize = 2_048;
const MAX_PACKAGE_JSON: u64 = 1024 * 1024;
const MAX_HASH_BYTES: u64 = 4 * 1024 * 1024;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CandidateDto {
    pub id: String,
    pub path: String,
    pub kind: String,
    pub files: usize,
    pub bytes: u64,
    pub blocked: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Scan {
    pub token: String,
    pub root: String,
    pub candidates: Vec<CandidateDto>,
    pub skipped: usize,
    pub incomplete: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Preview {
    pub token: String,
    pub root: String,
    pub candidates: Vec<CandidateDto>,
    pub files: usize,
    pub bytes: u64,
    pub mode: Mode,
}

#[derive(Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Outcome {
    pub mode: Mode,
    pub deleted_files: usize,
    pub removed_directories: usize,
    pub reclaimed_bytes: u64,
    pub recycled_files: usize,
    pub recycled_directories: usize,
    pub recycled_bytes: u64,
    pub errors: Vec<String>,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Mode {
    #[default]
    Permanent,
    Recycle,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Entry {
    relative: PathBuf,
    directory: bool,
    bytes: u64,
    modified: SystemTime,
    hash: Option<[u8; 32]>,
    link: Option<recycle::LinkInfo>,
}

#[derive(Clone)]
struct Candidate {
    dto: CandidateDto,
    path: PathBuf,
    kind: Kind,
    markers: Vec<Marker>,
    entries: Vec<Entry>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Marker {
    path: PathBuf,
    entry: Entry,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum Kind {
    NodeDependencies,
    NodeCache,
    PythonCache,
}

impl Kind {
    fn value(self) -> &'static str {
        match self {
            Self::NodeDependencies => "nodeDependencies",
            Self::NodeCache => "nodeCache",
            Self::PythonCache => "pythonCache",
        }
    }
}

struct ScanPlan {
    token: String,
    root: PathBuf,
    candidates: Vec<Candidate>,
    incomplete: bool,
}

struct PreviewPlan {
    token: String,
    root: PathBuf,
    candidates: Vec<Candidate>,
    mode: Mode,
}

#[derive(Default)]
struct Plans {
    scan: Option<ScanPlan>,
    preview: Option<PreviewPlan>,
}

#[derive(Default)]
struct Limits {
    entries: usize,
    directories: usize,
    skipped: usize,
    incomplete: bool,
}

struct ActiveScan {
    token: String,
    cancelled: Arc<AtomicBool>,
}

static PLANS: OnceLock<Mutex<Plans>> = OnceLock::new();
static ACTIVE_SCAN: OnceLock<Mutex<Option<ActiveScan>>> = OnceLock::new();
static NEXT_TOKEN: AtomicU64 = AtomicU64::new(1);

fn plans() -> &'static Mutex<Plans> {
    PLANS.get_or_init(Default::default)
}

fn token(prefix: &str) -> String {
    format!(
        "project-cleanup-{prefix}-{}-{}",
        time::OffsetDateTime::now_utc().unix_timestamp_nanos(),
        NEXT_TOKEN.fetch_add(1, Ordering::Relaxed)
    )
}

fn check_cancel(cancelled: Option<&AtomicBool>) -> Result<(), String> {
    if cancelled.is_some_and(|flag| flag.load(Ordering::Relaxed)) {
        Err("Scan cancelled.".into())
    } else {
        Ok(())
    }
}

fn display(path: &Path) -> String {
    let value = path.to_string_lossy();
    if let Some(value) = value.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{value}")
    } else {
        value.strip_prefix(r"\\?\").unwrap_or(&value).to_string()
    }
}

fn relative_display(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .map(|value| value.to_string_lossy().replace('\\', "/"))
        .unwrap_or_else(|_| display(path))
}

fn normalized(path: &Path) -> String {
    display(path)
        .trim_end_matches(|character| character == '\\' || character == '/')
        .to_ascii_lowercase()
}

fn normalized_descendant(parent: &str, child: &str) -> bool {
    child
        .strip_prefix(parent)
        .is_some_and(|rest| rest.starts_with('\\') || rest.starts_with('/'))
}

fn managed_roots() -> Vec<String> {
    let mut managed: Vec<PathBuf> = [
        "WINDIR",
        "ProgramFiles",
        "ProgramFiles(x86)",
        "ProgramData",
        "APPDATA",
        "LOCALAPPDATA",
    ]
    .into_iter()
    .filter_map(|name| std::env::var_os(name))
    .filter_map(|path| fs::canonicalize(path).ok())
    .collect();
    if let Some(home) = dirs::home_dir().and_then(|path| fs::canonicalize(path).ok()) {
        managed.push(home.join("AppData"));
    }
    managed.iter().map(|path| normalized(path)).collect()
}

fn within_managed(path: &Path, managed: &[String]) -> bool {
    let path = normalized(path);
    managed.iter().any(|root| {
        path == *root
            || path
                .strip_prefix(root)
                .is_some_and(|rest| rest.starts_with('\\') || rest.starts_with('/'))
    })
}

fn protected_ancestor(root: &Path) -> bool {
    let protected_component = root.components().any(|component| {
        component.as_os_str().to_str().is_some_and(|name| {
            let name = name.to_ascii_lowercase();
            name.starts_with(".codex-")
                || name.starts_with(".dev-companion-recycle-recovery-")
                || matches!(
                    name.as_str(),
                    ".git"
                        | ".svn"
                        | ".hg"
                        | ".codex"
                        | ".agents"
                        | "backup"
                        | "backups"
                        | "uploads"
                        | "db"
                        | "database"
                        | "databases"
                        | "appdata"
                        | "windows"
                        | "programdata"
                        | "program files"
                        | "program files (x86)"
                        | ".next"
                        | ".nuxt"
                        | ".output"
                        | ".svelte-kit"
                        | "coverage"
                        | "out"
                )
        })
    });
    if protected_component {
        return true;
    }
    within_managed(root, &managed_roots())
}

fn validate_root(value: &str) -> Result<PathBuf, String> {
    let path = Path::new(value);
    if !path.is_absolute() {
        return Err("Choose an absolute project or container folder.".into());
    }
    fs_safety::check(path)?;
    let metadata = fs::symlink_metadata(path).map_err(|e| format!("Cannot inspect root: {e}"))?;
    if !metadata.is_dir() || fs_safety::linked(&metadata) {
        return Err("Cleanup root must be a regular directory.".into());
    }
    let root = fs::canonicalize(path).map_err(|e| format!("Cannot resolve root: {e}"))?;
    if root.parent().is_none() {
        return Err("A drive root cannot be cleaned.".into());
    }
    if let Some(home) = dirs::home_dir().and_then(|p| fs::canonicalize(p).ok()) {
        if root == home {
            return Err("The user home folder cannot be cleaned.".into());
        }
    }
    if protected_ancestor(&root) {
        return Err("Protected source-control or system-managed folders cannot be cleaned.".into());
    }
    fs_safety::check(&root)?;
    Ok(root)
}

fn metadata_entry(
    root: &Path,
    path: &Path,
    metadata: &Metadata,
    force_hash: bool,
    cancelled: Option<&AtomicBool>,
) -> Result<Entry, String> {
    if fs_safety::linked(metadata) || (!metadata.is_file() && !metadata.is_dir()) {
        return Err("Link or unsupported filesystem entry found.".into());
    }
    let bytes = if metadata.is_file() {
        metadata.len()
    } else {
        0
    };
    let hash = if metadata.is_file() && (force_hash || bytes <= MAX_HASH_BYTES) {
        let hash = hash_file(path, cancelled)?;
        let after = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
        if fs_safety::linked(&after)
            || !after.is_file()
            || after.len() != bytes
            || after.modified().ok() != metadata.modified().ok()
        {
            return Err("File changed while it was being inspected.".into());
        }
        Some(hash)
    } else {
        None
    };
    Ok(Entry {
        relative: path
            .strip_prefix(root)
            .map_err(|_| "Candidate escaped its root.".to_string())?
            .to_path_buf(),
        directory: metadata.is_dir(),
        bytes,
        modified: metadata.modified().map_err(|e| e.to_string())?,
        hash,
        link: None,
    })
}

fn hash_file(path: &Path, cancelled: Option<&AtomicBool>) -> Result<[u8; 32], String> {
    fs_safety::check(path)?;
    let mut file = fs::File::open(path).map_err(|e| e.to_string())?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        check_cancel(cancelled)?;
        let read = file.read(&mut buffer).map_err(|e| e.to_string())?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }
    Ok(digest.finalize().into())
}

fn valid_package(path: &Path, cancelled: Option<&AtomicBool>) -> Result<Entry, String> {
    fs_safety::check(path)?;
    let metadata = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    if fs_safety::linked(&metadata) || !metadata.is_file() || metadata.len() > MAX_PACKAGE_JSON {
        return Err("Project package.json is not a small regular file.".into());
    }
    let bytes = fs::read(path).map_err(|e| e.to_string())?;
    if !serde_json::from_slice::<serde_json::Value>(&bytes)
        .map(|v| v.is_object())
        .unwrap_or(false)
    {
        return Err("Project package.json must contain a JSON object.".into());
    }
    metadata_entry(
        path.parent().unwrap_or(path),
        path,
        &metadata,
        true,
        cancelled,
    )
}

fn project_manifest(
    root: &Path,
    candidate: &Path,
    cancelled: Option<&AtomicBool>,
) -> Result<Marker, String> {
    for ancestor in candidate.parent().into_iter().flat_map(Path::ancestors) {
        if !ancestor.starts_with(root) {
            break;
        }
        let manifest = ancestor.join("package.json");
        if manifest.exists() {
            return valid_package(&manifest, cancelled).map(|entry| Marker {
                path: manifest,
                entry,
            });
        }
        if ancestor == root {
            break;
        }
    }
    Err("No valid package.json was found at the project ancestor.".into())
}

fn protected(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    name.starts_with(".codex-")
        || name.starts_with(".dev-companion-recycle-recovery-")
        || matches!(
            name.as_str(),
            ".git"
                | ".svn"
                | ".hg"
                | ".codex"
                | ".agents"
                | ".cache"
                | ".next"
                | ".nuxt"
                | ".output"
                | ".svelte-kit"
                | "system"
                | ".yarn"
                | "dist"
                | "build"
                | "target"
                | "vendor"
                | "db"
                | "database"
                | "databases"
                | "backup"
                | "backups"
                | "uploads"
                | "cache"
                | "log"
                | "logs"
                | "temp"
                | "tmp"
                | "output"
                | "coverage"
                | "out"
                | "appdata"
                | "windows"
                | "programdata"
                | "program files"
                | "program files (x86)"
        )
}

fn kind(path: &Path, metadata: &Metadata) -> Option<Kind> {
    let name = path.file_name()?.to_str()?;
    if metadata.is_dir() && name.eq_ignore_ascii_case("node_modules") {
        return Some(Kind::NodeDependencies);
    }
    if metadata.is_dir() && matches!(name, ".parcel-cache" | ".vite" | ".turbo") {
        return Some(Kind::NodeCache);
    }
    if metadata.is_dir()
        && name == "cache"
        && path.parent()?.file_name().and_then(|v| v.to_str()) == Some(".next")
    {
        return Some(Kind::NodeCache);
    }
    if metadata.is_file() && matches!(name, ".eslintcache" | ".stylelintcache") {
        return Some(Kind::NodeCache);
    }
    if metadata.is_dir() && name == "__pycache__" {
        return Some(Kind::PythonCache);
    }
    None
}

fn protected_candidate(path: &Path) -> bool {
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    name == ".git"
        || name == ".svn"
        || name == ".hg"
        || name == ".codex"
        || name.starts_with(".codex-")
        || name.starts_with(".dev-companion-recycle-recovery-")
        || name == ".agents"
        || name == ".env"
        || name.starts_with(".env.")
        || matches!(
            name.as_str(),
            "backup" | "backups" | "uploads" | "db" | "database" | "databases"
        )
        || matches!(
            extension.as_str(),
            "db" | "sqlite" | "sqlite3" | "sql" | "bak" | "backup"
        )
}

fn internal_dependency_link(root: &Path, link_path: &Path, link: &recycle::LinkInfo) -> bool {
    let unresolved = if link.target.is_absolute() {
        link.target.clone()
    } else {
        link_path.parent().unwrap_or(root).join(&link.target)
    };
    let mut resolved = PathBuf::new();
    for component in unresolved.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                if !resolved.pop() {
                    return false;
                }
            }
            _ => resolved.push(component.as_os_str()),
        }
    }
    let root_key = normalized(root);
    let resolved_key = normalized(&resolved);
    let link_key = normalized(link_path);
    normalized_descendant(&root_key, &resolved_key)
        && !normalized_descendant(&resolved_key, &link_key)
        && fs_safety::check(&resolved).is_ok()
}

fn snapshot(
    path: &Path,
    allow_links: bool,
    python: bool,
    limits: &mut Limits,
    cancelled: Option<&AtomicBool>,
) -> (Vec<Entry>, usize, u64, Option<String>) {
    let mut entries = Vec::new();
    let mut files = 0;
    let mut bytes = 0;
    let mut blocked = None;
    let mut queue = VecDeque::from([(path.to_path_buf(), 0_usize)]);
    while let Some((current, depth)) = queue.pop_front() {
        if check_cancel(cancelled).is_err() {
            limits.incomplete = true;
            blocked.get_or_insert_with(|| "Scan cancelled.".into());
            break;
        }
        if depth > MAX_DEPTH || limits.entries >= MAX_ENTRIES {
            limits.incomplete = true;
            blocked.get_or_insert_with(|| "Candidate inventory exceeds scan limits.".into());
            break;
        }
        limits.entries += 1;
        let metadata = match fs::symlink_metadata(&current) {
            Ok(value) => value,
            Err(_) => {
                blocked.get_or_insert_with(|| "Candidate contains an inaccessible entry.".into());
                continue;
            }
        };
        if current != path {
            if protected_candidate(&current) {
                blocked.get_or_insert_with(|| {
                    format!("Protected content: {}", relative_display(path, &current))
                });
                continue;
            }
            if python
                && (!metadata.is_file()
                    || current.extension().and_then(|v| v.to_str()) != Some("pyc"))
            {
                blocked.get_or_insert_with(|| "Python cache contains an unknown child.".into());
                continue;
            }
        }
        if fs_safety::linked(&metadata) {
            if current == path || !allow_links {
                blocked.get_or_insert_with(|| {
                    format!(
                        "Symbolic link or junction outside dependency contents: {}",
                        relative_display(path, &current)
                    )
                });
                continue;
            }
            match recycle::inspect_link(&current) {
                Ok(link) if internal_dependency_link(path, &current, &link) => {
                    entries.push(Entry {
                        relative: current.strip_prefix(path).unwrap().to_path_buf(),
                        directory: link.directory,
                        bytes: 0,
                        modified: metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH),
                        hash: None,
                        link: Some(link),
                    });
                    if entries.last().is_some_and(|entry| entry.directory) {
                        limits.directories += 1;
                    } else {
                        files += 1;
                    }
                }
                Ok(_) => {
                    blocked.get_or_insert_with(|| {
                        format!(
                            "Dependency link target is outside node_modules or crosses another link: {}",
                            relative_display(path, &current)
                        )
                    });
                }
                Err(error) => {
                    blocked.get_or_insert(error);
                }
            }
            continue;
        }
        match metadata_entry(path, &current, &metadata, false, cancelled) {
            Ok(entry) => {
                if entry.directory {
                    limits.directories += 1;
                    if limits.directories > MAX_DIRECTORIES {
                        limits.incomplete = true;
                        blocked.get_or_insert_with(|| {
                            "Candidate inventory exceeds directory limits.".into()
                        });
                        break;
                    }
                    match fs::read_dir(&current) {
                        Ok(children) => {
                            for child in children {
                                if check_cancel(cancelled).is_err() {
                                    limits.incomplete = true;
                                    blocked.get_or_insert_with(|| "Scan cancelled.".into());
                                    break;
                                }
                                if limits.entries.saturating_add(queue.len()) >= MAX_ENTRIES {
                                    limits.incomplete = true;
                                    blocked.get_or_insert_with(|| {
                                        "Candidate inventory exceeds scan limits.".into()
                                    });
                                    break;
                                }
                                match child {
                                    Ok(child) => queue.push_back((child.path(), depth + 1)),
                                    Err(_) => {
                                        blocked.get_or_insert_with(|| {
                                            "Candidate contains an inaccessible entry.".into()
                                        });
                                    }
                                }
                            }
                        }
                        Err(_) => {
                            blocked.get_or_insert_with(|| {
                                "Candidate contains an inaccessible directory.".into()
                            });
                        }
                    }
                } else {
                    files += 1;
                    bytes += entry.bytes;
                }
                entries.push(entry);
            }
            Err(error) => {
                blocked.get_or_insert(error);
            }
        }
    }
    entries.sort_by(|a, b| a.relative.cmp(&b.relative));
    (entries, files, bytes, blocked)
}

fn python_markers(
    cache: &Path,
    entries: &[Entry],
    cancelled: Option<&AtomicBool>,
) -> Result<Vec<Marker>, String> {
    let source_directory = cache
        .parent()
        .ok_or("Python cache has no source directory.")?;
    let mut seen = HashSet::new();
    let mut markers = Vec::new();
    for entry in entries
        .iter()
        .filter(|entry| !entry.directory && entry.link.is_none())
    {
        let stem = entry
            .relative
            .file_stem()
            .and_then(|value| value.to_str())
            .ok_or("Python cache filename is unsupported.")?;
        let source_stem = [".cpython-", ".pypy", ".python-"]
            .into_iter()
            .filter_map(|tag| stem.find(tag))
            .min()
            .map(|index| &stem[..index])
            .unwrap_or(stem);
        if source_stem.is_empty() {
            return Err("Python cache filename has no source stem.".into());
        }
        let source = ["py", "pyw"]
            .into_iter()
            .map(|extension| source_directory.join(format!("{source_stem}.{extension}")))
            .find(|path| path.exists())
            .ok_or("Python bytecode has no matching adjacent source file.")?;
        if seen.insert(source.clone()) {
            fs_safety::check(&source)?;
            let metadata = fs::symlink_metadata(&source).map_err(|e| e.to_string())?;
            if fs_safety::linked(&metadata) || !metadata.is_file() {
                return Err("Python source marker is not a regular file.".into());
            }
            markers.push(Marker {
                path: source.clone(),
                entry: metadata_entry(
                    source.parent().unwrap_or(source_directory),
                    &source,
                    &metadata,
                    true,
                    cancelled,
                )?,
            });
        }
    }
    Ok(markers)
}

fn candidate_at(
    root: &Path,
    path: PathBuf,
    kind: Kind,
    id: String,
    limits: &mut Limits,
    cancelled: Option<&AtomicBool>,
) -> Candidate {
    let (entries, files, bytes, tree_blocked) = snapshot(
        &path,
        kind == Kind::NodeDependencies,
        kind == Kind::PythonCache,
        limits,
        cancelled,
    );
    let markers = if kind == Kind::PythonCache {
        python_markers(&path, &entries, cancelled)
    } else {
        project_manifest(root, &path, cancelled).map(|marker| vec![marker])
    };
    let marker_error = markers.as_ref().err().cloned();
    let blocked = tree_blocked.or(marker_error);
    Candidate {
        dto: CandidateDto {
            id,
            path: relative_display(root, &path),
            kind: kind.value().into(),
            files,
            bytes,
            blocked,
        },
        path,
        kind,
        markers: markers.unwrap_or_default(),
        entries,
    }
}

fn discover(
    root: &Path,
    cancelled: Option<&AtomicBool>,
) -> Result<(Vec<Candidate>, Limits), String> {
    let mut candidates = Vec::new();
    let mut limits = Limits::default();
    let managed = managed_roots();
    let mut queue = VecDeque::from([(root.to_path_buf(), 0_usize)]);
    while let Some((directory, depth)) = queue.pop_front() {
        check_cancel(cancelled)?;
        if depth > MAX_DEPTH
            || limits.entries >= MAX_ENTRIES
            || limits.directories >= MAX_DIRECTORIES
        {
            limits.incomplete = true;
            break;
        }
        limits.directories += 1;
        let children = match fs::read_dir(&directory) {
            Ok(value) => value,
            Err(_) => {
                limits.skipped += 1;
                limits.incomplete = true;
                continue;
            }
        };
        for child in children {
            check_cancel(cancelled)?;
            if limits.entries >= MAX_ENTRIES {
                limits.incomplete = true;
                break;
            }
            limits.entries += 1;
            let child = match child {
                Ok(value) => value,
                Err(_) => {
                    limits.skipped += 1;
                    limits.incomplete = true;
                    continue;
                }
            };
            let path = child.path();
            let metadata = match fs::symlink_metadata(&path) {
                Ok(value) => value,
                Err(_) => {
                    limits.skipped += 1;
                    limits.incomplete = true;
                    continue;
                }
            };
            if fs_safety::linked(&metadata) {
                limits.skipped += 1;
                continue;
            }
            if metadata.is_dir() && within_managed(&path, &managed) {
                limits.skipped += 1;
                continue;
            }
            let name = path
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("");
            if metadata.is_dir() && name == ".next" {
                let cache = path.join("cache");
                match fs::symlink_metadata(&cache) {
                    Ok(cache_metadata)
                        if cache_metadata.is_dir() && !fs_safety::linked(&cache_metadata) =>
                    {
                        if candidates.len() == MAX_CANDIDATES {
                            limits.incomplete = true;
                            return Ok((candidates, limits));
                        }
                        let id = format!("candidate-{}", candidates.len() + 1);
                        candidates.push(candidate_at(
                            root,
                            cache,
                            Kind::NodeCache,
                            id,
                            &mut limits,
                            cancelled,
                        ));
                    }
                    Ok(_) => limits.skipped += 1,
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
                    Err(_) => {
                        limits.skipped += 1;
                        limits.incomplete = true;
                    }
                }
                continue;
            }
            if let Some(kind) = kind(&path, &metadata) {
                if candidates.len() == MAX_CANDIDATES {
                    limits.incomplete = true;
                    return Ok((candidates, limits));
                }
                let id = format!("candidate-{}", candidates.len() + 1);
                candidates.push(candidate_at(root, path, kind, id, &mut limits, cancelled));
                continue;
            }
            if metadata.is_dir() {
                if protected(name) {
                    limits.skipped += 1;
                } else {
                    queue.push_back((path, depth + 1));
                }
            }
        }
    }
    Ok((candidates, limits))
}

fn refresh(root: &Path, saved: &Candidate) -> Result<Candidate, String> {
    let mut limits = Limits::default();
    let current = candidate_at(
        root,
        saved.path.clone(),
        saved.kind,
        saved.dto.id.clone(),
        &mut limits,
        None,
    );
    if limits.incomplete || current.dto.blocked.is_some() {
        return Err(format!(
            "Candidate changed or became unsafe: {}",
            saved.dto.path
        ));
    }
    if current.markers != saved.markers || current.entries != saved.entries {
        return Err(format!("Candidate changed after scan: {}", saved.dto.path));
    }
    Ok(current)
}

pub fn prepare_scan() -> Result<String, String> {
    let request = token("request");
    let mut active = ACTIVE_SCAN
        .get_or_init(Default::default)
        .lock()
        .map_err(|_| "Cleanup scan state is unavailable.".to_string())?;
    if let Some(previous) = active.take() {
        previous.cancelled.store(true, Ordering::Relaxed);
    }
    *active = Some(ActiveScan {
        token: request.clone(),
        cancelled: Arc::new(AtomicBool::new(false)),
    });
    *plans().lock().unwrap() = Plans::default();
    Ok(request)
}

pub fn scan(request_token: &str, root: &str) -> Result<Scan, String> {
    let flag = {
        let active = ACTIVE_SCAN
            .get_or_init(Default::default)
            .lock()
            .map_err(|_| "Cleanup scan state is unavailable.".to_string())?;
        let request = active
            .as_ref()
            .filter(|request| request.token == request_token)
            .ok_or("Scan request is invalid, cancelled, or expired.")?;
        request.cancelled.clone()
    };
    let work = (|| {
        check_cancel(Some(&flag))?;
        let root = validate_root(root)?;
        let (candidates, limits) = discover(&root, Some(&flag))?;
        check_cancel(Some(&flag))?;
        let scan_token = token("scan");
        let dto = candidates
            .iter()
            .map(|candidate| candidate.dto.clone())
            .collect();
        let result = Scan {
            token: scan_token.clone(),
            root: display(&root),
            candidates: dto,
            skipped: limits.skipped,
            incomplete: limits.incomplete,
        };
        let plan = ScanPlan {
            token: scan_token,
            root,
            candidates,
            incomplete: result.incomplete,
        };
        Ok((result, plan))
    })();
    let mut active = ACTIVE_SCAN.get_or_init(Default::default).lock().unwrap();
    match work {
        Ok((result, plan))
            if active.as_ref().is_some_and(|request| {
                request.token == request_token && !request.cancelled.load(Ordering::Relaxed)
            }) =>
        {
            plans().lock().unwrap().scan = Some(plan);
            *active = None;
            Ok(result)
        }
        Ok(_) => Err("Scan request is invalid, cancelled, or expired.".into()),
        Err(error) => {
            if active
                .as_ref()
                .is_some_and(|request| request.token == request_token)
            {
                *active = None;
            }
            Err(error)
        }
    }
}

pub fn preview(scan_token: &str, ids: &[String], mode: Mode) -> Result<Preview, String> {
    if ids.is_empty() {
        return Err("Select at least one cleanup candidate.".into());
    }
    let unique: HashSet<&str> = ids.iter().map(String::as_str).collect();
    if unique.len() != ids.len() {
        return Err("Duplicate candidate IDs are not allowed.".into());
    }
    let mut guard = plans().lock().unwrap();
    let saved = guard.scan.as_ref().ok_or("Scan again.")?;
    if saved.token != scan_token {
        return Err("Scan token is invalid or expired.".into());
    }
    if saved.incomplete {
        return Err("The root scan was incomplete; scan a smaller folder.".into());
    }
    let mut selected = Vec::with_capacity(ids.len());
    for id in ids {
        let candidate = saved
            .candidates
            .iter()
            .find(|candidate| candidate.dto.id == *id)
            .ok_or("Unknown candidate ID.")?;
        if let Some(reason) = &candidate.dto.blocked {
            return Err(format!("Blocked candidate: {reason}"));
        }
        selected.push(refresh(&saved.root, candidate)?);
    }
    let root = saved.root.clone();
    let token = token("preview");
    let files = selected.iter().map(|candidate| candidate.dto.files).sum();
    let bytes = selected.iter().map(|candidate| candidate.dto.bytes).sum();
    let dto = selected
        .iter()
        .map(|candidate| candidate.dto.clone())
        .collect();
    guard.preview = Some(PreviewPlan {
        token: token.clone(),
        root: root.clone(),
        candidates: selected,
        mode,
    });
    drop(guard);
    Ok(Preview {
        token,
        root: display(&root),
        candidates: dto,
        files,
        bytes,
        mode,
    })
}

fn entry_matches(root: &Path, entry: &Entry) -> bool {
    let path = root.join(&entry.relative);
    if let Some(link) = &entry.link {
        return path
            .parent()
            .is_some_and(|parent| fs_safety::check(parent).is_ok())
            && recycle::inspect_link(&path).ok().as_ref() == Some(link);
    }
    fs_safety::check(&path).is_ok()
        && fs::symlink_metadata(&path).is_ok_and(|metadata| {
            !fs_safety::linked(&metadata)
                && metadata.is_dir() == entry.directory
                && (entry.directory || metadata.len() == entry.bytes)
                && metadata.modified().ok() == Some(entry.modified)
                && entry
                    .hash
                    .map(|hash| hash_file(&path, None).ok() == Some(hash))
                    .unwrap_or(true)
        })
}

fn delete_candidate(candidate: &Candidate, outcome: &mut Outcome) -> bool {
    let mut links: Vec<&Entry> = candidate
        .entries
        .iter()
        .filter(|entry| entry.link.is_some())
        .collect();
    links.sort_by(|a, b| b.relative.cmp(&a.relative));
    for entry in links {
        if !entry_matches(&candidate.path, entry) {
            outcome.errors.push(format!(
                "Dependency link changed before deletion: {}",
                display(&candidate.path.join(&entry.relative))
            ));
            return false;
        }
        let path = candidate.path.join(&entry.relative);
        let result = if entry.directory {
            fs::remove_dir(&path)
        } else {
            fs::remove_file(&path)
        };
        if let Err(error) = result {
            outcome.errors.push(format!(
                "Cannot remove dependency link {}: {error}",
                display(&path)
            ));
            return false;
        }
        if entry.directory {
            outcome.removed_directories += 1;
        } else {
            outcome.deleted_files += 1;
        }
    }
    let mut files: Vec<&Entry> = candidate
        .entries
        .iter()
        .filter(|entry| !entry.directory && entry.link.is_none())
        .collect();
    files.sort_by(|a, b| b.relative.cmp(&a.relative));
    for entry in files {
        if !entry_matches(&candidate.path, entry) {
            outcome.errors.push(format!(
                "Entry changed before deletion: {}",
                display(&candidate.path.join(&entry.relative))
            ));
            return false;
        }
        let path = candidate.path.join(&entry.relative);
        if let Err(error) = fs::remove_file(&path) {
            outcome
                .errors
                .push(format!("Cannot delete {}: {error}", display(&path)));
            return false;
        }
        outcome.deleted_files += 1;
        outcome.reclaimed_bytes += entry.bytes;
    }
    let mut directories: Vec<&Entry> = candidate
        .entries
        .iter()
        .filter(|entry| entry.directory && entry.link.is_none())
        .collect();
    directories.sort_by_key(|entry| std::cmp::Reverse(entry.relative.components().count()));
    for entry in directories {
        let path = candidate.path.join(&entry.relative);
        let safe_empty = fs_safety::check(&path).is_ok()
            && fs::symlink_metadata(&path)
                .is_ok_and(|metadata| metadata.is_dir() && !fs_safety::linked(&metadata))
            && fs::read_dir(&path).is_ok_and(|mut children| children.next().is_none());
        if !safe_empty {
            outcome.errors.push(format!(
                "Directory changed or is not empty: {}",
                display(&path)
            ));
            return false;
        }
        if let Err(error) = fs::remove_dir(&path) {
            outcome
                .errors
                .push(format!("Cannot remove {}: {error}", display(&path)));
            return false;
        }
        outcome.removed_directories += 1;
    }
    true
}

fn execute_with_recycler<F>(
    preview_token: &str,
    confirmation: &str,
    mut recycle_candidate: F,
) -> Result<Outcome, String>
where
    F: FnMut(&Path, &Candidate) -> Result<Option<String>, String>,
{
    let mut guard = plans().lock().unwrap();
    let saved = guard.preview.as_ref().ok_or("Preview again.")?;
    if saved.token != preview_token {
        return Err("Preview token is invalid or expired.".into());
    }
    let expected = match saved.mode {
        Mode::Permanent => DELETE_CONFIRMATION,
        Mode::Recycle => RECYCLE_CONFIRMATION,
    };
    if confirmation != expected {
        return Err(format!("Type {expected} to confirm."));
    }
    let plan = guard.preview.take().unwrap();
    drop(guard);
    let root = validate_root(&display(&plan.root))?;
    if root != plan.root {
        return Err("Cleanup root changed after preview.".into());
    }
    let mut validated = Vec::with_capacity(plan.candidates.len());
    for candidate in &plan.candidates {
        validated.push(refresh(&root, candidate)?);
    }
    let mut outcome = Outcome {
        mode: plan.mode,
        ..Outcome::default()
    };
    for candidate in &validated {
        match plan.mode {
            Mode::Permanent => {
                if !delete_candidate(candidate, &mut outcome) {
                    break;
                }
            }
            Mode::Recycle => {
                let refreshed = match refresh(&root, candidate) {
                    Ok(candidate) => candidate,
                    Err(error) => {
                        outcome.errors.push(format!(
                            "Candidate changed before recycling {}: {error}",
                            display(&candidate.path)
                        ));
                        break;
                    }
                };
                let warning = match recycle_candidate(&root, &refreshed) {
                    Ok(warning) => warning,
                    Err(error) => {
                        outcome.errors.push(format!(
                            "Cannot recycle {}: {error}",
                            display(&refreshed.path)
                        ));
                        break;
                    }
                };
                outcome.recycled_files += refreshed.dto.files;
                outcome.recycled_directories += refreshed
                    .entries
                    .iter()
                    .filter(|entry| entry.directory)
                    .count();
                outcome.recycled_bytes += refreshed.dto.bytes;
                if let Some(warning) = warning {
                    outcome.errors.push(warning);
                    break;
                }
            }
        }
    }
    Ok(outcome)
}

pub fn execute(preview_token: &str, confirmation: &str) -> Result<Outcome, String> {
    execute_with_recycler(preview_token, confirmation, |root, candidate| {
        recycle::move_to_bin(&candidate.path, || refresh(root, candidate).map(|_| ()))
    })
}

pub fn cancel() -> Result<(), String> {
    let mut active_scan = ACTIVE_SCAN
        .get_or_init(Default::default)
        .lock()
        .map_err(|_| "Cleanup scan state is unavailable.".to_string())?;
    if let Some(active) = active_scan.take() {
        active.cancelled.store(true, Ordering::Relaxed);
    }
    *plans()
        .lock()
        .map_err(|_| "Cleanup state is unavailable.".to_string())? = Plans::default();
    drop(active_scan);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    static TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    fn test_lock() -> std::sync::MutexGuard<'static, ()> {
        TEST_LOCK
            .get_or_init(Default::default)
            .lock()
            .unwrap_or_else(|error| error.into_inner())
    }

    fn fixture(name: &str) -> PathBuf {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("project-cleanup-tests")
            .join(format!("{name}-{}", token("test")));
        fs::create_dir_all(&root).unwrap();
        root
    }

    fn run_scan(root: &Path) -> Scan {
        let request = prepare_scan().unwrap();
        scan(&request, &display(root)).unwrap()
    }

    #[test]
    fn preserves_source_lockfiles_and_deletes_only_selected_generated_tree() {
        let _guard = test_lock();
        let root = fixture("selective");
        fs::write(root.join("package.json"), r#"{"name":"test"}"#).unwrap();
        fs::write(root.join("pnpm-lock.yaml"), "lockfileVersion: 9").unwrap();
        fs::write(root.join("index.ts"), "source").unwrap();
        fs::create_dir(root.join("node_modules")).unwrap();
        fs::write(root.join("node_modules/a.js"), "generated").unwrap();
        fs::create_dir(root.join(".turbo")).unwrap();
        fs::write(root.join(".turbo/cache"), "generated cache").unwrap();
        fs::create_dir_all(root.join(".next/cache")).unwrap();
        fs::write(root.join(".next/cache/index"), "generated next cache").unwrap();
        fs::create_dir_all(root.join(".next/standalone/node_modules")).unwrap();
        fs::write(root.join(".next/standalone/package.json"), "{}").unwrap();
        fs::write(
            root.join(".next/standalone/node_modules/sentinel"),
            "deployment output",
        )
        .unwrap();
        let scan = run_scan(&root);
        assert!(scan
            .candidates
            .iter()
            .any(|candidate| candidate.path == ".next/cache"));
        assert!(!scan
            .candidates
            .iter()
            .any(|candidate| candidate.path.contains("standalone")));
        let node = scan
            .candidates
            .iter()
            .find(|c| c.kind == "nodeDependencies")
            .unwrap();
        assert_eq!(node.path, "node_modules");
        assert!(!scan.root.starts_with(r"\\?\"));
        let preview =
            preview(&scan.token, std::slice::from_ref(&node.id), Mode::Permanent).unwrap();
        let result = execute(&preview.token, DELETE_CONFIRMATION).unwrap();
        assert_eq!(result.deleted_files, 1);
        assert!(!root.join("node_modules").exists());
        assert!(root.join(".turbo/cache").exists());
        assert!(root.join(".next/standalone/node_modules/sentinel").exists());
        assert!(root.join("package.json").exists());
        assert!(root.join("pnpm-lock.yaml").exists());
        assert!(root.join("index.ts").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn recycle_mode_binds_confirmation_and_counts_only_confirmed_candidate() {
        let _guard = test_lock();
        let root = fixture("recycle-mode");
        fs::write(root.join("package.json"), "{}").unwrap();
        for name in ["node_modules", ".turbo"] {
            fs::create_dir(root.join(name)).unwrap();
            fs::write(root.join(name).join("cache"), name).unwrap();
        }
        let scan = run_scan(&root);
        let selected = scan
            .candidates
            .iter()
            .find(|candidate| candidate.path == "node_modules")
            .unwrap();
        let preview = preview(
            &scan.token,
            std::slice::from_ref(&selected.id),
            Mode::Recycle,
        )
        .unwrap();
        assert_eq!(preview.mode, Mode::Recycle);
        assert!(
            execute_with_recycler(&preview.token, DELETE_CONFIRMATION, |_, _| Ok(None)).is_err()
        );

        let held = root.join("held-node-modules");
        let mut recycled_path = None;
        let outcome =
            execute_with_recycler(&preview.token, RECYCLE_CONFIRMATION, |_, candidate| {
                recycled_path = Some(candidate.path.clone());
                fs::rename(&candidate.path, &held)
                    .map(|()| None)
                    .map_err(|error| error.to_string())
            })
            .unwrap();
        let expected_path = root.join("node_modules");
        assert_eq!(
            recycled_path.as_deref().map(display),
            Some(display(&expected_path))
        );
        assert_eq!(outcome.mode, Mode::Recycle);
        assert_eq!(outcome.recycled_files, 1);
        assert_eq!(outcome.recycled_directories, 1);
        assert_eq!(outcome.recycled_bytes, "node_modules".len() as u64);
        assert_eq!(outcome.deleted_files, 0);
        assert_eq!(outcome.removed_directories, 0);
        assert_eq!(outcome.reclaimed_bytes, 0);
        assert!(outcome.errors.is_empty());
        assert!(held.join("cache").exists());
        assert!(root.join(".turbo/cache").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn failed_recycler_never_permanently_deletes_or_credits_candidate() {
        let _guard = test_lock();
        let root = fixture("recycle-failure");
        fs::write(root.join("package.json"), "{}").unwrap();
        fs::create_dir(root.join("node_modules")).unwrap();
        fs::write(root.join("node_modules/generated"), "keep").unwrap();
        let scan = run_scan(&root);
        let preview =
            preview(&scan.token, &[scan.candidates[0].id.clone()], Mode::Recycle).unwrap();
        let outcome = execute_with_recycler(&preview.token, RECYCLE_CONFIRMATION, |_, _| {
            Err("simulated recycle failure".into())
        })
        .unwrap();
        assert_eq!(outcome.mode, Mode::Recycle);
        assert_eq!(outcome.recycled_files, 0);
        assert_eq!(outcome.recycled_directories, 0);
        assert_eq!(outcome.recycled_bytes, 0);
        assert_eq!(outcome.deleted_files, 0);
        assert_eq!(outcome.removed_directories, 0);
        assert_eq!(outcome.reclaimed_bytes, 0);
        assert_eq!(outcome.errors.len(), 1);
        assert!(root.join("node_modules/generated").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn confirmed_recycle_with_stage_cleanup_warning_keeps_truthful_counts() {
        let _guard = test_lock();
        let root = fixture("recycle-warning");
        fs::write(root.join("package.json"), "{}").unwrap();
        fs::create_dir(root.join("node_modules")).unwrap();
        fs::write(root.join("node_modules/generated"), "moved").unwrap();
        let scan = run_scan(&root);
        let preview =
            preview(&scan.token, &[scan.candidates[0].id.clone()], Mode::Recycle).unwrap();
        let held = root.join("confirmed-bin-item");
        let outcome =
            execute_with_recycler(&preview.token, RECYCLE_CONFIRMATION, |_, candidate| {
                fs::rename(&candidate.path, &held).map_err(|error| error.to_string())?;
                Ok(Some("Recovery stage cleanup warning.".into()))
            })
            .unwrap();
        assert_eq!(outcome.recycled_files, 1);
        assert_eq!(outcome.recycled_directories, 1);
        assert_eq!(outcome.recycled_bytes, 5);
        assert_eq!(outcome.errors, vec!["Recovery stage cleanup warning."]);
        assert!(held.join("generated").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn recycle_preflight_rejects_new_file_before_invoking_recycler() {
        let _guard = test_lock();
        let root = fixture("recycle-change");
        fs::write(root.join("package.json"), "{}").unwrap();
        fs::create_dir(root.join("node_modules")).unwrap();
        fs::write(root.join("node_modules/original"), "keep").unwrap();
        let scan = run_scan(&root);
        let preview =
            preview(&scan.token, &[scan.candidates[0].id.clone()], Mode::Recycle).unwrap();
        fs::write(root.join("node_modules/new"), "new after preview").unwrap();
        let mut called = false;
        assert!(
            execute_with_recycler(&preview.token, RECYCLE_CONFIRMATION, |_, _| {
                called = true;
                Ok(None)
            })
            .is_err()
        );
        assert!(!called);
        assert!(root.join("node_modules/original").exists());
        assert!(root.join("node_modules/new").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn changed_tree_or_manifest_aborts_before_any_selected_deletion() {
        let _guard = test_lock();
        for change in ["new", "remove", "manifest", "manifest-remove"] {
            let root = fixture(change);
            fs::write(root.join("package.json"), "{}").unwrap();
            for name in ["node_modules", ".turbo"] {
                fs::create_dir(root.join(name)).unwrap();
                fs::write(root.join(name).join("cache"), name).unwrap();
            }
            let scan = run_scan(&root);
            let ids: Vec<String> = scan.candidates.iter().map(|c| c.id.clone()).collect();
            let preview = preview(&scan.token, &ids, Mode::Permanent).unwrap();
            match change {
                "new" => fs::write(root.join("node_modules/new"), "changed").unwrap(),
                "remove" => fs::remove_file(root.join("node_modules/cache")).unwrap(),
                "manifest" => fs::write(root.join("package.json"), r#"{"changed":true}"#).unwrap(),
                _ => fs::remove_file(root.join("package.json")).unwrap(),
            }
            assert!(execute(&preview.token, DELETE_CONFIRMATION).is_err());
            assert!(root.join(".turbo/cache").exists());
            fs::remove_dir_all(root).unwrap();
        }
    }

    #[test]
    fn deletes_python_cache_only_when_every_child_is_pyc() {
        let _guard = test_lock();
        let root = fixture("python");
        let cache = root.join("src/__pycache__");
        fs::create_dir_all(&cache).unwrap();
        fs::write(cache.join("module.pyc"), "bytecode").unwrap();
        fs::write(root.join("src/module.py"), "source").unwrap();
        let scan = run_scan(&root);
        let preview = preview(
            &scan.token,
            &[scan.candidates[0].id.clone()],
            Mode::Permanent,
        )
        .unwrap();
        assert!(execute(&preview.token, DELETE_CONFIRMATION)
            .unwrap()
            .errors
            .is_empty());
        assert!(!cache.exists());
        assert!(root.join("src/module.py").exists());
        let orphan = root.join("orphan/__pycache__");
        fs::create_dir_all(&orphan).unwrap();
        fs::write(orphan.join("only.cpython-312.pyc"), "bytecode").unwrap();
        let scan = run_scan(&root);
        assert!(scan.candidates[0]
            .blocked
            .as_deref()
            .unwrap()
            .contains("no matching adjacent source"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn queued_scan_cancellation_invalidates_the_request() {
        let _guard = test_lock();
        let root = fixture("cancel");
        let request = prepare_scan().unwrap();
        cancel().unwrap();
        assert!(scan(&request, &display(&root)).is_err());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn accepted_container_prunes_nested_managed_and_other_profile_trees() {
        let _guard = test_lock();
        let root = fixture("container");
        let project = root.join("project");
        fs::create_dir_all(project.join("node_modules")).unwrap();
        fs::write(project.join("package.json"), "{}").unwrap();
        fs::write(project.join("node_modules/generated"), "delete candidate").unwrap();
        for managed in [
            "OtherUser/AppData",
            "Windows",
            "Program Files",
            ".codex-work",
        ] {
            let nested = root.join(managed).join("project/node_modules");
            fs::create_dir_all(&nested).unwrap();
            fs::write(nested.parent().unwrap().join("package.json"), "{}").unwrap();
            fs::write(nested.join("sentinel"), "preserve").unwrap();
        }
        let scan = run_scan(&root);
        assert_eq!(
            scan.candidates
                .iter()
                .map(|candidate| candidate.path.as_str())
                .collect::<Vec<_>>(),
            vec!["project/node_modules"]
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn snapshot_marks_a_seeded_near_limit_inventory_blocked() {
        let _guard = test_lock();
        let root = fixture("limit");
        fs::write(root.join("entry"), "generated").unwrap();
        let mut limits = Limits {
            entries: MAX_ENTRIES - 1,
            ..Limits::default()
        };
        let (_, _, _, blocked) = snapshot(&root, false, false, &mut limits, None);
        assert!(limits.incomplete);
        assert!(blocked.unwrap().contains("limits"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn mid_delete_change_returns_truthful_partial_counts_and_stops() {
        let _guard = test_lock();
        let root = fixture("partial");
        fs::write(root.join("package.json"), "{}").unwrap();
        let generated = root.join("node_modules");
        fs::create_dir(&generated).unwrap();
        fs::write(generated.join("a-changed"), "old").unwrap();
        fs::write(generated.join("z-first"), "gone").unwrap();
        let mut limits = Limits::default();
        let candidate = candidate_at(
            &root,
            generated.clone(),
            Kind::NodeDependencies,
            "candidate-test".into(),
            &mut limits,
            None,
        );
        assert!(candidate.dto.blocked.is_none());
        fs::write(generated.join("a-changed"), "changed after snapshot").unwrap();
        let mut outcome = Outcome::default();
        assert!(!delete_candidate(&candidate, &mut outcome));
        assert_eq!(outcome.deleted_files, 1);
        assert_eq!(outcome.removed_directories, 0);
        assert_eq!(outcome.reclaimed_bytes, 4);
        assert_eq!(outcome.errors.len(), 1);
        assert!(!generated.join("z-first").exists());
        assert!(generated.join("a-changed").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rejects_tokens_duplicates_unknown_ids_and_unsafe_roots() {
        let _guard = test_lock();
        let root = fixture("tokens");
        fs::write(root.join("package.json"), "{}").unwrap();
        fs::create_dir(root.join("node_modules")).unwrap();
        let result = run_scan(&root);
        let id = result.candidates[0].id.clone();
        assert!(preview("wrong", std::slice::from_ref(&id), Mode::Permanent).is_err());
        assert!(preview(&result.token, &[id.clone(), id.clone()], Mode::Permanent).is_err());
        assert!(preview(&result.token, &["candidate-999".into()], Mode::Permanent).is_err());
        let request = prepare_scan().unwrap();
        assert!(scan(&request, "relative").is_err());
        let protected = root.join(".codex");
        fs::create_dir(&protected).unwrap();
        let request = prepare_scan().unwrap();
        assert!(scan(&request, &display(&protected)).is_err());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn protected_trees_are_pruned_and_python_unknown_children_are_blocked() {
        let _guard = test_lock();
        let root = fixture("protected");
        fs::write(root.join("package.json"), "{}").unwrap();
        for protected in [".git", ".svn", ".agents", ".codex-work", ".cache"] {
            let nested = root.join(protected).join("node_modules");
            fs::create_dir_all(&nested).unwrap();
            fs::write(nested.join("sentinel"), "keep").unwrap();
        }
        let python = root.join("python/__pycache__");
        fs::create_dir_all(&python).unwrap();
        fs::write(python.join("module.pyc"), "bytecode").unwrap();
        fs::write(python.join("README"), "unknown").unwrap();
        let first = run_scan(&root);
        assert_eq!(first.candidates.len(), 1);
        assert_eq!(first.candidates[0].kind, "pythonCache");
        assert!(first.candidates[0].blocked.is_some());
        let marker = root.join("node_modules/.git");
        fs::create_dir_all(&marker).unwrap();
        fs::write(marker.join("config"), "keep").unwrap();
        let sensitive = root.join(".turbo/backups");
        fs::create_dir_all(&sensitive).unwrap();
        fs::write(sensitive.join("important.zip"), "keep").unwrap();
        let scan = run_scan(&root);
        assert!(scan
            .candidates
            .iter()
            .find(|candidate| candidate.kind == "nodeDependencies")
            .unwrap()
            .blocked
            .as_deref()
            .unwrap()
            .contains("Protected content:"));
        assert!(scan
            .candidates
            .iter()
            .find(|candidate| candidate.path == ".turbo")
            .unwrap()
            .blocked
            .as_deref()
            .unwrap()
            .contains("Protected content:"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn recovery_stages_are_pruned_rejected_and_block_cleanup_candidates() {
        let _guard = test_lock();
        let root = fixture("recovery-protection");
        fs::write(root.join("package.json"), "{}").unwrap();
        let recovery = root.join(".dev-companion-recycle-recovery-owned");
        fs::create_dir_all(recovery.join("project/node_modules")).unwrap();
        fs::write(recovery.join("project/package.json"), "{}").unwrap();
        fs::write(
            recovery.join("project/node_modules/recovery-copy"),
            "preserve",
        )
        .unwrap();
        let nested_recovery = root.join(".turbo/.dev-companion-recycle-recovery-nested");
        fs::create_dir_all(&nested_recovery).unwrap();
        fs::write(nested_recovery.join("recovery-copy"), "preserve").unwrap();

        let result = run_scan(&root);
        assert_eq!(result.candidates.len(), 1);
        assert_eq!(result.candidates[0].path, ".turbo");
        assert!(result.candidates[0]
            .blocked
            .as_deref()
            .unwrap()
            .contains(".dev-companion-recycle-recovery-nested"));
        let request = prepare_scan().unwrap();
        assert!(scan(&request, &display(&recovery)).is_err());
        assert!(recovery.join("project/node_modules/recovery-copy").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn dependency_outputs_and_caches_do_not_block_cleanup() {
        let _guard = test_lock();
        let root = fixture("dependency-output");
        fs::write(root.join("package.json"), "{}").unwrap();
        for name in [
            ".cache",
            "fast-glob/out",
            "@nodelib/fs.scandir/out",
            "pkg/coverage",
            "pkg/.next",
        ] {
            let path = root.join("node_modules").join(name);
            fs::create_dir_all(&path).unwrap();
            fs::write(path.join("generated.js"), "dependency content").unwrap();
        }
        fs::create_dir_all(root.join("out/node_modules")).unwrap();
        fs::write(root.join("out/node_modules/retained"), "project output").unwrap();
        let scan = run_scan(&root);
        assert_eq!(scan.candidates.len(), 1);
        assert!(scan.candidates[0].blocked.is_none());
        let preview = preview(
            &scan.token,
            &[scan.candidates[0].id.clone()],
            Mode::Permanent,
        )
        .unwrap();
        let result = execute(&preview.token, DELETE_CONFIRMATION).unwrap();
        assert_eq!(result.deleted_files, 5);
        assert!(result.errors.is_empty());
        assert!(!root.join("node_modules").exists());
        assert!(root.join("out/node_modules/retained").exists());
        assert!(root.join("package.json").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn sensitive_content_remains_blocked_with_its_exact_path() {
        let _guard = test_lock();
        for name in ["pkg/.env", "pkg/data.sqlite", "pkg/backups/important.zip"] {
            let root = fixture("sensitive-output");
            fs::write(root.join("package.json"), "{}").unwrap();
            let target = root.join("node_modules").join(name);
            fs::create_dir_all(target.parent().unwrap()).unwrap();
            fs::write(&target, "keep").unwrap();
            let scan = run_scan(&root);
            let candidate = &scan.candidates[0];
            let reason = candidate.blocked.as_deref().unwrap();
            assert!(reason.contains(if name.contains("backups") {
                "pkg/backups"
            } else {
                name
            }));
            assert!(preview(&scan.token, &[candidate.id.clone()], Mode::Permanent).is_err());
            assert!(target.exists());
            fs::remove_dir_all(root).unwrap();
        }
    }

    #[test]
    #[ignore = "Set DEV_COMPANION_CLEANUP_SCAN_ROOT for an explicit read-only real project scan"]
    fn real_project_scan_is_read_only() {
        let _guard = test_lock();
        let root = std::env::var("DEV_COMPANION_CLEANUP_SCAN_ROOT").unwrap();
        let request = prepare_scan().unwrap();
        let result = scan(&request, &root).unwrap();
        println!(
            "incomplete={} skipped={}",
            result.incomplete, result.skipped
        );
        for candidate in result.candidates {
            println!(
                "{}: files={} bytes={} blocked={:?}",
                candidate.path, candidate.files, candidate.bytes, candidate.blocked
            );
            if candidate.kind == "nodeDependencies" {
                assert!(candidate.blocked.is_none());
            }
        }
        assert!(!result.incomplete);
    }

    #[cfg(windows)]
    #[test]
    fn pnpm_internal_junction_and_hardlink_are_safe_permanent_entries() {
        let _guard = test_lock();
        let root = fixture("pnpm-links");
        fs::write(root.join("package.json"), "{}").unwrap();
        let store_package = root
            .join("node_modules")
            .join(".pnpm")
            .join("pkg")
            .join("node_modules")
            .join("pkg");
        fs::create_dir_all(&store_package).unwrap();
        fs::write(store_package.join("index.js"), "generated").unwrap();
        let junction = root.join("node_modules").join("autoprefixer");
        let status = std::process::Command::new("cmd")
            .args(["/d", "/c", "mklink", "/J"])
            .arg(display(&junction))
            .arg(display(&store_package))
            .status()
            .unwrap();
        assert!(status.success());
        let file_target = store_package.join("license.txt");
        fs::write(&file_target, "license").unwrap();
        let outside = fixture("hardlink-outside");
        let outside_sentinel = outside.join("sentinel.txt");
        fs::write(&outside_sentinel, "outside store").unwrap();
        fs::hard_link(&outside_sentinel, store_package.join("hardlinked.txt")).unwrap();

        let scan = run_scan(&root);
        let candidate = scan
            .candidates
            .iter()
            .find(|candidate| candidate.path == "node_modules")
            .unwrap();
        assert!(candidate.blocked.is_none(), "{:?}", candidate.blocked);
        assert_eq!(candidate.files, 3);
        let preview = preview(&scan.token, &[candidate.id.clone()], Mode::Permanent).unwrap();
        let outcome = execute(&preview.token, DELETE_CONFIRMATION).unwrap();
        assert!(outcome.errors.is_empty(), "{:?}", outcome.errors);
        assert_eq!(outcome.deleted_files, 3);
        assert_eq!(outcome.removed_directories, 6);
        assert!(!root.join("node_modules").exists());
        assert!(root.join("package.json").exists());
        assert_eq!(
            fs::read_to_string(&outside_sentinel).unwrap(),
            "outside store"
        );
        fs::remove_file(root.join("package.json")).unwrap();
        fs::remove_dir(root).unwrap();
        fs::remove_dir_all(outside).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn pnpm_link_to_candidate_ancestor_is_blocked_as_a_cycle() {
        let _guard = test_lock();
        let root = fixture("pnpm-cycle");
        fs::write(root.join("package.json"), "{}").unwrap();
        let candidate = root.join("node_modules");
        fs::create_dir_all(candidate.join(".pnpm")).unwrap();
        let cycle = candidate.join(".pnpm").join("self");
        let status = std::process::Command::new("cmd")
            .args(["/d", "/c", "mklink", "/J"])
            .arg(display(&cycle))
            .arg(display(&candidate))
            .status()
            .unwrap();
        assert!(status.success());

        let scan = run_scan(&root);
        assert!(scan.candidates[0]
            .blocked
            .as_deref()
            .is_some_and(|reason| reason.contains("outside node_modules")));

        fs::remove_dir(cycle).unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn retargeted_pnpm_junction_aborts_before_deletion() {
        let _guard = test_lock();
        let root = fixture("pnpm-retarget");
        fs::write(root.join("package.json"), "{}").unwrap();
        let candidate = root.join("node_modules");
        let target_a = candidate
            .join(".pnpm")
            .join("a")
            .join("node_modules")
            .join("a");
        let target_b = candidate
            .join(".pnpm")
            .join("b")
            .join("node_modules")
            .join("b");
        fs::create_dir_all(&target_a).unwrap();
        fs::create_dir_all(&target_b).unwrap();
        fs::write(target_a.join("sentinel-a"), "a").unwrap();
        fs::write(target_b.join("sentinel-b"), "b").unwrap();
        let junction = candidate.join("pkg");
        let create_junction = |target: &Path| {
            std::process::Command::new("cmd")
                .args(["/d", "/c", "mklink", "/J"])
                .arg(display(&junction))
                .arg(display(target))
                .status()
                .unwrap()
        };
        assert!(create_junction(&target_a).success());

        let scan = run_scan(&root);
        let selected = scan
            .candidates
            .iter()
            .find(|item| item.path == "node_modules")
            .unwrap();
        assert!(selected.blocked.is_none(), "{:?}", selected.blocked);
        let preview = preview(&scan.token, &[selected.id.clone()], Mode::Permanent).unwrap();
        fs::remove_dir(&junction).unwrap();
        assert!(create_junction(&target_b).success());

        match execute(&preview.token, DELETE_CONFIRMATION) {
            Err(error) => assert!(error.contains("changed"), "{error}"),
            Ok(_) => panic!("retargeted junction was accepted"),
        }
        assert_eq!(
            fs::read_to_string(target_a.join("sentinel-a")).unwrap(),
            "a"
        );
        assert_eq!(
            fs::read_to_string(target_b.join("sentinel-b")).unwrap(),
            "b"
        );

        fs::remove_dir(junction).unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn junction_candidate_is_blocked_and_outside_sentinel_survives() {
        let _guard = test_lock();
        let root = fixture("junction");
        let outside = fixture("outside");
        fs::write(root.join("package.json"), "{}").unwrap();
        fs::write(outside.join("sentinel"), "keep").unwrap();
        fs::create_dir(root.join("node_modules")).unwrap();
        let status = std::process::Command::new("cmd")
            .args(["/c", "mklink", "/J"])
            .arg(display(&root.join("node_modules").join("linked")))
            .arg(display(&outside))
            .status()
            .unwrap();
        assert!(status.success());
        let scan = run_scan(&root);
        assert!(scan.candidates[0]
            .blocked
            .as_deref()
            .unwrap()
            .contains("outside node_modules"));
        fs::remove_dir(root.join("node_modules/linked")).unwrap();
        fs::remove_dir_all(root).unwrap();
        assert_eq!(
            fs::read_to_string(outside.join("sentinel")).unwrap(),
            "keep"
        );
        fs::remove_dir_all(outside).unwrap();
    }
}
