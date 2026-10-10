//! Strict Windows-only XAMPP file bundle. Database data, binaries, and automatic placement are excluded.
use std::{
    collections::{HashMap, HashSet},
    fs::{self, File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        Mutex, OnceLock,
    },
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use zip::{write::SimpleFileOptions, CompressionMethod, ZipArchive, ZipWriter};

use crate::{fs_safety, platform};

type Result<T> = std::result::Result<T, String>;
const MANIFEST: &str = "personal-manifest.json";
const APP: &str = "xampp";
const MAX_MANIFEST: u64 = 4 * 1024 * 1024;
const MAX_FILE: u64 = 64 * 1024 * 1024;
const MAX_CONFIG: u64 = 1024 * 1024;
const MAX_TOTAL: u64 = 512 * 1024 * 1024;
const MAX_FILES: usize = 20_000;
const MAX_CONTAINER_OVERHEAD: u64 = 4 * 1024 * 1024;
const CONFIGS: [&str; 4] = [
    "apache/conf/httpd.conf",
    "apache/conf/extra/httpd-vhosts.conf",
    "php/php.ini",
    "mysql/bin/my.ini",
];

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct InventoryEntry {
    archive_path: String,
    bytes: u64,
    sha256: String,
    source_kind: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Artifact {
    app_id: String,
    adapter_format_version: u32,
    source_app_version: String,
    architecture: String,
    kind: String,
    files: u32,
    bytes: u64,
    restore_mode: String,
    inventory: Vec<InventoryEntry>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Manifest {
    format_version: u32,
    kind: String,
    created_at: String,
    platform: String,
    sensitive: bool,
    artifacts: Vec<Artifact>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Readiness {
    pub supported: bool,
    pub installation_found: bool,
    pub stopped: bool,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Preview {
    pub token: String,
    pub source_app_version: String,
    pub architecture: String,
    pub projects: Vec<String>,
    pub config_files: Vec<String>,
    pub file_count: usize,
    pub bytes: u64,
    pub excluded_count: usize,
    pub sensitive: bool,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Created {
    pub bundle_name: String,
    pub bytes: u64,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Inspection {
    pub token: String,
    pub bundle_name: String,
    pub created_at: String,
    pub source_app_version: String,
    pub architecture: String,
    pub projects: Vec<String>,
    pub file_count: usize,
    pub bytes: u64,
    pub sensitive: bool,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryPreview {
    pub token: String,
    pub staging_path: String,
    pub destination_conflicts: usize,
    pub manual_only: bool,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryResult {
    pub recovered: bool,
    pub staging_path: String,
    pub manual_only: bool,
}

#[derive(Clone, Debug, PartialEq)]
struct SourceFile {
    source: PathBuf,
    entry: InventoryEntry,
}
#[derive(Clone, Debug, PartialEq)]
struct Identity {
    root: PathBuf,
    version: String,
    architecture: String,
    projects: Vec<PathBuf>,
    files: Vec<SourceFile>,
    excluded_count: usize,
}
#[derive(Clone)]
enum Plan {
    Create {
        identity: Identity,
    },
    Inspect {
        bundle: PathBuf,
        hash: String,
    },
    Recover {
        bundle: PathBuf,
        hash: String,
        target: PathBuf,
        destination_state: Vec<(String, bool)>,
    },
}
static PLANS: OnceLock<Mutex<HashMap<String, Plan>>> = OnceLock::new();
static NEXT: AtomicU64 = AtomicU64::new(1);
fn put(plan: Plan) -> String {
    let token = format!("xampp-{}", NEXT.fetch_add(1, Ordering::Relaxed));
    let mut plans = PLANS
        .get_or_init(Default::default)
        .lock()
        .expect("plan lock");
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
        .map_err(|_| "Preview expired; preview again.".to_string())?
        .get(token)
        .cloned()
        .ok_or_else(|| "Preview expired; preview again.".into())
}

fn timestamp() -> Result<String> {
    time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .map_err(|_| "Cannot create bundle timestamp.".into())
}
fn bundle_name(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("xampp-bundle.zip")
        .to_owned()
}
fn valid_hash(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
fn max_bundle_bytes() -> u64 {
    MAX_TOTAL + MAX_MANIFEST + MAX_CONTAINER_OVERHEAD
}
fn open_regular(path: &Path, limit: u64, description: &str) -> Result<File> {
    fs_safety::check(path)?;
    let metadata = fs::symlink_metadata(path).map_err(|_| format!("Cannot read {description}."))?;
    if fs_safety::linked(&metadata) || !metadata.is_file() || metadata.len() > limit {
        return Err(format!("{description} is not a supported regular file."));
    }
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.share_mode(1);
    }
    options
        .open(path)
        .map_err(|_| format!("Cannot read {description}. Stop XAMPP and retry."))
}
fn hash_reader(mut input: impl Read, limit: u64, description: &str) -> Result<(String, u64)> {
    let mut hash = Sha256::new();
    let mut bytes = 0u64;
    let mut buffer = [0u8; 65536];
    loop {
        let count = input
            .read(&mut buffer)
            .map_err(|_| format!("Cannot read {description}."))?;
        if count == 0 {
            break;
        }
        bytes = bytes
            .checked_add(count as u64)
            .ok_or_else(|| format!("{description} exceeds the size limit."))?;
        if bytes > limit {
            return Err(format!("{description} exceeds the size limit."));
        }
        hash.update(&buffer[..count]);
    }
    Ok((format!("{:x}", hash.finalize()), bytes))
}
fn hash_file(path: &Path, limit: u64, description: &str) -> Result<(String, u64)> {
    hash_reader(open_regular(path, limit, description)?, limit, description)
}
fn copy_checked(
    mut input: impl Read,
    mut output: impl Write,
    bytes: u64,
    sha256: &str,
    description: &str,
) -> Result<()> {
    let mut copied = 0u64;
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 65536];
    loop {
        let count = input
            .read(&mut buffer)
            .map_err(|_| format!("Cannot read {description}."))?;
        if count == 0 {
            break;
        }
        copied = copied
            .checked_add(count as u64)
            .ok_or_else(|| format!("{description} exceeds the size limit."))?;
        if copied > bytes {
            return Err(format!("{description} changed; preview again."));
        }
        hash.update(&buffer[..count]);
        output
            .write_all(&buffer[..count])
            .map_err(|_| format!("Cannot write {description}."))?;
    }
    if copied != bytes || format!("{:x}", hash.finalize()) != sha256 {
        return Err(format!("{description} changed; preview again."));
    }
    Ok(())
}
fn canonical_dir(path: &Path, description: &str) -> Result<PathBuf> {
    fs_safety::check(path)?;
    let metadata = fs::symlink_metadata(path).map_err(|_| format!("Cannot find {description}."))?;
    if fs_safety::linked(&metadata) || !metadata.is_dir() {
        return Err(format!("{description} is not a supported directory."));
    }
    fs::canonicalize(path).map_err(|_| format!("Cannot resolve {description}."))
}

pub fn xampp_root() -> Result<PathBuf> {
    #[cfg(not(windows))]
    {
        return Err("XAMPP file migration currently supports Windows only.".into());
    }
    #[cfg(windows)]
    {
        let path = crate::xampp_domains::installation()?;

        let root = canonical_dir(&path, "the XAMPP installation")?;
        canonical_dir(&root.join("htdocs"), "the XAMPP htdocs folder")?;
        Ok(root)
    }
}
fn version_architecture(root: &Path) -> Result<(String, String)> {
    let readme = root.join("readme_en.txt");
    let mut raw = Vec::new();
    open_regular(&readme, MAX_CONFIG, "the XAMPP release notes")?
        .take(MAX_CONFIG + 1)
        .read_to_end(&mut raw)
        .map_err(|_| "Cannot read the XAMPP release notes.".to_string())?;
    let text = std::str::from_utf8(&raw).map_err(|_| "Unsupported XAMPP release notes.")?;
    let version = text
        .lines()
        .find_map(|line| {
            line.trim()
                .strip_prefix("###### ApacheFriends XAMPP Version ")
                .and_then(|value| value.strip_suffix(" ######"))
        })
        .filter(|value| !value.is_empty())
        .ok_or("Unsupported XAMPP release notes.")?
        .to_owned();
    let architecture = if text.to_ascii_lowercase().contains("64bit") {
        "x64"
    } else if text.to_ascii_lowercase().contains("32bit") {
        "x86"
    } else {
        return Err("Cannot determine the XAMPP architecture.".into());
    };
    Ok((version, architecture.into()))
}
fn has_process(output: &str) -> bool {
    output.lines().any(|line| {
        matches!(
            line.split(',')
                .next()
                .unwrap_or("")
                .trim_matches('"')
                .to_ascii_lowercase()
                .as_str(),
            "httpd.exe"
                | "mysqld.exe"
                | "mariadbd.exe"
                | "xampp-control.exe"
                | "xampp_start.exe"
                | "xampp_stop.exe"
        )
    })
}
pub fn require_stopped() -> Result<()> {
    #[cfg(not(windows))]
    {
        return Err("XAMPP file migration currently supports Windows only.".into());
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let output = std::process::Command::new("tasklist.exe")
            .args(["/FO", "CSV", "/NH"])
            .creation_flags(0x08000000)
            .output()
            .map_err(|_| "Cannot check whether Apache, MariaDB, and XAMPP are stopped.")?;
        if !output.status.success() {
            return Err("Cannot check whether Apache, MariaDB, and XAMPP are stopped.".into());
        }
        if has_process(&String::from_utf8_lossy(&output.stdout)) {
            return Err("Stop Apache, MariaDB, and XAMPP-related processes before preview, creation, or recovery. Companion never stops them automatically.".into());
        }
        Ok(())
    }
}
pub fn readiness() -> Readiness {
    let installation_found = xampp_root().is_ok();
    let stopped = require_stopped().is_ok();
    Readiness {
        supported: cfg!(windows),
        installation_found,
        stopped,
    }
}

fn reject_sensitive_text(raw: &[u8]) -> Result<()> {
    let text =
        std::str::from_utf8(raw).map_err(|_| "XAMPP configuration must be reviewed UTF-8 text.")?;
    if text.contains('\0')
        || [
            "password",
            "passwd",
            "secret",
            "token",
            "api_key",
            "api-key",
            "private_key",
            "private-key",
            "credential",
        ]
        .iter()
        .any(|needle| text.to_ascii_lowercase().contains(needle))
    {
        return Err(
            "XAMPP configuration contains a credential or key indicator and is excluded.".into(),
        );
    }
    Ok(())
}
fn config_file(root: &Path, relative: &str) -> Result<SourceFile> {
    let source = root.join(relative.replace('/', "\\"));
    let mut raw = Vec::new();
    open_regular(&source, MAX_CONFIG, "reviewed XAMPP configuration")?
        .take(MAX_CONFIG + 1)
        .read_to_end(&mut raw)
        .map_err(|_| "Cannot read reviewed XAMPP configuration.".to_string())?;
    if raw.len() as u64 > MAX_CONFIG {
        return Err("Reviewed XAMPP configuration exceeds the size limit.".into());
    }
    reject_sensitive_text(&raw)?;
    Ok(SourceFile {
        source,
        entry: InventoryEntry {
            archive_path: format!("apps/xampp/config/{relative}"),
            bytes: raw.len() as u64,
            sha256: format!("{:x}", Sha256::digest(&raw)),
            source_kind: "config".into(),
        },
    })
}
fn excluded_name(name: &str, directory: bool) -> bool {
    let name = name.to_ascii_lowercase();
    (directory
        && matches!(
            name.as_str(),
            ".git"
                | ".hg"
                | ".svn"
                | ".cache"
                | "cache"
                | "logs"
                | "log"
                | "tmp"
                | ".ssh"
                | "keys"
                | "secrets"
                | "credentials"
        ))
        || (!directory
            && (name.ends_with(".log")
                || name.starts_with(".env")
                || name.ends_with(".key")
                || name.ends_with(".pem")
                || name.ends_with(".pfx")
                || name.ends_with(".p12")
                || name.ends_with(".ppk")
                || name.ends_with(".exe")
                || name.ends_with(".dll")
                || matches!(
                    name.as_str(),
                    ".git"
                        | ".gitmodules"
                        | ".htpasswd"
                        | ".npmrc"
                        | "id_rsa"
                        | "id_dsa"
                        | "id_ed25519"
                        | "credentials"
                )))
}
fn safe_component(name: &str) -> bool {
    !name.is_empty() && !name.ends_with(['.', ' ']) && !name.contains([':', '\0']) && {
        let stem = name.split('.').next().unwrap_or("").to_ascii_uppercase();
        !matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
            && !(stem.len() == 4
                && (stem.starts_with("COM") || stem.starts_with("LPT"))
                && stem.as_bytes()[3].is_ascii_digit())
    }
}
fn collect_project(
    dir: &Path,
    relative: &str,
    files: &mut Vec<SourceFile>,
    excluded: &mut usize,
) -> Result<()> {
    for item in fs::read_dir(dir).map_err(|_| "Cannot read the selected htdocs project.")? {
        let item = item.map_err(|_| "Cannot read the selected htdocs project.")?;
        let path = item.path();
        let name = item
            .file_name()
            .to_str()
            .ok_or("Selected htdocs project has a non-Unicode path.")?
            .to_owned();
        if !safe_component(&name) {
            return Err("Selected htdocs project has an unsafe path.".into());
        }
        let metadata =
            fs::symlink_metadata(&path).map_err(|_| "Cannot read the selected htdocs project.")?;
        if fs_safety::linked(&metadata) {
            return Err("Selected htdocs project contains a symbolic link or junction.".into());
        }
        let next = format!("{relative}/{name}");
        if metadata.is_dir() {
            if excluded_name(&name, true) {
                *excluded += 1;
            } else {
                collect_project(&path, &next, files, excluded)?;
            }
        } else if metadata.is_file() {
            if excluded_name(&name, false) {
                *excluded += 1;
            } else {
                if files.len() >= MAX_FILES {
                    return Err("Selected XAMPP files exceed the count limit.".into());
                }
                let (sha256, bytes) = hash_file(&path, MAX_FILE, "selected htdocs project file")?;
                files.push(SourceFile {
                    source: path,
                    entry: InventoryEntry {
                        archive_path: format!("apps/xampp/htdocs/{next}"),
                        bytes,
                        sha256,
                        source_kind: "project".into(),
                    },
                });
            }
        } else {
            return Err("Selected htdocs project contains an unsupported filesystem entry.".into());
        }
    }
    Ok(())
}
fn snapshot_at(root: &Path, selected: &[PathBuf]) -> Result<Identity> {
    let root = canonical_dir(root, "the XAMPP installation")?;
    let htdocs = canonical_dir(&root.join("htdocs"), "the XAMPP htdocs folder")?;
    let (version, architecture) = version_architecture(&root)?;
    if selected.is_empty() {
        return Err("Select one or more direct htdocs project folders.".into());
    }
    let mut projects = Vec::new();
    let mut files = Vec::new();
    let mut excluded_count = 0usize;
    let mut names = HashSet::new();
    for selected_path in selected {
        let project = canonical_dir(selected_path, "the selected htdocs project")?;
        let parent = project
            .parent()
            .ok_or("Selected htdocs project has no parent.")?;
        if parent != htdocs {
            return Err(
                "Select only direct project folders inside this XAMPP htdocs directory.".into(),
            );
        }
        let name = project
            .file_name()
            .and_then(|name| name.to_str())
            .filter(|name| safe_component(name))
            .ok_or("Selected htdocs project has an unsafe name.")?
            .to_owned();
        if !names.insert(name.to_ascii_lowercase()) {
            return Err("Select each htdocs project only once.".into());
        }
        collect_project(&project, &name, &mut files, &mut excluded_count)?;
        projects.push(project);
    }
    for relative in CONFIGS {
        files.push(config_file(&root, relative)?);
    }
    if files.len() > MAX_FILES {
        return Err("Selected XAMPP files exceed the count limit.".into());
    }
    let total = files.iter().try_fold(0u64, |sum, file| {
        sum.checked_add(file.entry.bytes)
            .ok_or("Selected XAMPP files exceed the size limit.")
    })?;
    if total > MAX_TOTAL {
        return Err("Selected XAMPP files exceed the 512 MiB limit.".into());
    }
    Ok(Identity {
        root,
        version,
        architecture,
        projects,
        files,
        excluded_count,
    })
}
fn project_names(identity: &Identity) -> Vec<String> {
    identity
        .projects
        .iter()
        .filter_map(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .map(str::to_owned)
        })
        .collect()
}
pub fn preview(selected: Vec<PathBuf>) -> Result<Preview> {
    require_stopped()?;
    let identity = snapshot_at(&xampp_root()?, &selected)?;
    let token = put(Plan::Create {
        identity: identity.clone(),
    });
    Ok(Preview {
        token,
        source_app_version: identity.version.clone(),
        architecture: identity.architecture.clone(),
        projects: project_names(&identity),
        config_files: CONFIGS.iter().map(|value| (*value).to_owned()).collect(),
        file_count: identity.files.len(),
        bytes: identity.files.iter().map(|file| file.entry.bytes).sum(),
        excluded_count: identity.excluded_count,
        sensitive: true,
    })
}
pub fn create(token: &str) -> Result<Created> {
    require_stopped()?;
    let Plan::Create { identity } = get(token)? else {
        return Err("Preview selected XAMPP files first.".into());
    };
    create_revalidated_at(&identity, &platform::personal_bundle_dir())
}
fn create_revalidated_at(identity: &Identity, output: &Path) -> Result<Created> {
    if snapshot_at(&identity.root, &identity.projects)? != *identity {
        return Err(
            "Selected XAMPP files, configuration, or installation changed; preview again.".into(),
        );
    }
    create_at(identity, output)
}
fn create_at(identity: &Identity, output: &Path) -> Result<Created> {
    fs_safety::check(output)?;
    fs::create_dir_all(output).map_err(|_| "Cannot create the Companion bundle folder.")?;
    fs_safety::check(output)?;
    let path = output.join(format!(
        "xampp-bundle-{}.zip",
        time::OffsetDateTime::now_utc().unix_timestamp_nanos()
    ));
    let mut created = false;
    let attempt = (|| -> Result<()> {
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|_| "Cannot create the XAMPP bundle.")?;
        created = true;
        let total: u64 = identity.files.iter().map(|file| file.entry.bytes).sum();
        let manifest = Manifest {
            format_version: 1,
            kind: "codex-companion-personal-bundle".into(),
            created_at: timestamp()?,
            platform: "windows".into(),
            sensitive: true,
            artifacts: vec![Artifact {
                app_id: APP.into(),
                adapter_format_version: 1,
                source_app_version: identity.version.clone(),
                architecture: identity.architecture.clone(),
                kind: "selected-files".into(),
                files: identity.files.len() as u32,
                bytes: total,
                restore_mode: "manual".into(),
                inventory: identity
                    .files
                    .iter()
                    .map(|file| file.entry.clone())
                    .collect(),
            }],
        };
        let mut zip = ZipWriter::new(file);
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        zip.start_file(MANIFEST, options)
            .map_err(|_| "Cannot create the XAMPP bundle.")?;
        zip.write_all(
            &serde_json::to_vec(&manifest).map_err(|_| "Cannot create the XAMPP bundle.")?,
        )
        .map_err(|_| "Cannot create the XAMPP bundle.")?;
        for source in &identity.files {
            zip.start_file(&source.entry.archive_path, options)
                .map_err(|_| "Cannot create the XAMPP bundle.")?;
            copy_checked(
                open_regular(
                    &source.source,
                    if source.entry.source_kind == "config" {
                        MAX_CONFIG
                    } else {
                        MAX_FILE
                    },
                    "selected XAMPP file",
                )?,
                &mut zip,
                source.entry.bytes,
                &source.entry.sha256,
                "selected XAMPP file",
            )?;
        }
        zip.finish()
            .map_err(|_| "Cannot finalize the XAMPP bundle.")?
            .sync_all()
            .map_err(|_| "Cannot finalize the XAMPP bundle.")?;
        inspect_at(&path).map(|_| ())
    })();
    if let Err(error) = attempt {
        if created && fs::remove_file(&path).is_err() {
            return Err(
                "XAMPP bundle creation failed and the partial file could not be removed.".into(),
            );
        }
        return Err(error);
    }
    Ok(Created {
        bundle_name: bundle_name(&path),
        bytes: fs::metadata(&path)
            .map_err(|_| "Cannot verify the XAMPP bundle.")?
            .len(),
    })
}

fn validate_entry(entry: &InventoryEntry) -> Result<()> {
    fs_safety::relative(&entry.archive_path).map_err(|_| "XAMPP bundle has an unsafe ZIP path.")?;
    let valid_config = CONFIGS
        .iter()
        .any(|path| entry.archive_path == format!("apps/xampp/config/{path}"));
    let valid_project = entry
        .archive_path
        .strip_prefix("apps/xampp/htdocs/")
        .is_some_and(|path| {
            let parts: Vec<_> = path.split('/').collect();
            parts.len() >= 2
                && parts.iter().enumerate().all(|(index, part)| {
                    safe_component(part) && !excluded_name(part, index + 1 < parts.len())
                })
        });
    if !valid_hash(&entry.sha256)
        || (entry.source_kind == "config" && (!valid_config || entry.bytes > MAX_CONFIG))
        || (entry.source_kind == "project" && (!valid_project || entry.bytes > MAX_FILE))
        || (entry.source_kind != "config" && entry.source_kind != "project")
    {
        return Err("Unsupported XAMPP bundle inventory entry.".into());
    }
    Ok(())
}
fn validate_manifest(manifest: &Manifest) -> Result<&Artifact> {
    if manifest.format_version != 1
        || manifest.kind != "codex-companion-personal-bundle"
        || manifest.platform != "windows"
        || !manifest.sensitive
        || manifest.artifacts.len() != 1
    {
        return Err("Unsupported XAMPP bundle manifest.".into());
    }
    time::OffsetDateTime::parse(
        &manifest.created_at,
        &time::format_description::well_known::Rfc3339,
    )
    .map_err(|_| "Invalid XAMPP bundle timestamp.")?;
    let artifact = &manifest.artifacts[0];
    if artifact.app_id != APP
        || artifact.adapter_format_version != 1
        || artifact.source_app_version.is_empty()
        || !matches!(artifact.architecture.as_str(), "x64" | "x86")
        || artifact.kind != "selected-files"
        || artifact.restore_mode != "manual"
        || artifact.inventory.is_empty()
        || artifact.inventory.len() > MAX_FILES
        || artifact.files as usize != artifact.inventory.len()
        || artifact
            .inventory
            .iter()
            .try_fold(0u64, |sum, entry| sum.checked_add(entry.bytes).ok_or(()))
            .ok()
            != Some(artifact.bytes)
        || artifact.bytes > MAX_TOTAL
    {
        return Err("Unsupported XAMPP bundle artifact.".into());
    }
    let mut names = HashSet::new();
    let mut config_count = 0usize;
    for entry in &artifact.inventory {
        validate_entry(entry)?;
        if !names.insert(entry.archive_path.to_ascii_lowercase()) {
            return Err("XAMPP bundle has duplicate inventory paths.".into());
        }
        if entry.source_kind == "config" {
            config_count += 1;
        }
    }
    if config_count != CONFIGS.len() {
        return Err("XAMPP bundle has incomplete reviewed configuration.".into());
    }
    Ok(artifact)
}
fn open_bundle(path: &Path) -> Result<File> {
    open_regular(path, max_bundle_bytes(), "the XAMPP bundle")
}
fn inspect_archive(archive: &mut ZipArchive<File>) -> Result<(Manifest, u64)> {
    if archive.len() == 0 || archive.len() > MAX_FILES + 1 {
        return Err("XAMPP bundle has an invalid ZIP inventory.".into());
    }
    let mut names = HashSet::new();
    for index in 0..archive.len() {
        let entry = archive
            .by_index(index)
            .map_err(|_| "Invalid XAMPP bundle.")?;
        fs_safety::relative(entry.name()).map_err(|_| "XAMPP bundle has an unsafe ZIP path.")?;
        if entry.is_dir()
            || entry
                .unix_mode()
                .is_some_and(|mode| mode & 0o170000 == 0o120000)
            || !names.insert(entry.name().to_ascii_lowercase())
        {
            return Err("XAMPP bundle has an invalid ZIP inventory.".into());
        }
    }
    let mut raw = Vec::new();
    archive
        .by_name(MANIFEST)
        .map_err(|_| "XAMPP bundle has an incomplete ZIP inventory.")?
        .take(MAX_MANIFEST + 1)
        .read_to_end(&mut raw)
        .map_err(|_| "Cannot read the XAMPP bundle manifest.")?;
    if raw.len() as u64 > MAX_MANIFEST {
        return Err("XAMPP bundle manifest exceeds the size limit.".into());
    }
    let manifest: Manifest =
        serde_json::from_slice(&raw).map_err(|_| "Invalid XAMPP bundle manifest.")?;
    let artifact = validate_manifest(&manifest)?.clone();
    if archive.len() != artifact.inventory.len() + 1 || !names.contains(MANIFEST) {
        return Err("XAMPP bundle has an invalid ZIP inventory.".into());
    }
    for expected in &artifact.inventory {
        if !names.contains(&expected.archive_path.to_ascii_lowercase()) {
            return Err("XAMPP bundle has an incomplete ZIP inventory.".into());
        }
        let entry = archive
            .by_name(&expected.archive_path)
            .map_err(|_| "XAMPP bundle has an incomplete ZIP inventory.")?;
        let limit = if expected.source_kind == "config" {
            MAX_CONFIG
        } else {
            MAX_FILE
        };
        if entry.size() != expected.bytes
            || hash_reader(entry, limit, "XAMPP bundle file")?
                != (expected.sha256.clone(), expected.bytes)
        {
            return Err("XAMPP bundle file verification failed.".into());
        }
        if expected.source_kind == "config" {
            let mut config = Vec::new();
            archive
                .by_name(&expected.archive_path)
                .map_err(|_| "XAMPP bundle has an incomplete ZIP inventory.")?
                .take(MAX_CONFIG + 1)
                .read_to_end(&mut config)
                .map_err(|_| "Cannot read reviewed XAMPP configuration.")?;
            if config.len() as u64 > MAX_CONFIG {
                return Err("Reviewed XAMPP configuration exceeds the size limit.".into());
            }
            reject_sensitive_text(&config)?;
        }
    }
    Ok((manifest, artifact.bytes))
}
fn inspect_at(path: &Path) -> Result<(Manifest, u64)> {
    let mut archive = ZipArchive::new(open_bundle(path)?).map_err(|_| "Invalid XAMPP bundle.")?;
    inspect_archive(&mut archive)
}
fn inspect_hash(path: &Path) -> Result<(Manifest, u64, String)> {
    let mut file = open_bundle(path)?;
    let hash = hash_reader(&mut file, max_bundle_bytes(), "the XAMPP bundle")?.0;
    file.seek(SeekFrom::Start(0))
        .map_err(|_| "Cannot read the XAMPP bundle.")?;
    let mut archive = ZipArchive::new(file).map_err(|_| "Invalid XAMPP bundle.")?;
    let (manifest, bytes) = inspect_archive(&mut archive)?;
    Ok((manifest, bytes, hash))
}
pub fn inspect(path: PathBuf) -> Result<Inspection> {
    let (manifest, bytes, hash) = inspect_hash(&path)?;
    let artifact = validate_manifest(&manifest)?;
    let source_app_version = artifact.source_app_version.clone();
    let architecture = artifact.architecture.clone();
    let file_count = artifact.inventory.len();
    let projects: HashSet<_> = artifact
        .inventory
        .iter()
        .filter_map(|entry| {
            entry
                .archive_path
                .strip_prefix("apps/xampp/htdocs/")
                .and_then(|path| path.split('/').next())
        })
        .map(str::to_owned)
        .collect();
    let token = put(Plan::Inspect {
        bundle: path.clone(),
        hash,
    });
    Ok(Inspection {
        token,
        bundle_name: bundle_name(&path),
        created_at: manifest.created_at,
        source_app_version,
        architecture,
        projects: projects.into_iter().collect(),
        file_count,
        bytes,
        sensitive: true,
    })
}

fn destination_path(root: &Path, archive_path: &str) -> Result<PathBuf> {
    if let Some(project) = archive_path.strip_prefix("apps/xampp/htdocs/") {
        return Ok(root.join("htdocs").join(project.replace('/', "\\")));
    }
    if let Some(config) = archive_path.strip_prefix("apps/xampp/config/") {
        return Ok(root.join(config.replace('/', "\\")));
    }
    Err("Unsupported XAMPP bundle inventory entry.".into())
}
fn destination_state(root: &Path, artifact: &Artifact) -> Result<Vec<(String, bool)>> {
    let mut state = Vec::new();
    for entry in &artifact.inventory {
        let path = destination_path(root, &entry.archive_path)?;
        fs_safety::check(&path)?;
        let exists = match fs::symlink_metadata(path) {
            Ok(metadata) if fs_safety::linked(&metadata) => {
                return Err("XAMPP destination is a symbolic link or junction.".into())
            }
            Ok(_) => true,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
            Err(_) => return Err("Cannot inspect the XAMPP destination.".into()),
        };
        state.push((entry.archive_path.clone(), exists));
    }
    Ok(state)
}
fn staging_path(target: &Path, archive_path: &str) -> Result<PathBuf> {
    if let Some(project) = archive_path.strip_prefix("apps/xampp/htdocs/") {
        return Ok(target.join("htdocs").join(project.replace('/', "\\")));
    }
    if let Some(config) = archive_path.strip_prefix("apps/xampp/config/") {
        return Ok(target.join("configuration").join(config.replace('/', "\\")));
    }
    Err("Unsupported XAMPP bundle inventory entry.".into())
}
pub fn preview_recovery(token: &str) -> Result<RecoveryPreview> {
    require_stopped()?;
    let Plan::Inspect { bundle, hash } = get(token)? else {
        return Err("Inspect an XAMPP bundle first.".into());
    };
    let (manifest, _, actual) = inspect_hash(&bundle)?;
    if actual != hash {
        return Err("XAMPP bundle changed; inspect again.".into());
    }
    let artifact = validate_manifest(&manifest)?;
    let root = xampp_root()?;
    let (version, architecture) = version_architecture(&root)?;
    if version != artifact.source_app_version || architecture != artifact.architecture {
        return Err(
            "The destination XAMPP version or architecture is not compatible with this bundle."
                .into(),
        );
    }
    let state = destination_state(&root, artifact)?;
    let conflicts = state.iter().filter(|(_, exists)| *exists).count();
    let target = platform::personal_staging_dir()
        .join("xampp")
        .join(format!("recovery-{}", NEXT.fetch_add(1, Ordering::Relaxed)));
    fs_safety::check(&target)?;
    if target.exists() {
        return Err("Companion staging destination already exists; inspect again.".into());
    }
    let token = put(Plan::Recover {
        bundle,
        hash,
        target: target.clone(),
        destination_state: state,
    });
    Ok(RecoveryPreview {
        token,
        staging_path: target.to_string_lossy().into_owned(),
        destination_conflicts: conflicts,
        manual_only: true,
    })
}
fn restore_at(
    bundle: &Path,
    hash: &str,
    target: &Path,
    expected_state: &[(String, bool)],
    fail_after_write: bool,
) -> Result<()> {
    let mut file = open_bundle(bundle)?;
    if hash_reader(&mut file, max_bundle_bytes(), "the XAMPP bundle")?.0 != hash {
        return Err("XAMPP bundle changed; inspect again.".into());
    }
    file.seek(SeekFrom::Start(0))
        .map_err(|_| "Cannot read the XAMPP bundle.")?;
    let mut archive = ZipArchive::new(file).map_err(|_| "Invalid XAMPP bundle.")?;
    let (manifest, _) = inspect_archive(&mut archive)?;
    let artifact = validate_manifest(&manifest)?;
    let root = xampp_root()?;
    let (version, architecture) = version_architecture(&root)?;
    if version != artifact.source_app_version
        || architecture != artifact.architecture
        || destination_state(&root, artifact)? != expected_state
        || target.exists()
    {
        return Err("XAMPP destination changed; preview again.".into());
    }
    fs_safety::check(target)?;
    let mut created = Vec::new();
    let attempt = (|| -> Result<()> {
        for entry in &artifact.inventory {
            let path = staging_path(target, &entry.archive_path)?;
            let parent = path
                .parent()
                .ok_or("Invalid Companion staging destination.")?;
            fs_safety::check(parent)?;
            fs::create_dir_all(parent).map_err(|_| "Cannot create Companion staging.")?;
            fs_safety::check(&path)?;
            let mut output = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)
                .map_err(|_| "Companion staging destination changed; preview again.")?;
            created.push(path.clone());
            let limit = entry
                .bytes
                .checked_add(1)
                .ok_or("XAMPP bundle file exceeds the size limit.")?;
            let source = archive
                .by_name(&entry.archive_path)
                .map_err(|_| "XAMPP bundle has an incomplete ZIP inventory.")?;
            copy_checked(
                source.take(limit),
                &mut output,
                entry.bytes,
                &entry.sha256,
                "XAMPP bundle file",
            )?;
            output
                .sync_all()
                .map_err(|_| "Cannot recover XAMPP files to Companion staging.")?;
        }
        if fail_after_write {
            return Err("Injected recovery failure.".into());
        }
        Ok(())
    })();
    if let Err(error) = attempt {
        let mut rollback_failed = false;
        for path in created.into_iter().rev() {
            if fs::remove_file(path).is_err() {
                rollback_failed = true;
            }
        }
        if rollback_failed {
            return Err(
                "XAMPP recovery failed and Companion staging could not be rolled back.".into(),
            );
        }
        return Err(error);
    }
    Ok(())
}
pub fn recover(token: &str, confirmation: &str) -> Result<RecoveryResult> {
    if confirmation != "RECOVER" {
        return Err("Type RECOVER to confirm.".into());
    }
    require_stopped()?;
    let Plan::Recover {
        bundle,
        hash,
        target,
        destination_state,
    } = get(token)?
    else {
        return Err("Preview XAMPP recovery first.".into());
    };
    restore_at(&bundle, &hash, &target, &destination_state, false)?;
    Ok(RecoveryResult {
        recovered: true,
        staging_path: target.to_string_lossy().into_owned(),
        manual_only: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn root() -> PathBuf {
        std::env::temp_dir().join(format!(
            "xampp-test-{}",
            time::OffsetDateTime::now_utc().unix_timestamp_nanos()
        ))
    }
    fn write(path: &Path, text: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }
    fn fixture(root: &Path) {
        write(
            &root.join("readme_en.txt"),
            "###### ApacheFriends XAMPP Version 8.2.12 ######\n+ PHP 8.2.12 64bit\n",
        );
        for config in CONFIGS {
            write(&root.join(config.replace('/', "\\")), "Listen 80\n");
        }
        write(&root.join("htdocs/demo/index.php"), "<?php echo 'ok';");
    }
    fn snapshot(root: &Path) -> Identity {
        snapshot_at(root, &[root.join("htdocs/demo")]).unwrap()
    }
    fn write_hash_valid_bundle(path: &Path, identity: &Identity, sensitive_config: bool) {
        let mut inventory: Vec<_> = identity
            .files
            .iter()
            .map(|file| file.entry.clone())
            .collect();
        if sensitive_config {
            let entry = inventory
                .iter_mut()
                .find(|entry| entry.archive_path.ends_with("php/php.ini"))
                .unwrap();
            entry.bytes = b"password=synthetic\n".len() as u64;
            entry.sha256 = format!("{:x}", Sha256::digest(b"password=synthetic\n"));
        }
        let bytes = inventory.iter().map(|entry| entry.bytes).sum();
        let manifest = Manifest {
            format_version: 1,
            kind: "codex-companion-personal-bundle".into(),
            created_at: "2026-09-07T00:00:00Z".into(),
            platform: "windows".into(),
            sensitive: true,
            artifacts: vec![Artifact {
                app_id: APP.into(),
                adapter_format_version: 1,
                source_app_version: identity.version.clone(),
                architecture: identity.architecture.clone(),
                kind: "selected-files".into(),
                files: inventory.len() as u32,
                bytes,
                restore_mode: "manual".into(),
                inventory: inventory.clone(),
            }],
        };
        let file = File::create(path).unwrap();
        let mut zip = ZipWriter::new(file);
        let options = SimpleFileOptions::default();
        zip.start_file(MANIFEST, options).unwrap();
        zip.write_all(&serde_json::to_vec(&manifest).unwrap())
            .unwrap();
        for source in &identity.files {
            zip.start_file(&source.entry.archive_path, options).unwrap();
            if sensitive_config && source.entry.archive_path.ends_with("php/php.ini") {
                zip.write_all(b"password=synthetic\n").unwrap();
            } else {
                zip.write_all(&fs::read(&source.source).unwrap()).unwrap();
            }
        }
        zip.finish().unwrap();
    }
    #[test]
    fn accepts_selected_projects_and_reviewed_config_only() {
        let root = root();
        fixture(&root);
        write(&root.join("htdocs/demo/.env"), "SECRET=synthetic");
        write(&root.join("htdocs/demo/.env.local"), "SECRET=synthetic");
        write(&root.join("htdocs/demo/server.key"), "synthetic-key");
        write(&root.join("htdocs/demo/id_ed25519"), "synthetic-key");
        let identity = snapshot(&root);
        assert_eq!(identity.files.len(), 5);
        assert_eq!(identity.excluded_count, 4);
    }
    #[test]
    fn rejects_outside_projects_secrets_and_links() {
        let root = root();
        fixture(&root);
        assert!(snapshot_at(&root, &[root.join("htdocs")]).is_err());
        write(&root.join("php/php.ini"), "password=synthetic");
        assert!(snapshot_at(&root, &[root.join("htdocs/demo")]).is_err());
    }
    #[test]
    fn bundle_validates_hash_inventory_source_change_and_rollback() {
        let root = root();
        fixture(&root);
        let identity = snapshot(&root);
        let out = root.join("out");
        create_at(&identity, &out).unwrap();
        let bundle = out.read_dir().unwrap().next().unwrap().unwrap().path();
        assert!(inspect_at(&bundle).is_ok());
        write(&root.join("htdocs/demo/index.php"), "changed");
        assert_ne!(snapshot(&root), identity);
        assert!(create_revalidated_at(&identity, &root.join("changed-out")).is_err());
        let hash = hash_file(&bundle, max_bundle_bytes(), "bundle").unwrap().0;
        let stage = root.join("stage");
        let state = destination_state(
            &root,
            validate_manifest(&inspect_at(&bundle).unwrap().0).unwrap(),
        )
        .unwrap();
        assert!(restore_at(&bundle, &hash, &stage, &state, true).is_err());
        assert!(!stage.join("htdocs/demo/index.php").exists());
    }
    #[test]
    fn rejects_hash_valid_archives_with_sensitive_config_or_excluded_project_names() {
        let root = root();
        fixture(&root);
        let identity = snapshot(&root);
        let sensitive = root.join("sensitive.zip");
        write_hash_valid_bundle(&sensitive, &identity, true);
        assert!(inspect_at(&sensitive).is_err());
        let mut entry = identity
            .files
            .iter()
            .find(|file| file.entry.source_kind == "project")
            .unwrap()
            .entry
            .clone();
        entry.archive_path = "apps/xampp/htdocs/demo/.env.local".into();
        assert!(validate_entry(&entry).is_err());
    }
    #[test]
    fn identifies_xampp_processes() {
        assert!(has_process("\"httpd.exe\",\"1\""));
        assert!(has_process("\"mysqld.exe\",\"1\""));
        assert!(!has_process("\"Code.exe\",\"1\""));
    }
}
