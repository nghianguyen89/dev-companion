//! Explicit SourceTree configuration bundle. Windows Vault and SSH credentials stay out of scope.
use std::{
    cmp::Reverse,
    collections::{HashMap, HashSet},
    fs::{self, File, OpenOptions},
    io::{Read, Seek, Write},
    path::{Path, PathBuf},
    process::Command,
    sync::{
        atomic::{AtomicU64, Ordering},
        Mutex, OnceLock,
    },
    time::UNIX_EPOCH,
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use zip::{write::SimpleFileOptions, AesMode, CompressionMethod, ZipArchive, ZipWriter};

use crate::{fs_safety, platform, sourcetree};

type Result<T> = std::result::Result<T, String>;
const MANIFEST: &str = "sourcetree-config-manifest.json";
const ROOT: &str = "apps/sourcetree/";
const LIMIT: u64 = 32 * 1024 * 1024;
const TOTAL_LIMIT: u64 = 64 * 1024 * 1024;
const CONFIG_FILES: [(&str, &str); 7] = [
    ("accounts.json", "accounts.json"),
    ("bookmarks.xml", "bookmarks.xml"),
    ("customactions.xml", "customactions.xml"),
    ("hostedaccounts.xml", "hostedaccounts.xml"),
    ("opentabs.xml", "opentabs.xml"),
    ("passwd", "passwd"),
    ("userhosts", "userhosts"),
];

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileInfo {
    pub name: String,
    pub bytes: u64,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Preview {
    pub token: String,
    pub files: Vec<FileInfo>,
    pub missing: Vec<String>,
    pub bytes: u64,
    pub sensitive: bool,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Created {
    pub bundle_name: String,
    pub bundle_path: String,
    pub bytes: u64,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Inspection {
    pub token: String,
    pub bundle_name: String,
    pub files: Vec<FileInfo>,
    pub bytes: u64,
    pub sensitive: bool,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryPreview {
    pub token: String,
    pub files: Vec<FileInfo>,
    pub existing_targets: u32,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryResult {
    pub restored: u32,
    pub safety_copy_path: String,
    pub files: Vec<RestoredFile>,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoredFile {
    pub name: String,
    pub destination: String,
    pub bytes: u64,
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
    pub archives: Vec<Archive>,
    pub total_bytes: u64,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Bundle {
    format_version: u32,
    kind: String,
    created_at: String,
    sensitive: bool,
    #[serde(default)]
    encryption: Option<String>,
    #[serde(default)]
    profile: Option<ConfigProfile>,
    files: Vec<Entry>,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum ConfigProfile {
    Local,
    Roaming,
}
#[derive(Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Entry {
    name: String,
    archive_path: String,
    destination: String,
    bytes: u64,
    sha256: String,
}
#[derive(Clone, PartialEq)]
struct Source {
    entry: Entry,
    path: PathBuf,
}
#[derive(Clone)]
enum Plan {
    Create(Vec<Source>, ConfigProfile),
    Inspect {
        path: PathBuf,
        hash: String,
        bundle: Bundle,
    },
    Recover {
        path: PathBuf,
        hash: String,
    },
}
static PLANS: OnceLock<Mutex<HashMap<String, Plan>>> = OnceLock::new();
static NEXT: AtomicU64 = AtomicU64::new(1);

fn put(plan: Plan) -> String {
    let token = format!("sourcetree-config-{}", NEXT.fetch_add(1, Ordering::Relaxed));
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
fn hash_file(path: &Path) -> Result<(String, u64)> {
    fs_safety::check(path)?;
    let metadata = fs::symlink_metadata(path)
        .map_err(|_| "Cannot inspect SourceTree configuration.".to_string())?;
    if fs_safety::linked(&metadata) || !metadata.is_file() || metadata.len() > LIMIT {
        return Err("SourceTree configuration contains an unsupported file.".into());
    }
    let mut file = File::open(path).map_err(|_| {
        "Cannot read SourceTree configuration. Close SourceTree and retry.".to_string()
    })?;
    let mut hash = Sha256::new();
    let mut bytes = 0;
    let mut buffer = [0; 65536];
    loop {
        let count = file
            .read(&mut buffer)
            .map_err(|_| "Cannot read SourceTree configuration.".to_string())?;
        if count == 0 {
            break;
        }
        bytes += count as u64;
        if bytes > LIMIT {
            return Err("SourceTree configuration file exceeds the size limit.".into());
        }
        hash.update(&buffer[..count]);
    }
    Ok((format!("{:x}", hash.finalize()), bytes))
}
fn local_atlassian() -> Result<PathBuf> {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .map(|path| path.join("Atlassian"))
        .ok_or_else(|| "Cannot locate Windows LocalAppData.".into())
}
fn roaming_atlassian() -> Result<PathBuf> {
    std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .map(|path| path.join("Atlassian"))
        .ok_or_else(|| "Cannot locate Windows AppData.".into())
}
fn user_config_at(root: &Path) -> Option<(PathBuf, String)> {
    let mut candidates = Vec::new();
    let Ok(apps) = fs::read_dir(root) else {
        return None;
    };
    for app in apps.flatten() {
        let name = app.file_name().to_string_lossy().into_owned();
        if !name.starts_with("SourceTree.exe_Url_") || fs_safety::check(&app.path()).is_err() {
            continue;
        }
        let Ok(versions) = fs::read_dir(app.path()) else {
            continue;
        };
        for version in versions.flatten() {
            let path = version.path().join("user.config");
            if let Ok(metadata) = fs::symlink_metadata(&path) {
                if metadata.is_file() && !fs_safety::linked(&metadata) {
                    candidates.push((
                        metadata.modified().ok(),
                        path,
                        format!(
                            "{}/{}/user.config",
                            name,
                            version.file_name().to_string_lossy()
                        ),
                    ));
                }
            }
        }
    }
    candidates
        .into_iter()
        .max_by_key(|item| item.0)
        .map(|(_, path, relative)| (path, relative))
}
fn current_user_config_at(local: &Path, roaming: &Path) -> Option<(PathBuf, String)> {
    [local, roaming]
        .into_iter()
        .filter_map(user_config_at)
        .max_by_key(|(path, _)| {
            fs::metadata(path)
                .ok()
                .and_then(|metadata| metadata.modified().ok())
        })
}
fn source_files() -> Result<(Vec<Source>, Vec<String>, ConfigProfile)> {
    let local = local_atlassian()?;
    let roaming = roaming_atlassian()?;
    let local_root = local.join("SourceTree");
    let roaming_root = roaming.join("SourceTree");
    let profile = if local_root.join("bookmarks.xml").is_file()
        || !roaming_root.join("bookmarks.xml").is_file()
    {
        ConfigProfile::Local
    } else {
        ConfigProfile::Roaming
    };
    let mut found = Vec::new();
    let mut missing = Vec::new();
    for (name, relative) in CONFIG_FILES {
        let local_path = local_root.join(relative);
        let roaming_path = roaming_root.join(relative);
        let path = if local_path.is_file() {
            local_path
        } else {
            roaming_path
        };
        if path.exists() {
            let (sha256, bytes) = hash_file(&path)?;
            found.push(Source {
                entry: Entry {
                    name: name.into(),
                    archive_path: format!("{ROOT}{relative}"),
                    destination: relative.into(),
                    bytes,
                    sha256,
                },
                path,
            });
        } else {
            missing.push(name.into());
        }
    }
    if let Some((path, relative)) = current_user_config_at(&local, &roaming) {
        let (sha256, bytes) = hash_file(&path)?;
        found.push(Source {
            entry: Entry {
                name: "user.config".into(),
                archive_path: format!("{ROOT}user.config"),
                destination: relative,
                bytes,
                sha256,
            },
            path,
        });
    } else {
        missing.push("user.config".into());
    }
    if found.is_empty() {
        return Err("No supported SourceTree configuration files were found.".into());
    }
    Ok((found, missing, profile))
}
fn info(files: &[Entry]) -> Vec<FileInfo> {
    files
        .iter()
        .map(|file| FileInfo {
            name: file.name.clone(),
            bytes: file.bytes,
        })
        .collect()
}
fn valid_hash(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
fn valid_destination(value: &str, name: &str) -> bool {
    if name != "user.config" {
        return CONFIG_FILES.iter().any(|(allowed, _)| name == *allowed) && value == name;
    }
    fs_safety::relative(value).is_ok()
        && value.starts_with("SourceTree.exe_Url_")
        && value.ends_with("/user.config")
        && value.split('/').count() == 3
}
fn validate(bundle: &Bundle) -> Result<()> {
    let encrypted =
        matches!(bundle.format_version, 2 | 3) && bundle.encryption.as_deref() == Some("aes-256");
    if !matches!(bundle.format_version, 1 | 2 | 3)
        || bundle.kind != "dev-companion-sourcetree-config"
        || !bundle.sensitive
        || (bundle.format_version == 1 && bundle.encryption.is_some())
        || (matches!(bundle.format_version, 2 | 3) && !encrypted)
        || (bundle.format_version == 3 && bundle.profile.is_none())
        || (bundle.format_version < 3 && bundle.profile.is_some())
        || bundle.files.is_empty()
        || bundle.files.len() > CONFIG_FILES.len() + 1
    {
        return Err("Unsupported SourceTree configuration bundle.".into());
    }
    let mut names = HashSet::new();
    let mut total = 0;
    for file in &bundle.files {
        let allowed =
            file.name == "user.config" || CONFIG_FILES.iter().any(|(name, _)| file.name == *name);
        if !allowed
            || (bundle.format_version == 1
                && !matches!(
                    file.name.as_str(),
                    "accounts.json" | "bookmarks.xml" | "customactions.xml" | "user.config"
                ))
            || file.archive_path != format!("{ROOT}{}", file.name)
            || !valid_destination(&file.destination, &file.name)
            || !valid_hash(&file.sha256)
            || !names.insert(file.name.clone())
            || file.bytes > LIMIT
        {
            return Err("SourceTree configuration bundle has an invalid manifest.".into());
        }
        total += file.bytes;
    }
    if total > TOTAL_LIMIT {
        return Err("SourceTree configuration bundle exceeds the size limit.".into());
    }
    Ok(())
}
fn validate_password(password: &str) -> Result<()> {
    if password.chars().count() < 6 {
        return Err("Use a bundle password of at least 6 characters.".into());
    }
    Ok(())
}
fn bundle_hash(path: &Path) -> Result<String> {
    let (hash, _) = hash_file(path)?;
    Ok(hash)
}
fn aes256_entry<R: Read + Seek>(archive: &mut ZipArchive<R>, name: &str) -> Result<bool> {
    let Some(index) = archive.index_for_name(name) else {
        return Ok(false);
    };
    Ok(archive
        .get_aes_verification_key_and_salt(index)
        .map_err(|_| "Cannot inspect SourceTree bundle encryption.")?
        .is_some_and(|info| info.aes_mode == AesMode::Aes256))
}

pub fn preview() -> Result<Preview> {
    sourcetree::require_closed()?;
    let (sources, missing, profile) = source_files()?;
    let files = sources
        .iter()
        .map(|source| source.entry.clone())
        .collect::<Vec<_>>();
    let bytes = files.iter().map(|file| file.bytes).sum();
    Ok(Preview {
        token: put(Plan::Create(sources, profile)),
        files: info(&files),
        missing,
        bytes,
        sensitive: true,
    })
}
pub fn create(token: &str, password: &str) -> Result<Created> {
    sourcetree::require_closed()?;
    validate_password(password)?;
    let Plan::Create(sources, profile) = get(token)? else {
        return Err("Preview SourceTree configuration first.".into());
    };
    let (current, _, current_profile) = source_files()?;
    if current != sources || current_profile != profile {
        return Err("SourceTree configuration changed; preview again.".into());
    }
    let output = platform::personal_bundle_dir();
    fs_safety::check(&output)?;
    fs::create_dir_all(&output).map_err(|_| "Cannot create the Companion bundle folder.")?;
    let path = output.join(format!(
        "sourcetree-config-{}.zip",
        time::OffsetDateTime::now_utc().unix_timestamp_nanos()
    ));
    let result = (|| -> Result<()> {
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|_| "Cannot create the SourceTree configuration bundle.")?;
        let manifest = Bundle {
            format_version: 3,
            kind: "dev-companion-sourcetree-config".into(),
            created_at: timestamp()?,
            sensitive: true,
            encryption: Some("aes-256".into()),
            profile: Some(profile),
            files: sources.iter().map(|source| source.entry.clone()).collect(),
        };
        let mut zip = ZipWriter::new(file);
        let options = SimpleFileOptions::default()
            .compression_method(CompressionMethod::Deflated)
            .with_aes_encryption(AesMode::Aes256, password);
        zip.start_file(MANIFEST, options)
            .map_err(|_| "Cannot create the SourceTree configuration bundle.")?;
        zip.write_all(
            &serde_json::to_vec(&manifest)
                .map_err(|_| "Cannot create the SourceTree configuration bundle.")?,
        )
        .map_err(|_| "Cannot create the SourceTree configuration bundle.")?;
        for source in &sources {
            let (hash, bytes) = hash_file(&source.path)?;
            if hash != source.entry.sha256 || bytes != source.entry.bytes {
                return Err("SourceTree configuration changed; preview again.".into());
            }
            zip.start_file(&source.entry.archive_path, options)
                .map_err(|_| "Cannot create the SourceTree configuration bundle.")?;
            let mut input =
                File::open(&source.path).map_err(|_| "Cannot read SourceTree configuration.")?;
            std::io::copy(&mut input, &mut zip)
                .map_err(|_| "Cannot create the SourceTree configuration bundle.")?;
        }
        zip.finish()
            .map_err(|_| "Cannot finalize the SourceTree configuration bundle.")?
            .sync_all()
            .map_err(|_| "Cannot finalize the SourceTree configuration bundle.")?;
        let _ = inspect(path.clone(), password)?;
        Ok(())
    })();
    if let Err(error) = result {
        let _ = fs::remove_file(&path);
        return Err(error);
    }
    Ok(Created {
        bundle_name: path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned(),
        bundle_path: path.display().to_string(),
        bytes: fs::metadata(&path)
            .map_err(|_| "Cannot verify the SourceTree configuration bundle.")?
            .len(),
    })
}

pub fn open_bundle_folder() -> Result<()> {
    let folder = platform::personal_bundle_dir();
    fs_safety::check(&folder)?;
    if !folder.is_dir() {
        return Err("The SourceTree bundle folder does not exist.".into());
    }
    Command::new("explorer.exe")
        .arg(folder)
        .spawn()
        .map(|_| ())
        .map_err(|_| "Cannot open the SourceTree bundle folder.".into())
}
fn archive_name(name: &str) -> bool {
    name.strip_prefix("sourcetree-config-")
        .and_then(|value| value.strip_suffix(".zip"))
        .is_some_and(|stamp| !stamp.is_empty() && stamp.bytes().all(|byte| byte.is_ascii_digit()))
}
fn archive_path_at(directory: &Path, name: &str) -> Result<PathBuf> {
    if !archive_name(name) {
        return Err("Unknown SourceTree configuration bundle.".into());
    }
    fs_safety::check(directory)?;
    let path = directory.join(name);
    fs_safety::check(&path)?;
    let metadata = fs::symlink_metadata(&path)
        .map_err(|_| "SourceTree configuration bundle is unavailable.")?;
    if fs_safety::linked(&metadata) || !metadata.is_file() {
        return Err("SourceTree configuration bundle is not a regular file.".into());
    }
    Ok(path)
}
fn list_archives_at(directory: &Path) -> Result<Archives> {
    fs_safety::check(directory)?;
    if !directory.exists() {
        return Ok(Archives {
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
        archives,
        total_bytes,
    })
}

fn migrate_legacy_archives_at(legacy: &Path, destination: &Path) -> Result<()> {
    fs_safety::check(legacy)?;
    fs_safety::check(destination)?;
    if !legacy.is_dir() {
        return Ok(());
    }
    fs::create_dir_all(destination).map_err(|_| "Cannot create the Companion bundle folder.")?;
    for entry in fs::read_dir(legacy).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if !archive_name(&name) {
            continue;
        }
        let source = entry.path();
        let metadata = fs::symlink_metadata(&source).map_err(|error| error.to_string())?;
        if fs_safety::linked(&metadata) || !metadata.is_file() {
            continue;
        }
        let target = destination.join(&name);
        fs_safety::check(&target)?;
        if target.exists() {
            continue;
        }
        fs::rename(source, target)
            .map_err(|_| "Cannot move the legacy SourceTree configuration bundle.")?;
    }
    Ok(())
}
pub fn list_archives() -> Result<Archives> {
    let directory = platform::personal_bundle_dir();
    if let Some(legacy) = platform::legacy_personal_bundle_dir() {
        migrate_legacy_archives_at(&legacy, &directory)?;
    }
    list_archives_at(&directory)
}
pub fn open_archive(name: &str) -> Result<()> {
    let path = archive_path_at(&platform::personal_bundle_dir(), name)?;
    Command::new("explorer.exe")
        .arg(format!("/select,{}", path.display()))
        .spawn()
        .map(|_| ())
        .map_err(|_| "Cannot open the SourceTree configuration bundle folder.".into())
}
fn delete_archive_at(directory: &Path, name: &str) -> Result<()> {
    let path = archive_path_at(directory, name)?;
    fs::remove_file(path).map_err(|_| "Cannot remove SourceTree configuration bundle.")?;
    Ok(())
}
pub fn delete_archive(name: &str) -> Result<()> {
    delete_archive_at(&platform::personal_bundle_dir(), name)
}
pub fn inspect_archive(name: &str, password: &str) -> Result<Inspection> {
    inspect(
        archive_path_at(&platform::personal_bundle_dir(), name)?,
        password,
    )
}
fn read_bundle(path: &Path, password: &str) -> Result<Bundle> {
    fs_safety::check(path)?;
    let hash = bundle_hash(path)?;
    if hash.is_empty() {
        return Err("Cannot read the SourceTree configuration bundle.".into());
    }
    let file = File::open(path).map_err(|_| "Cannot read the SourceTree configuration bundle.")?;
    let mut archive =
        ZipArchive::new(file).map_err(|_| "Invalid SourceTree configuration bundle.")?;
    if archive.len() < 2 || archive.len() > CONFIG_FILES.len() + 2 {
        return Err("SourceTree configuration bundle has an invalid ZIP inventory.".into());
    }
    let mut manifest = Vec::new();
    let manifest_encrypted = aes256_entry(&mut archive, MANIFEST)?;
    archive
        .by_name_decrypt(MANIFEST, password.as_bytes())
        .map_err(|_| "Cannot decrypt the SourceTree configuration bundle. Check its password.")?
        .take(1024 * 1024)
        .read_to_end(&mut manifest)
        .map_err(|_| "Cannot read SourceTree configuration bundle.")?;
    let bundle: Bundle = serde_json::from_slice(&manifest)
        .map_err(|_| "Invalid SourceTree configuration bundle manifest.")?;
    validate(&bundle)?;
    if matches!(bundle.format_version, 2 | 3) && !manifest_encrypted {
        return Err("SourceTree configuration bundle is not AES-256 encrypted.".into());
    }
    let mut expected = HashSet::from([MANIFEST.to_owned()]);
    for entry in &bundle.files {
        expected.insert(entry.archive_path.clone());
        let encrypted = aes256_entry(&mut archive, &entry.archive_path)?;
        let mut file = archive
            .by_name_decrypt(&entry.archive_path, password.as_bytes())
            .map_err(|_| {
                "Cannot decrypt the SourceTree configuration bundle. Check its password."
            })?;
        if file.is_dir()
            || file.size() != entry.bytes
            || (matches!(bundle.format_version, 2 | 3) && !encrypted)
        {
            return Err("SourceTree configuration bundle has an invalid entry.".into());
        }
        let mut hash = Sha256::new();
        let mut bytes = 0;
        let mut buffer = [0; 65536];
        loop {
            let count = file
                .read(&mut buffer)
                .map_err(|_| "Cannot read SourceTree configuration bundle.")?;
            if count == 0 {
                break;
            }
            bytes += count as u64;
            if bytes > LIMIT {
                return Err("SourceTree configuration bundle exceeds the size limit.".into());
            }
            hash.update(&buffer[..count]);
        }
        if bytes != entry.bytes || format!("{:x}", hash.finalize()) != entry.sha256 {
            return Err("SourceTree configuration bundle verification failed.".into());
        }
    }
    if archive.file_names().any(|name| !expected.contains(name)) {
        return Err("SourceTree configuration bundle has unexpected files.".into());
    }
    Ok(bundle)
}
pub fn inspect(path: PathBuf, password: &str) -> Result<Inspection> {
    let bundle = read_bundle(&path, password)?;
    let hash = bundle_hash(&path)?;
    let files = bundle.files.clone();
    Ok(Inspection {
        token: put(Plan::Inspect {
            path: path.clone(),
            hash,
            bundle,
        }),
        bundle_name: path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned(),
        files: info(&files),
        bytes: files.iter().map(|file| file.bytes).sum(),
        sensitive: true,
    })
}
fn targets_at(root: &Path, bundle: &Bundle) -> Result<Vec<(Entry, PathBuf)>> {
    bundle
        .files
        .iter()
        .cloned()
        .map(|entry| {
            let target = if entry.name == "user.config" {
                root.join(entry.destination.replace('/', "\\"))
            } else {
                root.join("SourceTree").join(&entry.destination)
            };
            fs_safety::check(&target)?;
            Ok((entry, target))
        })
        .collect()
}
fn targets_for_at(local: &Path, roaming: &Path, bundle: &Bundle) -> Result<Vec<(Entry, PathBuf)>> {
    let mut targets = Vec::new();
    for entry in &bundle.files {
        if entry.name == "user.config" {
            if let Some((path, _)) = current_user_config_at(local, roaming) {
                fs_safety::check(&path)?;
                targets.push((entry.clone(), path));
            } else {
                let root = match bundle.profile.unwrap_or(ConfigProfile::Local) {
                    ConfigProfile::Local => local,
                    ConfigProfile::Roaming => roaming,
                };
                targets.extend(targets_at(
                    root,
                    &Bundle {
                        files: vec![entry.clone()],
                        ..bundle.clone()
                    },
                )?);
            }
        } else {
            targets.push((
                entry.clone(),
                local.join("SourceTree").join(&entry.destination),
            ));
            targets.push((
                entry.clone(),
                roaming.join("SourceTree").join(&entry.destination),
            ));
        }
    }
    for (_, target) in &targets {
        fs_safety::check(target)?;
    }
    Ok(targets)
}
fn targets(bundle: &Bundle) -> Result<Vec<(Entry, PathBuf)>> {
    targets_for_at(&local_atlassian()?, &roaming_atlassian()?, bundle)
}
pub fn preview_recovery(token: &str, password: &str) -> Result<RecoveryPreview> {
    sourcetree::require_closed()?;
    let Plan::Inspect { path, hash, bundle } = get(token)? else {
        return Err("Inspect a SourceTree configuration bundle first.".into());
    };
    if bundle_hash(&path)? != hash {
        return Err("SourceTree configuration bundle changed; inspect again.".into());
    }
    let _ = read_bundle(&path, password)?;
    let existing_targets = targets(&bundle)?
        .iter()
        .filter(|(_, path)| path.exists())
        .count() as u32;
    let files = bundle.files.clone();
    Ok(RecoveryPreview {
        token: put(Plan::Recover { path, hash }),
        files: info(&files),
        existing_targets,
    })
}
pub fn recover(token: &str, confirmation: &str, password: &str) -> Result<RecoveryResult> {
    if confirmation != "RESTORE" {
        return Err("Type RESTORE to apply the SourceTree configuration bundle.".into());
    }
    sourcetree::require_closed()?;
    let Plan::Recover { path, hash, .. } = get(token)? else {
        return Err("Preview SourceTree configuration recovery first.".into());
    };
    if bundle_hash(&path)? != hash {
        return Err("SourceTree configuration bundle changed; inspect again.".into());
    }
    let bundle = read_bundle(&path, password)?;
    let targets = targets(&bundle)?;
    let safety = platform::personal_staging_dir().join(format!(
        "sourcetree-safety-{}",
        time::OffsetDateTime::now_utc().unix_timestamp_nanos()
    ));
    fs::create_dir_all(&safety).map_err(|_| "Cannot create SourceTree safety copy.")?;
    let file = File::open(&path).map_err(|_| "Cannot read the SourceTree configuration bundle.")?;
    let mut archive =
        ZipArchive::new(file).map_err(|_| "Invalid SourceTree configuration bundle.")?;
    let restored_files = restore_targets(targets, &safety, |entry, target| {
        let mut input = archive
            .by_name_decrypt(&entry.archive_path, password.as_bytes())
            .map_err(|_| {
                "Cannot decrypt the SourceTree configuration bundle. Check its password."
            })?;
        replace_target(entry, target, &mut input)
    })?;
    Ok(RecoveryResult {
        restored: restored_files.len() as u32,
        safety_copy_path: safety.display().to_string(),
        files: restored_files,
    })
}
fn replace_target<R: Read>(entry: &Entry, target: &Path, input: &mut R) -> Result<()> {
    let temporary = target.with_extension("dev-companion.tmp");
    let mut owns_temporary = false;
    let result = (|| -> Result<()> {
        let mut output = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|_| "Cannot prepare SourceTree restore.")?;
        owns_temporary = true;
        std::io::copy(input, &mut output)
            .map_err(|_| "Cannot restore SourceTree configuration.")?;
        output
            .sync_all()
            .map_err(|_| "Cannot restore SourceTree configuration.")?;
        if target.exists() {
            fs::remove_file(target).map_err(|_| "Cannot replace SourceTree configuration.")?;
        }
        fs::rename(&temporary, target).map_err(|_| "Cannot finalize SourceTree restore.")?;
        if hash_file(target)? != (entry.sha256.clone(), entry.bytes) {
            return Err("SourceTree restore verification failed.".into());
        }
        Ok(())
    })();
    if result.is_err() && owns_temporary {
        let _ = fs::remove_file(temporary);
    }
    result
}
fn restore_targets<F>(
    targets: Vec<(Entry, PathBuf)>,
    safety: &Path,
    mut write: F,
) -> Result<Vec<RestoredFile>>
where
    F: FnMut(&Entry, &Path) -> Result<()>,
{
    let mut written: Vec<(PathBuf, Option<PathBuf>)> = Vec::new();
    let mut restored_files = Vec::new();
    let result = (|| -> Result<()> {
        for (index, (entry, target)) in targets.into_iter().enumerate() {
            let backup = if target.exists() {
                let path = safety.join(index.to_string()).join(&entry.name);
                fs::create_dir_all(path.parent().ok_or("Invalid SourceTree safety path.")?)
                    .map_err(|_| "Cannot create SourceTree safety copy.")?;
                fs::copy(&target, &path).map_err(|_| "Cannot save SourceTree safety copy.")?;
                Some(path)
            } else {
                None
            };
            let parent = target.parent().ok_or("Invalid SourceTree destination.")?;
            fs::create_dir_all(parent).map_err(|_| "Cannot create SourceTree destination.")?;
            written.push((target.clone(), backup.clone()));
            write(&entry, &target)?;
            restored_files.push(RestoredFile {
                name: entry.name,
                destination: target.display().to_string(),
                bytes: entry.bytes,
            });
        }
        Ok(())
    })();
    if let Err(error) = result {
        let mut rollback_failures = 0;
        for (target, backup) in written.into_iter().rev() {
            let mut failed = target.exists() && fs::remove_file(&target).is_err();
            if let Some(backup) = backup {
                failed |= fs::copy(backup, target).is_err();
            }
            if failed {
                rollback_failures += 1;
            }
        }
        if rollback_failures > 0 {
            return Err(format!(
                "{error} Rollback incomplete for {rollback_failures} target(s); safety copies remain at {}.",
                safety.display()
            ));
        }
        return Err(error);
    }
    Ok(restored_files)
}
pub fn delete(token: &str, confirmation: &str) -> Result<()> {
    if confirmation != "DELETE" {
        return Err("Type DELETE to remove the SourceTree configuration bundle.".into());
    }
    let Plan::Inspect { path, hash, .. } = get(token)? else {
        return Err("Inspect a SourceTree configuration bundle first.".into());
    };
    if bundle_hash(&path)? != hash {
        return Err("SourceTree configuration bundle changed; inspect again.".into());
    }
    let metadata = fs::symlink_metadata(&path)
        .map_err(|_| "Cannot inspect SourceTree configuration bundle.")?;
    if fs_safety::linked(&metadata) || !metadata.is_file() {
        return Err("SourceTree configuration bundle is not a regular file.".into());
    }
    fs::remove_file(path).map_err(|_| "Cannot remove SourceTree configuration bundle.")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_unexpected_destinations() {
        assert!(!valid_destination("../user.config", "user.config"));
        assert!(valid_destination(
            "SourceTree.exe_Url_hash/3.4.0.0/user.config",
            "user.config"
        ));
    }

    #[test]
    fn aes_bundle_entries_require_the_bundle_password() {
        let password = "twelve-char-password";
        let mut cursor = std::io::Cursor::new(Vec::new());
        let mut writer = ZipWriter::new(&mut cursor);
        let options = SimpleFileOptions::default()
            .compression_method(CompressionMethod::Deflated)
            .with_aes_encryption(AesMode::Aes256, password);
        writer.start_file("fixture", options).unwrap();
        writer.write_all(b"sensitive fixture").unwrap();
        writer.finish().unwrap();
        let mut archive = ZipArchive::new(std::io::Cursor::new(cursor.into_inner())).unwrap();
        let mut file = archive
            .by_name_decrypt("fixture", password.as_bytes())
            .unwrap();
        let mut content = String::new();
        file.read_to_string(&mut content).unwrap();
        assert_eq!(content, "sensitive fixture");
        drop(file);
        assert!(archive
            .by_name_decrypt("fixture", b"wrong-password")
            .is_err());
    }

    #[test]
    fn bundle_password_requires_six_characters() {
        assert!(validate_password("12345").is_err());
        assert!(validate_password("123456").is_ok());
    }

    #[test]
    fn archive_list_and_delete_are_scoped_to_configuration_zips() {
        let root = std::env::temp_dir().join(format!(
            "companion-sourcetree-config-archives-{}",
            time::OffsetDateTime::now_utc().unix_timestamp_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        let archive = "sourcetree-config-123.zip";
        fs::write(root.join(archive), "configuration").unwrap();
        fs::write(root.join("sourcetree-bookmarks-123.zip"), "other").unwrap();
        let archives = list_archives_at(&root).unwrap();
        assert_eq!(archives.archives.len(), 1);
        assert_eq!(archives.archives[0].name, archive);
        assert!(delete_archive_at(&root, archive).is_ok());
        assert!(!root.join(archive).exists());
        assert!(root.join("sourcetree-bookmarks-123.zip").exists());
        assert!(delete_archive_at(&root, "../sourcetree-bookmarks-123.zip").is_err());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn legacy_configuration_archives_move_without_overwriting() {
        let root = std::env::temp_dir().join(format!(
            "companion-sourcetree-config-legacy-{}",
            time::OffsetDateTime::now_utc().unix_timestamp_nanos()
        ));
        let legacy = root.join("backup");
        let destination = root.join("backups");
        fs::create_dir_all(&legacy).unwrap();
        fs::write(legacy.join("sourcetree-config-123.zip"), "move").unwrap();
        fs::write(legacy.join("sourcetree-config-456.zip"), "legacy").unwrap();
        fs::create_dir_all(&destination).unwrap();
        fs::write(destination.join("sourcetree-config-456.zip"), "current").unwrap();
        fs::write(legacy.join("sourcetree-bookmarks-123.zip"), "other").unwrap();

        migrate_legacy_archives_at(&legacy, &destination).unwrap();

        assert_eq!(
            fs::read(destination.join("sourcetree-config-123.zip")).unwrap(),
            b"move"
        );
        assert_eq!(
            fs::read(destination.join("sourcetree-config-456.zip")).unwrap(),
            b"current"
        );
        assert!(legacy.join("sourcetree-config-456.zip").exists());
        assert!(legacy.join("sourcetree-bookmarks-123.zip").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn full_allowlist_is_only_available_to_encrypted_bundles() {
        let entry = Entry {
            name: "passwd".into(),
            archive_path: format!("{ROOT}passwd"),
            destination: "passwd".into(),
            bytes: 1,
            sha256: "0".repeat(64),
        };
        let encrypted = Bundle {
            format_version: 2,
            kind: "dev-companion-sourcetree-config".into(),
            created_at: "2026-10-03T00:00:00Z".into(),
            sensitive: true,
            encryption: Some("aes-256".into()),
            profile: None,
            files: vec![entry.clone()],
        };
        assert!(validate(&encrypted).is_ok());
        let mut legacy = encrypted;
        legacy.format_version = 1;
        legacy.encryption = None;
        assert!(validate(&legacy).is_err());
    }

    #[test]
    fn primary_sourcetree_files_target_the_active_configuration_root() {
        let root = std::env::temp_dir().join(format!(
            "companion-sourcetree-config-targets-{}",
            time::OffsetDateTime::now_utc().unix_timestamp_nanos()
        ));
        fs::create_dir_all(root.join("SourceTree")).unwrap();
        let bundle = Bundle {
            format_version: 2,
            kind: "dev-companion-sourcetree-config".into(),
            created_at: "2026-10-03T00:00:00Z".into(),
            sensitive: true,
            encryption: Some("aes-256".into()),
            profile: None,
            files: ["bookmarks.xml", "opentabs.xml", "passwd", "userhosts"]
                .into_iter()
                .map(|name| Entry {
                    name: name.into(),
                    archive_path: format!("{ROOT}{name}"),
                    destination: name.into(),
                    bytes: 1,
                    sha256: "0".repeat(64),
                })
                .collect(),
        };

        let restored = targets_at(&root, &bundle).unwrap();

        assert_eq!(
            restored
                .iter()
                .map(|(_, path)| path.strip_prefix(&root).unwrap().to_path_buf())
                .collect::<Vec<_>>(),
            [
                PathBuf::from("SourceTree/bookmarks.xml"),
                PathBuf::from("SourceTree/opentabs.xml"),
                PathBuf::from("SourceTree/passwd"),
                PathBuf::from("SourceTree/userhosts"),
            ]
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn restore_targets_both_profiles_despite_stale_files_and_source_profile() {
        let root = std::env::temp_dir().join(format!(
            "companion-sourcetree-config-restore-profile-{}",
            time::OffsetDateTime::now_utc().unix_timestamp_nanos()
        ));
        let local = root.join("local");
        let roaming = root.join("roaming");
        fs::create_dir_all(local.join("SourceTree")).unwrap();
        fs::write(local.join("SourceTree/bookmarks.xml"), "stale").unwrap();
        let app = roaming.join("SourceTree.exe_Url_target/3.4.0.0");
        fs::create_dir_all(&app).unwrap();
        fs::write(app.join("user.config"), "target installation").unwrap();
        let bundle = Bundle {
            format_version: 3,
            kind: "dev-companion-sourcetree-config".into(),
            created_at: "2026-10-05T00:00:00Z".into(),
            sensitive: true,
            encryption: Some("aes-256".into()),
            profile: Some(ConfigProfile::Local),
            files: ["bookmarks.xml", "opentabs.xml", "passwd", "userhosts"]
                .into_iter()
                .map(|name| Entry {
                    name: name.into(),
                    archive_path: format!("{ROOT}{name}"),
                    destination: name.into(),
                    bytes: 1,
                    sha256: "0".repeat(64),
                })
                .collect(),
        };

        let restored = targets_for_at(&local, &roaming, &bundle).unwrap();

        assert_eq!(
            restored
                .iter()
                .map(|(_, path)| path.strip_prefix(&root).unwrap().to_path_buf())
                .collect::<Vec<_>>(),
            [
                PathBuf::from("local/SourceTree/bookmarks.xml"),
                PathBuf::from("roaming/SourceTree/bookmarks.xml"),
                PathBuf::from("local/SourceTree/opentabs.xml"),
                PathBuf::from("roaming/SourceTree/opentabs.xml"),
                PathBuf::from("local/SourceTree/passwd"),
                PathBuf::from("roaming/SourceTree/passwd"),
                PathBuf::from("local/SourceTree/userhosts"),
                PathBuf::from("roaming/SourceTree/userhosts"),
            ]
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn duplicate_profile_restore_overwrites_and_rolls_back_every_target() {
        let root = std::env::temp_dir().join(format!(
            "companion-sourcetree-config-restore-transaction-{}",
            time::OffsetDateTime::now_utc().unix_timestamp_nanos()
        ));
        let safety = root.join("safety");
        let local = root.join("local/bookmarks.xml");
        let roaming = root.join("roaming/bookmarks.xml");
        fs::create_dir_all(local.parent().unwrap()).unwrap();
        fs::create_dir_all(roaming.parent().unwrap()).unwrap();
        fs::create_dir_all(&safety).unwrap();
        fs::write(&local, "local-old").unwrap();
        fs::write(&roaming, "roaming-old").unwrap();
        let entry = Entry {
            name: "bookmarks.xml".into(),
            archive_path: format!("{ROOT}bookmarks.xml"),
            destination: "bookmarks.xml".into(),
            bytes: 3,
            sha256: "0".repeat(64),
        };

        let restored = restore_targets(
            vec![
                (entry.clone(), local.clone()),
                (entry.clone(), roaming.clone()),
            ],
            &safety,
            |_, target| fs::write(target, "new").map_err(|error| error.to_string()),
        )
        .unwrap();
        assert_eq!(restored.len(), 2);
        assert_eq!(fs::read_to_string(&local).unwrap(), "new");
        assert_eq!(fs::read_to_string(&roaming).unwrap(), "new");
        assert_eq!(
            fs::read_to_string(safety.join("0/bookmarks.xml")).unwrap(),
            "local-old"
        );
        assert_eq!(
            fs::read_to_string(safety.join("1/bookmarks.xml")).unwrap(),
            "roaming-old"
        );

        fs::write(&local, "local-old").unwrap();
        fs::write(&roaming, "roaming-old").unwrap();
        let content = b"changed";
        let mut good = entry.clone();
        good.bytes = content.len() as u64;
        good.sha256 = format!("{:x}", Sha256::digest(content));
        let mut bad = good.clone();
        bad.sha256 = "0".repeat(64);
        assert!(restore_targets(
            vec![(good, local.clone()), (bad, roaming.clone())],
            &root.join("rollback-safety"),
            |entry, target| replace_target(entry, target, &mut std::io::Cursor::new(content)),
        )
        .is_err());
        assert_eq!(fs::read_to_string(&local).unwrap(), "local-old");
        assert_eq!(fs::read_to_string(&roaming).unwrap(), "roaming-old");
        assert!(!local.with_extension("dev-companion.tmp").exists());
        assert!(!roaming.with_extension("dev-companion.tmp").exists());
        let existing_temporary = local.with_extension("dev-companion.tmp");
        fs::write(&existing_temporary, "other process").unwrap();
        assert!(replace_target(&entry, &local, &mut std::io::Cursor::new(b"new")).is_err());
        assert_eq!(
            fs::read_to_string(existing_temporary).unwrap(),
            "other process"
        );
        fs::remove_file(local.with_extension("dev-companion.tmp")).unwrap();
        fs::write(&local, "local-old").unwrap();
        fs::write(&roaming, "roaming-old").unwrap();
        let mut writes = 0;
        let incomplete_safety = root.join("incomplete-safety");
        let error = restore_targets(
            vec![(entry.clone(), local.clone()), (entry, roaming.clone())],
            &incomplete_safety,
            |_, target| {
                writes += 1;
                if writes == 1 {
                    fs::write(target, "changed").map_err(|error| error.to_string())?;
                } else {
                    fs::remove_file(target).map_err(|error| error.to_string())?;
                    fs::create_dir(target).map_err(|error| error.to_string())?;
                    return Err("forced rollback obstruction".into());
                }
                Ok(())
            },
        )
        .err()
        .unwrap();
        assert!(error.contains("Rollback incomplete for 1 target(s)"));
        assert!(error.contains(&incomplete_safety.display().to_string()));
        assert_eq!(fs::read_to_string(&local).unwrap(), "local-old");
        assert!(roaming.is_dir());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn restore_overwrites_all_four_primary_files_in_both_profiles() {
        let root = std::env::temp_dir().join(format!(
            "companion-sourcetree-config-four-files-{}",
            time::OffsetDateTime::now_utc().unix_timestamp_nanos()
        ));
        let safety = root.join("safety");
        fs::create_dir_all(&safety).unwrap();
        let mut targets = Vec::new();
        for name in ["bookmarks.xml", "opentabs.xml", "passwd", "userhosts"] {
            for profile in ["local", "roaming"] {
                let target = root.join(profile).join("SourceTree").join(name);
                fs::create_dir_all(target.parent().unwrap()).unwrap();
                fs::write(&target, "old").unwrap();
                targets.push((
                    Entry {
                        name: name.into(),
                        archive_path: format!("{ROOT}{name}"),
                        destination: name.into(),
                        bytes: name.len() as u64,
                        sha256: format!("{:x}", Sha256::digest(name.as_bytes())),
                    },
                    target,
                ));
            }
        }

        let restored = restore_targets(targets.clone(), &safety, |entry, target| {
            replace_target(
                entry,
                target,
                &mut std::io::Cursor::new(entry.name.as_bytes()),
            )
        })
        .unwrap();

        assert_eq!(restored.len(), 8);
        for (entry, target) in targets {
            assert_eq!(fs::read_to_string(target).unwrap(), entry.name);
        }
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn version_three_bundle_requires_its_source_profile() {
        let bundle = Bundle {
            format_version: 3,
            kind: "dev-companion-sourcetree-config".into(),
            created_at: "2026-10-05T00:00:00Z".into(),
            sensitive: true,
            encryption: Some("aes-256".into()),
            profile: None,
            files: vec![Entry {
                name: "bookmarks.xml".into(),
                archive_path: format!("{ROOT}bookmarks.xml"),
                destination: "bookmarks.xml".into(),
                bytes: 1,
                sha256: "0".repeat(64),
            }],
        };
        assert!(validate(&bundle).is_err());
        assert!(validate(&Bundle {
            profile: Some(ConfigProfile::Local),
            ..bundle
        })
        .is_ok());
    }
}
