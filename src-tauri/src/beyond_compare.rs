//! The native Beyond Compare package remains opaque: Companion never parses it.
use std::{fs, path::Path};

use crate::fs_safety;

pub const APP_ID: &str = "beyond-compare";
pub const ARTIFACT_PATH: &str = "apps/beyond-compare/settings.bcpkg";
pub const MAX_PACKAGE_BYTES: u64 = 512 * 1024 * 1024;

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Readiness {
    pub supported: bool,
    pub secret_export_acknowledgement_required: bool,
}

pub fn readiness() -> Readiness {
    Readiness {
        supported: cfg!(windows),
        secret_export_acknowledgement_required: true,
    }
}

pub fn validate_package(path: &Path) -> Result<u64, String> {
    if !cfg!(windows) {
        return Err("Beyond Compare migration currently supports Windows only.".into());
    }
    if !path.is_absolute()
        || !path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("bcpkg"))
    {
        return Err("Choose a Beyond Compare .bcpkg package.".into());
    }
    fs_safety::check(path)?;
    let metadata =
        fs::symlink_metadata(path).map_err(|_| "Cannot read the selected package.".to_string())?;
    if fs_safety::linked(&metadata) || !metadata.is_file() || metadata.len() > MAX_PACKAGE_BYTES {
        return Err("The selected package is unsupported or exceeds the 512 MiB limit.".into());
    }
    Ok(metadata.len())
}
