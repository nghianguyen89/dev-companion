use crate::{environment, fs_safety, platform};
use serde::Serialize;
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
    time::SystemTime,
};
const DIRECTORY: &str = "cache/remote_plugin_catalog";
#[derive(Clone)]
struct Candidate {
    path: PathBuf,
    hash: String,
    modified: SystemTime,
    bytes: u64,
}
static SCAN: OnceLock<Mutex<Option<(String, Vec<Candidate>)>>> = OnceLock::new();
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Scan {
    pub token: String,
    pub location: String,
    pub files: usize,
    pub bytes: u64,
    pub skipped: usize,
    pub blocked: Option<String>,
}
#[derive(Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Outcome {
    pub deleted: usize,
    pub skipped: usize,
    pub reclaimed_bytes: u64,
    pub errors: Vec<String>,
}
fn eligible(path: &Path) -> Result<Candidate, String> {
    fs_safety::check(path)?;
    let name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or("Unknown filename")?;
    if path.extension().and_then(|s| s.to_str()) != Some("json")
        || name.len() != 16
        || !name.bytes().all(|c| c.is_ascii_hexdigit())
    {
        return Err("Unknown catalog filename".into());
    }
    let mut file = environment::locked_read(path)?;
    let metadata = file.metadata().map_err(|e| e.to_string())?;
    if metadata.len() > 64 * 1024 * 1024 {
        return Err("Unknown catalog size".into());
    }
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).map_err(|e| e.to_string())?;
    let value: serde_json::Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    if value["schema_version"] != 1
        || !value["plugins"].is_array()
        || !value["fetched_at"].is_string()
    {
        return Err("Unknown catalog schema".into());
    }
    Ok(Candidate {
        path: path.into(),
        hash: environment::digest(&bytes),
        modified: metadata.modified().map_err(|e| e.to_string())?,
        bytes: metadata.len(),
    })
}
fn scan_at(home: &Path) -> Result<(Vec<Candidate>, usize), String> {
    let root = home.join(DIRECTORY);
    fs_safety::check(&root)?;
    if !root.exists() {
        return Ok((Vec::new(), 0));
    }
    let mut candidates = Vec::new();
    let mut skipped = 0;
    for entry in fs::read_dir(root).map_err(|e| e.to_string())? {
        match entry
            .map_err(|e| e.to_string())
            .and_then(|e| eligible(&e.path()))
        {
            Ok(c) => candidates.push(c),
            Err(_) => skipped += 1,
        }
    }
    Ok((candidates, skipped))
}
pub fn scan() -> Result<Scan, String> {
    let home = platform::codex_home();
    let (candidates, skipped) = scan_at(&home)?;
    let token = format!(
        "cleanup-{}",
        time::OffsetDateTime::now_utc().unix_timestamp_nanos()
    );
    let result = Scan {
        token: token.clone(),
        location: home.join(DIRECTORY).to_string_lossy().into(),
        files: candidates.len(),
        bytes: candidates.iter().map(|c| c.bytes).sum(),
        skipped,
        blocked: environment::require_closed().err(),
    };
    *SCAN.get_or_init(Default::default).lock().unwrap() = Some((token, candidates));
    Ok(result)
}
pub fn execute(token: &str, confirmation: &str) -> Result<Outcome, String> {
    if confirmation != "CLEAN" {
        return Err("Type CLEAN to confirm.".into());
    }
    environment::require_closed()?;
    let mut scan = SCAN.get_or_init(Default::default).lock().unwrap();
    let Some((saved, _)) = scan.as_ref() else {
        return Err("Scan again.".into());
    };
    if saved != token {
        return Err("Scan expired.".into());
    }
    let (_, candidates) = scan.take().unwrap();
    Ok(execute_at(&platform::codex_home(), &candidates))
}
fn execute_at(home: &Path, candidates: &[Candidate]) -> Outcome {
    let root = home.join(DIRECTORY);
    let mut result = Outcome::default();
    for candidate in candidates {
        if candidate.path.parent() != Some(root.as_path()) {
            result.skipped += 1;
            continue;
        }
        match eligible(&candidate.path) {
            Ok(current)
                if current.hash == candidate.hash
                    && current.modified == candidate.modified
                    && current.bytes == candidate.bytes =>
            {
                if let Err(error) = fs::remove_file(&candidate.path) {
                    result.errors.push(error.to_string());
                } else {
                    result.deleted += 1;
                    result.reclaimed_bytes += current.bytes;
                }
            }
            _ => result.skipped += 1,
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cleanup_preserves_protected_unknown_changed_and_outside_files() {
        let home = std::env::temp_dir().join(format!(
            "companion-cleanup-test-{}",
            time::OffsetDateTime::now_utc().unix_timestamp_nanos()
        ));
        let root = home.join(DIRECTORY);
        fs::create_dir_all(&root).unwrap();
        let bytes = br#"{"schema_version":1,"fetched_at":"2026-09-07T00:00:00Z","plugins":[]}"#;
        let target = root.join("0123456789abcdef.json");
        fs::write(&target, bytes).unwrap();
        fs::write(root.join("unknown.json"), "do not delete").unwrap();
        fs::write(home.join("auth.json"), "protected").unwrap();
        let (candidates, skipped) = scan_at(&home).unwrap();
        assert_eq!(skipped, 1);
        assert_eq!(candidates.len(), 1);
        fs::write(&target, b"changed").unwrap();
        assert_eq!(execute_at(&home, &candidates).skipped, 1);
        fs::write(&target, bytes).unwrap();
        let (mut candidates, _) = scan_at(&home).unwrap();
        let mut outside = candidates[0].clone();
        outside.path = home.join("auth.json");
        candidates.push(outside);
        let result = execute_at(&home, &candidates);
        assert_eq!(result.deleted, 1);
        assert_eq!(result.skipped, 1);
        assert_eq!(result.reclaimed_bytes, bytes.len() as u64);
        assert!(home.join("auth.json").exists());
        assert!(root.join("unknown.json").exists());
        fs::remove_dir_all(home).unwrap();
    }
    #[test]
    fn cleanup_rejects_junction_root() {
        let home = std::env::temp_dir().join(format!(
            "companion-cleanup-link-{}",
            time::OffsetDateTime::now_utc().unix_timestamp_nanos()
        ));
        fs::create_dir_all(home.join("cache")).unwrap();
        fs::create_dir_all(home.join("outside")).unwrap();
        #[cfg(windows)]
        {
            let status = std::process::Command::new("cmd")
                .args(["/c", "mklink", "/J"])
                .arg(home.join(DIRECTORY).to_string_lossy().replace('/', "\\"))
                .arg(home.join("outside"))
                .status()
                .unwrap();
            assert!(status.success());
            assert!(scan_at(&home).is_err());
            fs::remove_dir(home.join(DIRECTORY)).unwrap();
        }
        fs::remove_dir_all(home).unwrap();
    }
}
