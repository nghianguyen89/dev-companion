use std::{
    fs, io,
    path::{Component, Path},
};

pub fn linked(metadata: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        metadata.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        metadata.file_type().is_symlink()
    }
}

/// Inspect every existing ancestor, including CODEX_HOME itself. Never follow reparse points.
pub fn check(path: &Path) -> Result<(), String> {
    check_with_progress(path, &|_| {})
}

pub fn check_with_progress(path: &Path, progress: &dyn Fn(&Path)) -> Result<(), String> {
    if !path.is_absolute() || path.components().any(|c| matches!(c, Component::ParentDir)) {
        return Err("Rejected unsafe non-absolute path.".into());
    }
    for ancestor in path.ancestors() {
        progress(ancestor);
        match fs::symlink_metadata(ancestor) {
            Ok(m) if linked(&m) => return Err("Rejected symbolic link or junction.".into()),
            Ok(_) => (),
            Err(e) if e.kind() == io::ErrorKind::NotFound => (),
            Err(e) => return Err(format!("Cannot inspect path: {e}")),
        }
    }
    Ok(())
}

pub fn relative(name: &str) -> Result<(), String> {
    if name.is_empty()
        || name.contains(['\\', ':', '\0'])
        || name.split('/').any(|p| {
            let stem = p.split('.').next().unwrap_or("").to_ascii_uppercase();
            p.is_empty()
                || p == "."
                || p == ".."
                || p.ends_with(['.', ' '])
                || matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
                || (stem.len() == 4
                    && (stem.starts_with("COM") || stem.starts_with("LPT"))
                    && stem.as_bytes()[3].is_ascii_digit())
        })
    {
        return Err("Rejected unsafe archive path.".into());
    }
    Ok(())
}
