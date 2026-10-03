//! Strict personal-bundle-v1 container for the one supported Beyond Compare workflow.
use std::{
    collections::{HashMap, HashSet},
    fmt,
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

use crate::{beyond_compare, fs_safety, platform};

type Result<T> = std::result::Result<T, String>;
const MANIFEST_PATH: &str = "personal-manifest.json";
const FORMAT_VERSION: u32 = 1;
const KIND: &str = "codex-companion-personal-bundle";
const PLATFORM: &str = "windows";
const MAX_MANIFEST_BYTES: u64 = 1024 * 1024;
const MAX_ENTRIES: usize = 2;
const MAX_TOTAL_BYTES: u64 = beyond_compare::MAX_PACKAGE_BYTES;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Artifact {
    app_id: String,
    adapter_format_version: u32,
    source_app_version: ExplicitNull,
    kind: String,
    archive_path: String,
    files: u32,
    bytes: u64,
    sha256: String,
    restore_mode: String,
    secret_export_disabled_acknowledged: bool,
}

#[derive(Clone, Debug, PartialEq)]
struct ExplicitNull(bool);
impl Serialize for ExplicitNull {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_none()
    }
}
impl<'de> Deserialize<'de> for ExplicitNull {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct NullVisitor;
        impl<'de> serde::de::Visitor<'de> for NullVisitor {
            type Value = ExplicitNull;
            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("null")
            }
            fn visit_unit<E>(self) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(ExplicitNull(true))
            }
        }
        deserializer.deserialize_unit(NullVisitor)
    }
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
pub struct BundlePreview {
    pub token: String,
    pub package_name: String,
    pub bytes: u64,
    pub sensitive: bool,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BundleCreated {
    pub bundle_name: String,
    pub bytes: u64,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BundleInspection {
    pub token: String,
    pub bundle_name: String,
    pub created_at: String,
    pub bytes: u64,
    pub sensitive: bool,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryPreview {
    pub token: String,
    pub package_name: String,
    pub bytes: u64,
    pub staging_path: String,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryResult {
    pub recovered: bool,
    pub staging_path: String,
}

#[derive(Clone)]
enum Plan {
    Create {
        package: PathBuf,
        bytes: u64,
        sha256: String,
        credentials_included: bool,
    },
    Inspect {
        bundle: PathBuf,
        bundle_hash: String,
    },
    Recover {
        bundle: PathBuf,
        bundle_hash: String,
        target: PathBuf,
        bytes: u64,
    },
}
static PLANS: OnceLock<Mutex<HashMap<String, Plan>>> = OnceLock::new();
static NEXT: AtomicU64 = AtomicU64::new(1);

fn put(plan: Plan) -> String {
    let token = format!("personal-{}", NEXT.fetch_add(1, Ordering::Relaxed));
    let mut plans = PLANS
        .get_or_init(Default::default)
        .lock()
        .map_err(|_| ())
        .unwrap();
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
        .map_err(|_| "Preview expired; inspect again.".to_string())?
        .get(token)
        .cloned()
        .ok_or_else(|| "Preview expired; inspect again.".into())
}
fn hash_reader(mut reader: impl Read) -> Result<(String, u64)> {
    hash_reader_limited(&mut reader, MAX_TOTAL_BYTES)
}
fn hash_reader_limited(mut reader: impl Read, limit: u64) -> Result<(String, u64)> {
    let mut hash = Sha256::new();
    let mut bytes = 0u64;
    let mut buffer = [0u8; 65536];
    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|_| "Cannot read package data.".to_string())?;
        if read == 0 {
            break;
        }
        bytes = bytes
            .checked_add(read as u64)
            .ok_or("Package exceeds the size limit.")?;
        if bytes > limit {
            return Err("Package exceeds the size limit.".into());
        }
        hash.update(&buffer[..read]);
    }
    Ok((format!("{:x}", hash.finalize()), bytes))
}
fn hash_file(path: &Path) -> Result<(String, u64)> {
    hash_reader(File::open(path).map_err(|_| "Cannot read the selected package.".to_string())?)
}
fn open_bundle(path: &Path) -> Result<File> {
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
        .map_err(|_| "Cannot read the selected personal bundle.".to_string())?;
    if !file
        .metadata()
        .map_err(|_| "Cannot read the selected personal bundle.".to_string())?
        .is_file()
    {
        return Err("Cannot read the selected personal bundle.".into());
    }
    Ok(file)
}
fn hash_open_bundle(file: &mut File) -> Result<String> {
    file.seek(SeekFrom::Start(0))
        .map_err(|_| "Cannot read the selected personal bundle.".to_string())?;
    let hash = hash_reader_limited(&mut *file, MAX_TOTAL_BYTES + MAX_MANIFEST_BYTES + 65536)
        .map_err(|_| "Cannot read the selected personal bundle.".to_string())?
        .0;
    file.seek(SeekFrom::Start(0))
        .map_err(|_| "Cannot read the selected personal bundle.".to_string())?;
    Ok(hash)
}
fn timestamp() -> Result<String> {
    time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .map_err(|_| "Cannot create bundle timestamp.".into())
}
fn valid_hash(hash: &str) -> bool {
    hash.len() == 64
        && hash
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
fn exact_artifact(artifact: &Artifact) -> bool {
    artifact.app_id == beyond_compare::APP_ID
        && artifact.adapter_format_version == 1
        && artifact.source_app_version.0
        && artifact.kind == "settings-package"
        && artifact.archive_path == beyond_compare::ARTIFACT_PATH
        && artifact.files == 1
        && artifact.restore_mode == "manual"
        && artifact.bytes <= beyond_compare::MAX_PACKAGE_BYTES
        && valid_hash(&artifact.sha256)
}
fn validate_manifest(manifest: &Manifest) -> Result<&Artifact> {
    if manifest.format_version != FORMAT_VERSION
        || manifest.kind != KIND
        || manifest.platform != PLATFORM
        || !manifest.sensitive
        || manifest.artifacts.len() != 1
    {
        return Err("Unsupported personal bundle manifest.".into());
    }
    time::OffsetDateTime::parse(
        &manifest.created_at,
        &time::format_description::well_known::Rfc3339,
    )
    .map_err(|_| "Invalid personal bundle timestamp.".to_string())?;
    let artifact = &manifest.artifacts[0];
    if !exact_artifact(artifact) {
        return Err("Unsupported personal bundle artifact.".into());
    }
    Ok(artifact)
}
fn safe_name(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("settings.bcpkg")
        .to_owned()
}

pub fn preview_create(package: PathBuf, credentials_included: bool) -> Result<BundlePreview> {
    let declared = beyond_compare::validate_package(&package)?;
    let (sha256, bytes) = hash_file(&package)?;
    if bytes != declared {
        return Err("Selected package changed; choose it again.".into());
    }
    let package_name = safe_name(&package);
    let token = put(Plan::Create {
        package,
        bytes,
        sha256,
        credentials_included,
    });
    Ok(BundlePreview {
        token,
        package_name,
        bytes,
        sensitive: true,
    })
}

pub fn create(token: &str) -> Result<BundleCreated> {
    let Plan::Create {
        package,
        bytes,
        sha256,
        credentials_included,
    } = get(token)?
    else {
        return Err("Preview the Beyond Compare package first.".into());
    };
    if beyond_compare::validate_package(&package)? != bytes
        || hash_file(&package)? != (sha256.clone(), bytes)
    {
        return Err("Selected package changed; preview again.".into());
    }
    create_at(
        &package,
        bytes,
        sha256,
        credentials_included,
        &platform::personal_bundle_dir(),
    )
}
fn create_at(
    package: &Path,
    bytes: u64,
    sha256: String,
    credentials_included: bool,
    output: &Path,
) -> Result<BundleCreated> {
    fs_safety::check(output)?;
    fs::create_dir_all(output)
        .map_err(|_| "Cannot create the Companion bundle folder.".to_string())?;
    fs_safety::check(output)?;
    let stamp = time::OffsetDateTime::now_utc().unix_timestamp_nanos();
    let path = output.join(format!("personal-bundle-{stamp}.zip"));
    let mut created = false;
    let result = (|| -> Result<()> {
        let output_file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&path)
            .map_err(|_| "Cannot create the personal bundle.".to_string())?;
        created = true;
        let artifact = Artifact {
            app_id: beyond_compare::APP_ID.into(),
            adapter_format_version: 1,
            source_app_version: ExplicitNull(true),
            kind: "settings-package".into(),
            archive_path: beyond_compare::ARTIFACT_PATH.into(),
            files: 1,
            bytes,
            sha256: sha256.clone(),
            restore_mode: "manual".into(),
            secret_export_disabled_acknowledged: !credentials_included,
        };
        let manifest = Manifest {
            format_version: FORMAT_VERSION,
            kind: KIND.into(),
            created_at: timestamp()?,
            platform: PLATFORM.into(),
            sensitive: true,
            artifacts: vec![artifact],
        };
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        let mut zip = ZipWriter::new(output_file);
        zip.start_file(MANIFEST_PATH, options)
            .map_err(|_| "Cannot create the personal bundle.".to_string())?;
        zip.write_all(
            &serde_json::to_vec(&manifest)
                .map_err(|_| "Cannot create the personal bundle.".to_string())?,
        )
        .map_err(|_| "Cannot create the personal bundle.".to_string())?;
        zip.start_file(beyond_compare::ARTIFACT_PATH, options)
            .map_err(|_| "Cannot create the personal bundle.".to_string())?;
        let mut input = File::open(package)
            .map_err(|_| "Selected package changed; preview again.".to_string())?;
        let mut copied = 0u64;
        let mut copied_hash = Sha256::new();
        let mut buffer = [0u8; 65536];
        loop {
            let count = input
                .read(&mut buffer)
                .map_err(|_| "Cannot package the selected file.".to_string())?;
            if count == 0 {
                break;
            }
            copied = copied
                .checked_add(count as u64)
                .ok_or("Selected package exceeds the size limit.")?;
            if copied > beyond_compare::MAX_PACKAGE_BYTES {
                return Err("Selected package exceeds the 512 MiB limit.".into());
            }
            copied_hash.update(&buffer[..count]);
            zip.write_all(&buffer[..count])
                .map_err(|_| "Cannot package the selected file.".to_string())?;
        }
        if copied != bytes || format!("{:x}", copied_hash.finalize()) != sha256 {
            return Err("Selected package changed; preview again.".into());
        }
        zip.finish()
            .map_err(|_| "Cannot finalize the personal bundle.".to_string())?
            .sync_all()
            .map_err(|_| "Cannot finalize the personal bundle.".to_string())?;
        inspect_at(&path).map(|_| ())
    })();
    if let Err(error) = result {
        if created && fs::remove_file(&path).is_err() {
            return Err(
                "Personal bundle creation failed and its partial file could not be removed.".into(),
            );
        }
        return Err(error);
    }
    Ok(BundleCreated {
        bundle_name: safe_name(&path),
        bytes: fs::metadata(&path)
            .map_err(|_| "Cannot verify the personal bundle.".to_string())?
            .len(),
    })
}

fn inspect_archive(archive: &mut ZipArchive<File>) -> Result<(Manifest, u64)> {
    if archive.len() != MAX_ENTRIES {
        return Err("Personal bundle has an invalid ZIP inventory.".into());
    }
    let mut names = HashSet::new();
    let mut total = 0u64;
    for index in 0..archive.len() {
        let entry = archive
            .by_index(index)
            .map_err(|_| "Invalid personal bundle.".to_string())?;
        fs_safety::relative(entry.name())
            .map_err(|_| "Personal bundle has an unsafe ZIP path.".to_string())?;
        if entry.is_dir()
            || entry
                .unix_mode()
                .is_some_and(|mode| mode & 0o170000 == 0o120000)
            || !names.insert(entry.name().to_ascii_lowercase())
        {
            return Err("Personal bundle has an invalid ZIP inventory.".into());
        }
        if entry.name() == MANIFEST_PATH {
            if entry.size() > MAX_MANIFEST_BYTES {
                return Err("Personal bundle manifest exceeds the size limit.".into());
            }
        } else if entry.name() == beyond_compare::ARTIFACT_PATH {
            if entry.size() > beyond_compare::MAX_PACKAGE_BYTES {
                return Err("Personal bundle artifact exceeds the size limit.".into());
            }
        } else {
            return Err("Personal bundle has unexpected ZIP entries.".into());
        }
        total = total
            .checked_add(entry.size())
            .ok_or("Personal bundle exceeds the size limit.")?;
        if total > MAX_TOTAL_BYTES + MAX_MANIFEST_BYTES {
            return Err("Personal bundle exceeds the size limit.".into());
        }
    }
    if !names.contains(MANIFEST_PATH) || !names.contains(beyond_compare::ARTIFACT_PATH) {
        return Err("Personal bundle has an incomplete ZIP inventory.".into());
    }
    let mut manifest_bytes = Vec::new();
    archive
        .by_name(MANIFEST_PATH)
        .map_err(|_| "Personal bundle has an incomplete ZIP inventory.".to_string())?
        .take(MAX_MANIFEST_BYTES + 1)
        .read_to_end(&mut manifest_bytes)
        .map_err(|_| "Cannot read the personal bundle manifest.".to_string())?;
    if manifest_bytes.len() as u64 > MAX_MANIFEST_BYTES {
        return Err("Personal bundle manifest exceeds the size limit.".into());
    }
    let manifest: Manifest = serde_json::from_slice(&manifest_bytes)
        .map_err(|_| "Invalid personal bundle manifest.".to_string())?;
    let artifact = validate_manifest(&manifest)?.clone();
    let file = archive
        .by_name(&artifact.archive_path)
        .map_err(|_| "Personal bundle has an incomplete ZIP inventory.".to_string())?;
    if file.size() != artifact.bytes || hash_reader(file)? != (artifact.sha256, artifact.bytes) {
        return Err("Personal bundle artifact verification failed.".into());
    }
    Ok((manifest, total))
}
fn inspect_at(path: &Path) -> Result<(Manifest, u64)> {
    let file = open_bundle(path)?;
    let mut archive = ZipArchive::new(file).map_err(|_| "Invalid personal bundle.".to_string())?;
    inspect_archive(&mut archive)
}
fn inspect_and_hash_at(path: &Path) -> Result<(Manifest, u64, String)> {
    let mut file = open_bundle(path)?;
    let hash = hash_open_bundle(&mut file)?;
    let mut archive = ZipArchive::new(file).map_err(|_| "Invalid personal bundle.".to_string())?;
    let (manifest, bytes) = inspect_archive(&mut archive)?;
    Ok((manifest, bytes, hash))
}
fn revalidate_bundle(path: &Path, expected_hash: &str) -> Result<(Manifest, u64)> {
    let (manifest, bytes, actual_hash) = inspect_and_hash_at(path)?;
    if actual_hash != expected_hash {
        return Err("Personal bundle changed; inspect again.".into());
    }
    Ok((manifest, bytes))
}

pub fn inspect(path: PathBuf) -> Result<BundleInspection> {
    let (manifest, bytes, hash) = inspect_and_hash_at(&path)?;
    let bundle_name = safe_name(&path);
    let token = put(Plan::Inspect {
        bundle: path,
        bundle_hash: hash,
    });
    Ok(BundleInspection {
        token,
        bundle_name,
        created_at: manifest.created_at,
        bytes,
        sensitive: true,
    })
}

pub fn preview_recovery(token: &str) -> Result<RecoveryPreview> {
    let Plan::Inspect {
        bundle,
        bundle_hash,
    } = get(token)?
    else {
        return Err("Inspect the personal bundle first.".into());
    };
    let (manifest, _) = revalidate_bundle(&bundle, &bundle_hash)?;
    let artifact = validate_manifest(&manifest)?;
    let target = platform::personal_staging_dir()
        .join("beyond-compare")
        .join(format!("recovery-{}", NEXT.fetch_add(1, Ordering::Relaxed)))
        .join("settings.bcpkg");
    fs_safety::check(&target)?;
    if target.exists() {
        return Err("Companion staging destination already exists; inspect again.".into());
    }
    let staging_path = target.to_string_lossy().into_owned();
    let recovery_token = put(Plan::Recover {
        bundle,
        bundle_hash,
        target,
        bytes: artifact.bytes,
    });
    Ok(RecoveryPreview {
        token: recovery_token,
        package_name: "settings.bcpkg".into(),
        bytes: artifact.bytes,
        staging_path,
    })
}

fn restore_at(
    bundle: &Path,
    bundle_hash: &str,
    target: &Path,
    bytes: u64,
    fail_after_write: bool,
) -> Result<()> {
    let mut file = open_bundle(bundle)?;
    if hash_open_bundle(&mut file)? != bundle_hash {
        return Err("Personal bundle changed; inspect again.".into());
    }
    let mut archive = ZipArchive::new(file).map_err(|_| "Invalid personal bundle.".to_string())?;
    let (manifest, _) = inspect_archive(&mut archive)?;
    let artifact = validate_manifest(&manifest)?;
    if artifact.bytes != bytes || target.exists() {
        return Err("Recovery destination changed; preview again.".into());
    }
    let parent = target
        .parent()
        .ok_or("Invalid Companion staging destination.")?;
    fs_safety::check(parent)?;
    fs::create_dir_all(parent).map_err(|_| "Cannot create Companion staging.".to_string())?;
    fs_safety::check(target)?;
    if target.exists() {
        return Err("Recovery destination changed; preview again.".into());
    }
    let mut created = false;
    let attempt = (|| -> Result<()> {
        let entry = archive
            .by_name(beyond_compare::ARTIFACT_PATH)
            .map_err(|_| "Personal bundle has an incomplete ZIP inventory.".to_string())?;
        let mut output = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(target)
            .map_err(|_| "Recovery destination changed; preview again.".to_string())?;
        created = true;
        let limit = bytes
            .checked_add(1)
            .ok_or("Recovered package exceeds the size limit.")?;
        let copied = std::io::copy(&mut entry.take(limit), &mut output)
            .map_err(|_| "Cannot recover the package to Companion staging.".to_string())?;
        if copied != bytes {
            return Err("Recovered package size verification failed.".into());
        }
        output
            .sync_all()
            .map_err(|_| "Cannot recover the package to Companion staging.".to_string())?;
        drop(output);
        if fail_after_write {
            return Err("Injected recovery failure.".into());
        }
        if hash_file(target)? != (artifact.sha256.clone(), artifact.bytes) {
            return Err("Recovered package verification failed.".into());
        }
        Ok(())
    })();
    if let Err(error) = attempt {
        if created && fs::remove_file(target).is_err() {
            return Err("Recovery failed and Companion staging could not be rolled back.".into());
        }
        return Err(error);
    }
    Ok(())
}

pub fn recover(token: &str, confirmation: &str) -> Result<RecoveryResult> {
    if confirmation != "RECOVER" {
        return Err("Type RECOVER to confirm.".into());
    }
    let Plan::Recover {
        bundle,
        bundle_hash,
        target,
        bytes,
    } = get(token)?
    else {
        return Err("Preview recovery first.".into());
    };
    restore_at(&bundle, &bundle_hash, &target, bytes, false)?;
    Ok(RecoveryResult {
        recovered: true,
        staging_path: target.to_string_lossy().into_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn root() -> PathBuf {
        std::env::temp_dir().join(format!(
            "personal-bundle-test-{}",
            time::OffsetDateTime::now_utc().unix_timestamp_nanos()
        ))
    }
    fn package(root: &Path) -> PathBuf {
        let path = root.join("settings.bcpkg");
        fs::create_dir_all(root).unwrap();
        fs::write(&path, b"opaque-synthetic-package").unwrap();
        path
    }
    fn write_bundle(path: &Path, manifest: Manifest, extra: Option<&str>) {
        let file = File::create(path).unwrap();
        let mut zip = ZipWriter::new(file);
        let options = SimpleFileOptions::default();
        zip.start_file(MANIFEST_PATH, options).unwrap();
        zip.write_all(&serde_json::to_vec(&manifest).unwrap())
            .unwrap();
        zip.start_file(beyond_compare::ARTIFACT_PATH, options)
            .unwrap();
        zip.write_all(b"opaque-synthetic-package").unwrap();
        if let Some(name) = extra {
            zip.start_file(name, options).unwrap();
            zip.write_all(b"x").unwrap();
        }
        zip.finish().unwrap();
    }
    fn manifest() -> Manifest {
        let hash = format!("{:x}", Sha256::digest(b"opaque-synthetic-package"));
        Manifest {
            format_version: 1,
            kind: KIND.into(),
            created_at: "2026-09-07T00:00:00Z".into(),
            platform: PLATFORM.into(),
            sensitive: true,
            artifacts: vec![Artifact {
                app_id: beyond_compare::APP_ID.into(),
                adapter_format_version: 1,
                source_app_version: ExplicitNull(true),
                kind: "settings-package".into(),
                archive_path: beyond_compare::ARTIFACT_PATH.into(),
                files: 1,
                bytes: 24,
                sha256: hash,
                restore_mode: "manual".into(),
                secret_export_disabled_acknowledged: true,
            }],
        }
    }
    #[test]
    fn roundtrip_records_credential_choice_and_keeps_package_opaque() {
        if !cfg!(windows) {
            return;
        }
        let root = root();
        let package = package(&root);
        let preview = preview_create(package.clone(), true).unwrap();
        let created = create_at(
            &package,
            preview.bytes,
            hash_file(&package).unwrap().0,
            true,
            &root.join("bundles"),
        )
        .unwrap();
        assert!(created.bundle_name.ends_with(".zip"));
        let bundle = root.join("bundles").join(created.bundle_name);
        let (manifest, _) = inspect_at(&bundle).unwrap();
        assert!(!manifest.artifacts[0].secret_export_disabled_acknowledged);
        let inspected = inspect(bundle).unwrap();
        let recovery = preview_recovery(&inspected.token).unwrap();
        assert_eq!(recovery.package_name, "settings.bcpkg");
        assert!(recovery.bytes > 0 && recovery.staging_path.contains("staging"));
    }
    #[test]
    fn rejects_extra_unsafe_and_case_colliding_zip_entries() {
        let root = root();
        fs::create_dir_all(&root).unwrap();
        let bundle = root.join("bad.zip");
        write_bundle(&bundle, manifest(), Some("apps/beyond-compare/extra.txt"));
        assert!(inspect_at(&bundle).is_err());
        let file = File::create(&bundle).unwrap();
        let mut zip = ZipWriter::new(file);
        let options = SimpleFileOptions::default();
        let m = manifest();
        zip.start_file(MANIFEST_PATH, options).unwrap();
        zip.write_all(&serde_json::to_vec(&m).unwrap()).unwrap();
        zip.start_file(beyond_compare::ARTIFACT_PATH, options)
            .unwrap();
        zip.write_all(b"opaque-synthetic-package").unwrap();
        zip.start_file("APPS/BEYOND-COMPARE/SETTINGS.BCPKG", options)
            .unwrap();
        zip.write_all(b"x").unwrap();
        zip.finish().unwrap();
        assert!(inspect_at(&bundle).is_err());
    }
    #[test]
    fn rejects_unknown_manifest_fields_and_hash_mismatches() {
        let root = root();
        fs::create_dir_all(&root).unwrap();
        let bundle = root.join("strict.zip");
        let file = File::create(&bundle).unwrap();
        let mut zip = ZipWriter::new(file);
        let options = SimpleFileOptions::default();
        let mut raw_manifest = serde_json::to_value(manifest()).unwrap();
        raw_manifest
            .as_object_mut()
            .unwrap()
            .insert("futureField".into(), serde_json::Value::Bool(true));
        zip.start_file(MANIFEST_PATH, options).unwrap();
        zip.write_all(&serde_json::to_vec(&raw_manifest).unwrap())
            .unwrap();
        zip.start_file(beyond_compare::ARTIFACT_PATH, options)
            .unwrap();
        zip.write_all(b"opaque-synthetic-package").unwrap();
        zip.finish().unwrap();
        assert!(inspect_at(&bundle).is_err());
        let hash_mismatch = root.join("hash-mismatch.zip");
        let mut manifest = manifest();
        manifest.artifacts[0].sha256 = "0".repeat(64);
        write_bundle(&hash_mismatch, manifest, None);
        assert!(inspect_at(&hash_mismatch).is_err());
    }
    #[test]
    fn rejects_manifest_without_explicit_null_source_app_version() {
        let root = root();
        fs::create_dir_all(&root).unwrap();
        let bundle = root.join("missing-source-version.zip");
        let file = File::create(&bundle).unwrap();
        let mut zip = ZipWriter::new(file);
        let options = SimpleFileOptions::default();
        let mut manifest = serde_json::to_value(manifest()).unwrap();
        manifest["artifacts"][0]
            .as_object_mut()
            .unwrap()
            .remove("sourceAppVersion");
        zip.start_file(MANIFEST_PATH, options).unwrap();
        zip.write_all(&serde_json::to_vec(&manifest).unwrap())
            .unwrap();
        zip.start_file(beyond_compare::ARTIFACT_PATH, options)
            .unwrap();
        zip.write_all(b"opaque-synthetic-package").unwrap();
        zip.finish().unwrap();
        assert!(inspect_at(&bundle).is_err());
    }
    #[test]
    fn rejects_duplicate_manifest_keys() {
        let root = root();
        fs::create_dir_all(&root).unwrap();
        let bundle = root.join("duplicate-key.zip");
        let file = File::create(&bundle).unwrap();
        let mut zip = ZipWriter::new(file);
        let options = SimpleFileOptions::default();
        let manifest = serde_json::to_string(&manifest()).unwrap().replacen(
            "\"sourceAppVersion\":null",
            "\"sourceAppVersion\":null,\"sourceAppVersion\":null",
            1,
        );
        zip.start_file(MANIFEST_PATH, options).unwrap();
        zip.write_all(manifest.as_bytes()).unwrap();
        zip.start_file(beyond_compare::ARTIFACT_PATH, options)
            .unwrap();
        zip.write_all(b"opaque-synthetic-package").unwrap();
        zip.finish().unwrap();
        assert!(inspect_at(&bundle).is_err());
    }
    #[test]
    fn recovery_is_create_new_and_rolls_back_failed_write() {
        let root = root();
        fs::create_dir_all(&root).unwrap();
        let bundle = root.join("bundle.zip");
        write_bundle(&bundle, manifest(), None);
        let hash = hash_file(&bundle).unwrap().0;
        let target = root.join("stage/settings.bcpkg");
        assert!(restore_at(&bundle, &hash, &target, 24, true).is_err());
        assert!(!target.exists());
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::write(&target, b"keep").unwrap();
        assert!(restore_at(&bundle, &hash, &target, 24, false).is_err());
        assert_eq!(fs::read(&target).unwrap(), b"keep");
        let swapped = root.join("swapped/settings.bcpkg");
        assert!(restore_at(&bundle, &"0".repeat(64), &swapped, 24, false).is_err());
        assert!(!swapped.exists());
    }
    #[test]
    fn create_revalidates_the_selected_package() {
        if !cfg!(windows) {
            return;
        }
        let root = root();
        let source = package(&root);
        let preview = preview_create(source.clone(), false).unwrap();
        fs::write(source, b"changed").unwrap();
        assert!(create(&preview.token).is_err());
    }
}
