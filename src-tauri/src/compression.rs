use std::{
    collections::HashSet,
    fs::{self, File, OpenOptions},
    io::{self, BufReader, Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex, OnceLock,
    },
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use regex::Regex;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

use crate::fs_safety;

static ACTIVE: OnceLock<Mutex<Option<Arc<AtomicBool>>>> = OnceLock::new();

fn active() -> &'static Mutex<Option<Arc<AtomicBool>>> {
    ACTIVE.get_or_init(|| Mutex::new(None))
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Readiness {
    pub available: bool,
    pub path: Option<String>,
    pub message: String,
    pub running: bool,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TreeEntry {
    pub path: String,
    pub is_directory: bool,
    pub bytes: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceTree {
    pub source: String,
    pub default_output_folder: String,
    pub entries: Vec<TreeEntry>,
    pub skipped_reparse_points: usize,
}

#[derive(Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Mode {
    Fast,
    Strong,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompressionConfig {
    pub source: String,
    pub output_folder: String,
    pub mode: Mode,
    pub excluded_paths: Vec<String>,
    pub regex_exclusions: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandPreview {
    pub command: String,
    pub archive_path: String,
    pub included_files: usize,
    pub included_folders: usize,
    pub skipped_reparse_points: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Started {
    pub state: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProgressEvent {
    percent: u8,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct OutputEvent {
    line: String,
    stream: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct CompletionEvent {
    state: String,
    archive_path: Option<String>,
    exit_code: Option<i32>,
    message: String,
}

struct Prepared {
    output: PathBuf,
    archive: PathBuf,
    source_parent: PathBuf,
    list_entries: Vec<String>,
    level: &'static str,
    preview: CommandPreview,
}

pub fn readiness() -> Readiness {
    let path = seven_zip_path();
    Readiness {
        available: path.is_some(),
        path: path.as_deref().map(display_path),
        message: if path.is_some() {
            "7-Zip is ready."
        } else {
            "Install 7-Zip (7z.exe) to create .7z archives."
        }
        .into(),
        running: active().lock().map(|job| job.is_some()).unwrap_or(false),
    }
}

fn seven_zip_path() -> Option<PathBuf> {
    for key in ["ProgramFiles", "ProgramFiles(x86)"] {
        if let Some(root) = std::env::var_os(key) {
            let path = PathBuf::from(root).join("7-Zip").join("7z.exe");
            if usable_seven_zip(&path) {
                return Some(path);
            }
        }
    }
    std::env::split_paths(&std::env::var_os("PATH")?)
        .map(|folder| folder.join("7z.exe"))
        .find(|path| usable_seven_zip(path))
}

fn usable_seven_zip(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }
    let mut command = Command::new(path);
    command.arg("i").stdout(Stdio::null()).stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    command
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

pub fn scan_source(source: &str) -> Result<SourceTree, String> {
    let source = Path::new(source);
    fs_safety::check(source)?;
    let metadata = fs::symlink_metadata(source).map_err(|_| "Source folder is unavailable.")?;
    if fs_safety::linked(&metadata) || !metadata.is_dir() {
        return Err("Choose a regular source folder, not a link or junction.".into());
    }
    let source = source
        .canonicalize()
        .map_err(|_| "Source folder is unavailable.")?;
    let mut entries = Vec::new();
    let mut skipped = 0;
    walk(&source, &source, &mut entries, &mut skipped)?;
    Ok(SourceTree {
        default_output_folder: display_path(
            source.parent().ok_or("Source folder must have a parent.")?,
        ),
        source: display_path(&source),
        entries,
        skipped_reparse_points: skipped,
    })
}

fn walk(
    root: &Path,
    folder: &Path,
    entries: &mut Vec<TreeEntry>,
    skipped: &mut usize,
) -> Result<(), String> {
    let mut children = fs::read_dir(folder)
        .map_err(|_| "Unable to read a source folder.")?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| "Unable to read a source folder.")?;
    children.sort_by_key(|entry| entry.file_name());
    for entry in children {
        let metadata =
            fs::symlink_metadata(entry.path()).map_err(|_| "Unable to inspect a source item.")?;
        if fs_safety::linked(&metadata) {
            *skipped += 1;
            continue;
        }
        if !metadata.is_file() && !metadata.is_dir() {
            continue;
        }
        let relative = entry
            .path()
            .strip_prefix(root)
            .map_err(|_| "Invalid source item path.")?
            .components()
            .map(|part| {
                part.as_os_str()
                    .to_str()
                    .ok_or("Source item name is not valid Unicode.")
            })
            .collect::<Result<Vec<_>, _>>()?
            .join("/");
        entries.push(TreeEntry {
            path: relative,
            is_directory: metadata.is_dir(),
            bytes: metadata.len(),
        });
        if metadata.is_dir() {
            walk(root, &entry.path(), entries, skipped)?;
        }
    }
    Ok(())
}

fn prepare(config: &CompressionConfig) -> Result<Prepared, String> {
    let tree = scan_source(&config.source)?;
    let source = PathBuf::from(&tree.source);
    let source_name = source
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or("Source folder name is invalid.")?;
    let output = if config.output_folder.trim().is_empty() {
        PathBuf::from(&tree.default_output_folder)
    } else {
        PathBuf::from(&config.output_folder)
    };
    fs_safety::check(&output)?;
    if !output.is_dir() {
        return Err("Choose an existing output folder.".into());
    }
    let output = output
        .canonicalize()
        .map_err(|_| "Output folder is unavailable.")?;
    let archive = output.join(format!("{source_name}.7z"));
    if fs::symlink_metadata(&archive).is_ok() {
        return Err(
            "The archive already exists. Choose another output folder or move it first.".into(),
        );
    }
    let available: HashSet<_> = tree
        .entries
        .iter()
        .map(|entry| entry.path.to_lowercase())
        .collect();
    let mut excluded = HashSet::new();
    for path in &config.excluded_paths {
        let normalized = path.replace('\\', "/");
        fs_safety::relative(&normalized)?;
        let normalized = normalized.to_lowercase();
        if !available.contains(&normalized) {
            return Err(format!("Excluded item is no longer in the source: {path}"));
        }
        excluded.insert(normalized);
    }
    let regexes = config
        .regex_exclusions
        .iter()
        .map(|pattern| {
            Regex::new(pattern)
                .map_err(|error| format!("Invalid exclusion regex `{pattern}`: {error}"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut selected = Vec::new();
    for entry in &tree.entries {
        let lower = entry.path.to_lowercase();
        if excluded
            .iter()
            .any(|path| lower == *path || lower.starts_with(&format!("{path}/")))
        {
            continue;
        }
        if regexes.iter().any(|regex| {
            entry
                .path
                .split('/')
                .scan(String::new(), |prefix, part| {
                    if !prefix.is_empty() {
                        prefix.push('/');
                    }
                    prefix.push_str(part);
                    Some(regex.is_match(prefix))
                })
                .any(|matched| matched)
        }) {
            continue;
        }
        selected.push(entry);
    }
    let included_files = selected.iter().filter(|entry| !entry.is_directory).count();
    let mut empty_folders = Vec::new();
    for entry in selected.iter().filter(|entry| entry.is_directory) {
        if source_folder_is_empty(&source.join(&entry.path))? {
            empty_folders.push(*entry);
        }
    }
    let mut list_entries = selected
        .iter()
        .copied()
        .filter(|entry| !entry.is_directory)
        .chain(empty_folders.iter().copied())
        .map(|entry| format!("{source_name}\\{}", entry.path.replace('/', "\\")))
        .collect::<Vec<_>>();
    list_entries.sort();
    if list_entries.is_empty() {
        return Err("No files or folders remain after exclusions.".into());
    }
    let level = match config.mode {
        Mode::Fast => "-mx=1",
        Mode::Strong => "-mx=9",
    };
    let preview = CommandPreview {
        command: format!(
            "7z.exe a -t7z {level} -scsUTF-8 -snl -bsp1 \"{}\" @<temporary-selection-list>",
            display_path(&archive)
        ),
        archive_path: display_path(&archive),
        included_files,
        included_folders: empty_folders.len(),
        skipped_reparse_points: tree.skipped_reparse_points,
    };
    Ok(Prepared {
        output,
        archive,
        source_parent: source
            .parent()
            .ok_or("Source folder must have a parent.")?
            .to_owned(),
        list_entries,
        level,
        preview,
    })
}

pub fn preview(config: &CompressionConfig) -> Result<CommandPreview, String> {
    Ok(prepare(config)?.preview)
}

fn source_folder_is_empty(path: &Path) -> Result<bool, String> {
    fs_safety::check(path)?;
    let mut children = fs::read_dir(path).map_err(|_| "Unable to inspect a source folder.")?;
    match children.next() {
        None => Ok(true),
        Some(Ok(_)) => Ok(false),
        Some(Err(_)) => Err("Unable to inspect a source folder.".into()),
    }
}

pub fn start(app: AppHandle, config: CompressionConfig) -> Result<Started, String> {
    let prepared = prepare(&config)?;
    let seven_zip = seven_zip_path().ok_or("Install 7-Zip (7z.exe) to create archives.")?;
    let cancellation = Arc::new(AtomicBool::new(false));
    let mut guard = active()
        .lock()
        .map_err(|_| "Compression state is unavailable.")?;
    if guard.is_some() {
        return Err("A compression job is already running.".into());
    }
    *guard = Some(cancellation.clone());
    let spawned = thread::Builder::new()
        .name("file-compression".into())
        .spawn(move || {
            let result = run(&app, &prepared, &seven_zip, &cancellation);
            let event = match result {
                Ok((archive, exit_code, message)) => CompletionEvent {
                    state: "completed".into(),
                    archive_path: Some(archive),
                    exit_code: Some(exit_code),
                    message,
                },
                Err((message, exit_code)) => CompletionEvent {
                    state: if cancellation.load(Ordering::SeqCst) {
                        "cancelled"
                    } else {
                        "failed"
                    }
                    .into(),
                    archive_path: None,
                    exit_code,
                    message,
                },
            };
            if let Ok(mut guard) = active().lock() {
                *guard = None;
            }
            let _ = app.emit("compression-completed", event);
        });
    if spawned.is_err() {
        *guard = None;
        return Err("Unable to start compression worker.".into());
    }
    Ok(Started {
        state: "running".into(),
    })
}

pub fn cancel() -> Result<(), String> {
    let guard = active()
        .lock()
        .map_err(|_| "Compression state is unavailable.")?;
    let job = guard.as_ref().ok_or("No compression job is running.")?;
    job.store(true, Ordering::SeqCst);
    Ok(())
}

fn run(
    app: &AppHandle,
    prepared: &Prepared,
    seven_zip: &Path,
    cancellation: &AtomicBool,
) -> Result<(String, i32, String), (String, Option<i32>)> {
    if cancellation.load(Ordering::SeqCst) {
        return Err(("Compression cancelled.".into(), None));
    }
    if fs::symlink_metadata(&prepared.archive).is_ok() {
        return Err(("The archive already exists.".into(), None));
    }
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| ("Clock unavailable.".into(), None))?
        .as_nanos();
    let temp_dir = prepared
        .output
        .join(format!(".dev-companion-{}-{nonce}", std::process::id()));
    fs::create_dir(&temp_dir).map_err(|_| {
        (
            "Unable to create a private temporary archive folder.".into(),
            None,
        )
    })?;
    let list_path = temp_dir.join("selection.lst");
    let temp_archive = temp_dir.join("archive.7z");
    let result = (|| {
        let mut list = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&list_path)
            .map_err(|_| ("Unable to create temporary selection list.".into(), None))?;
        for entry in &prepared.list_entries {
            writeln!(list, "{entry}")
                .map_err(|_| ("Unable to write temporary selection list.".into(), None))?;
        }
        drop(list);
        let mut command = Command::new(seven_zip);
        command
            .current_dir(&prepared.source_parent)
            .args([
                "a",
                "-t7z",
                prepared.level,
                "-scsUTF-8",
                "-snl",
                "-bsp1",
                "-bso1",
                "-bse2",
            ])
            .arg(&temp_archive)
            .arg(format!("@{}", list_path.display()))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x0800_0000);
        }
        let mut child = command
            .spawn()
            .map_err(|_| ("Unable to launch 7-Zip.".into(), None))?;
        let stdout = child
            .stdout
            .take()
            .map(|pipe| emit_output(app.clone(), pipe, "stdout"));
        let stderr = child
            .stderr
            .take()
            .map(|pipe| emit_output(app.clone(), pipe, "stderr"));
        let status = loop {
            if cancellation.load(Ordering::SeqCst) {
                let _ = child.kill();
                break child.wait();
            }
            match child.try_wait() {
                Ok(Some(status)) => break Ok(status),
                Ok(None) => thread::sleep(Duration::from_millis(100)),
                Err(error) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    break Err(error);
                }
            }
        }
        .map_err(|_| ("Unable to wait for 7-Zip.".into(), None))?;
        if let Some(join) = stdout {
            let _ = join.join();
        }
        if let Some(join) = stderr {
            let _ = join.join();
        }
        if cancellation.load(Ordering::SeqCst) {
            return Err(("Compression cancelled.".into(), status.code()));
        }
        let exit_code = status.code().unwrap_or(-1);
        if exit_code != 0 {
            return Err((
                format!("7-Zip exited with code {exit_code}."),
                Some(exit_code),
            ));
        }
        let metadata = fs::metadata(&temp_archive)
            .map_err(|_| ("7-Zip did not produce an archive.".into(), Some(exit_code)))?;
        if !metadata.is_file() || metadata.len() == 0 {
            return Err(("7-Zip produced an empty archive.".into(), Some(exit_code)));
        }
        publish(&temp_archive, &prepared.archive).map_err(|error| (error, Some(exit_code)))?;
        let _ = app.emit("compression-progress", ProgressEvent { percent: 100 });
        Ok((display_path(&prepared.archive), exit_code))
    })();
    let cleanup = cleanup_temp(&list_path, &temp_archive, &temp_dir);
    match (result, cleanup) {
        (Ok((archive, code)), None) => Ok((archive, code, "Archive created.".into())),
        (Ok((archive, code)), Some(problem)) => {
            Ok((archive, code, format!("Archive created, but {problem}")))
        }
        (Err((message, code)), None) => Err((message, code)),
        (Err((message, code)), Some(problem)) => Err((format!("{message} {problem}"), code)),
    }
}

fn cleanup_temp(list: &Path, archive: &Path, directory: &Path) -> Option<String> {
    let mut failures = Vec::new();
    for (path, name) in [(list, "selection list"), (archive, "temporary archive")] {
        match fs::remove_file(path) {
            Ok(()) => (),
            Err(error) if error.kind() == io::ErrorKind::NotFound => (),
            Err(_) => failures.push(format!("could not remove the {name}")),
        }
    }
    if fs::remove_dir(directory).is_err() {
        failures.push("could not remove the temporary folder".into());
    }
    (!failures.is_empty()).then(|| {
        format!(
            "temporary cleanup is incomplete: {} at {}.",
            failures.join(", "),
            directory.display()
        )
    })
}

fn emit_output(
    app: AppHandle,
    pipe: impl io::Read + Send + 'static,
    stream: &'static str,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let mut reader = BufReader::new(pipe);
        let mut buffer = Vec::new();
        let mut chunk = [0_u8; 1024];
        let mut last_progress = None;
        loop {
            let count = match reader.read(&mut chunk) {
                Ok(0) | Err(_) => break,
                Ok(count) => count,
            };
            for byte in &chunk[..count] {
                if matches!(byte, b'\r' | b'\n') {
                    emit_output_segment(&app, &buffer, stream, &mut last_progress);
                    buffer.clear();
                } else {
                    buffer.push(*byte);
                }
            }
        }
        emit_output_segment(&app, &buffer, stream, &mut last_progress);
    })
}

fn emit_output_segment(
    app: &AppHandle,
    bytes: &[u8],
    stream: &'static str,
    last_progress: &mut Option<u8>,
) {
    let value = String::from_utf8_lossy(bytes).trim().to_owned();
    if value.is_empty() {
        return;
    }
    if stream == "stdout" {
        if let Some(percent) = progress_percent(&value) {
            if last_progress.replace(percent) != Some(percent) {
                let _ = app.emit("compression-progress", ProgressEvent { percent });
            }
            return;
        }
    }
    let _ = app.emit(
        "compression-output",
        OutputEvent {
            line: value,
            stream: stream.into(),
        },
    );
}

fn progress_percent(value: &str) -> Option<u8> {
    value
        .split_ascii_whitespace()
        .next()?
        .strip_suffix('%')?
        .parse::<u8>()
        .ok()
        .filter(|percent| *percent <= 100)
}

pub fn display_path(path: &Path) -> String {
    let value = path.display().to_string();
    #[cfg(windows)]
    {
        if let Some(unc) = value.strip_prefix(r"\\?\UNC\") {
            return format!(r"\\{unc}");
        }
        if let Some(normal) = value.strip_prefix(r"\\?\") {
            return normal.into();
        }
    }
    value
}

fn publish(temp: &Path, final_path: &Path) -> Result<(), String> {
    match fs::hard_link(temp, final_path) {
        Ok(()) => Ok(()),
        Err(error)
            if error.kind() == io::ErrorKind::AlreadyExists
                || fs::symlink_metadata(final_path).is_ok() =>
        {
            Err("The archive already exists; nothing was overwritten.".into())
        }
        Err(_) => {
            let mut source = File::open(temp).map_err(|_| "Unable to publish archive.")?;
            let mut destination = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(final_path)
                .map_err(|_| "Unable to create archive without overwriting.")?;
            if io::copy(&mut source, &mut destination).is_err() || destination.sync_all().is_err() {
                drop(destination);
                return Err(match fs::remove_file(final_path) {
                    Ok(()) => "Unable to publish archive completely; the incomplete archive was removed.".into(),
                    Err(_) => format!("Unable to publish archive completely; the incomplete archive at {} could not be removed.", final_path.display()),
                });
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_root(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "dev-companion-{name}-{}-{nonce}",
            std::process::id()
        ))
    }

    #[test]
    fn parses_only_complete_7zip_progress_tokens() {
        assert_eq!(progress_percent("  0%"), Some(0));
        assert_eq!(progress_percent("100%"), Some(100));
        assert_eq!(progress_percent(" 32% + input.bin"), Some(32));
        assert_eq!(progress_percent(" 99% 1 + input.bin"), Some(99));
        assert_eq!(progress_percent("7-Zip 26.03"), None);
        assert_eq!(progress_percent("101%"), None);
        assert_eq!(progress_percent("50% completed"), Some(50));
    }

    #[cfg(windows)]
    #[test]
    fn display_path_hides_windows_extended_prefix() {
        assert_eq!(
            display_path(Path::new(r"\\?\E:\Zalo Data")),
            r"E:\Zalo Data"
        );
        assert_eq!(
            display_path(Path::new(r"\\?\UNC\server\share")),
            r"\\server\share"
        );
    }

    #[test]
    fn selection_excludes_relative_folder_and_regex_matches_normalized_paths() {
        let root = temp_root("selection");
        fs::create_dir_all(root.join("source").join("nested")).unwrap();
        fs::write(root.join("source").join("nested").join("keep.txt"), "keep").unwrap();
        fs::write(root.join("source").join("nested").join("skip.log"), "skip").unwrap();
        let config = CompressionConfig {
            source: root.join("source").display().to_string(),
            output_folder: root.display().to_string(),
            mode: Mode::Fast,
            excluded_paths: vec![],
            regex_exclusions: vec![r"(?i)\.log$".into()],
        };
        let prepared = prepare(&config).unwrap();
        assert_eq!(prepared.preview.included_files, 1);
        assert!(prepared
            .list_entries
            .iter()
            .any(|entry| entry.ends_with("keep.txt")));
        assert!(!prepared
            .list_entries
            .iter()
            .any(|entry| entry.ends_with("skip.log")));
        let config = CompressionConfig {
            excluded_paths: vec!["nested".into()],
            regex_exclusions: vec![],
            ..config
        };
        assert!(prepare(&config).is_err());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn publish_refuses_existing_archive() {
        let root = temp_root("publish");
        fs::create_dir_all(&root).unwrap();
        let temp = root.join("temp.7z");
        let final_path = root.join("final.7z");
        fs::write(&temp, "new").unwrap();
        fs::write(&final_path, "old").unwrap();
        assert!(publish(&temp, &final_path).is_err());
        assert_eq!(fs::read_to_string(&final_path).unwrap(), "old");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn cleanup_reports_files_it_cannot_remove() {
        let root = temp_root("cleanup");
        fs::create_dir_all(root.join("selection.lst")).unwrap();
        fs::write(root.join("archive.7z"), "partial").unwrap();
        let message =
            cleanup_temp(&root.join("selection.lst"), &root.join("archive.7z"), &root).unwrap();
        assert!(message.contains("could not remove the selection list"));
        assert!(!root.join("archive.7z").exists());
        assert!(root.exists());
        fs::remove_dir(root.join("selection.lst")).unwrap();
        fs::remove_dir(root).unwrap();
    }

    #[test]
    fn excluding_only_nested_file_does_not_reinclude_it_as_an_empty_folder() {
        let root = temp_root("excluded-only-child");
        fs::create_dir_all(root.join("source").join("nested")).unwrap();
        fs::write(
            root.join("source").join("nested").join("secret.log"),
            "secret",
        )
        .unwrap();
        let result = prepare(&CompressionConfig {
            source: root.join("source").display().to_string(),
            output_folder: root.display().to_string(),
            mode: Mode::Fast,
            excluded_paths: vec![],
            regex_exclusions: vec![r"\.log$".into()],
        });
        assert!(
            matches!(result, Err(message) if message == "No files or folders remain after exclusions.")
        );
        assert!(!root.join("source.7z").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn directory_with_unlisted_reparse_point_is_not_treated_as_empty() {
        let root = temp_root("linked-child");
        let folder = root.join("source").join("nested");
        fs::create_dir_all(&folder).unwrap();
        let target = root.join("target.txt");
        fs::write(&target, "outside source").unwrap();
        let link = folder.join("shortcut.txt");
        #[cfg(windows)]
        let linked = std::os::windows::fs::symlink_file(&target, &link).is_ok();
        #[cfg(not(windows))]
        let linked = std::os::unix::fs::symlink(&target, &link).is_ok();
        if linked {
            let config = CompressionConfig {
                source: root.join("source").display().to_string(),
                output_folder: root.display().to_string(),
                mode: Mode::Fast,
                excluded_paths: vec![],
                regex_exclusions: vec![],
            };
            assert_eq!(
                scan_source(&config.source).unwrap().skipped_reparse_points,
                1
            );
            assert!(
                matches!(prepare(&config), Err(message) if message == "No files or folders remain after exclusions.")
            );
            fs::remove_file(link).unwrap();
        } else {
            fs::write(&link, "ordinary child").unwrap();
            assert!(!source_folder_is_empty(&folder).unwrap());
        }
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn seven_zip_archives_only_selected_files_when_installed() {
        let Some(seven_zip) = seven_zip_path() else {
            return;
        };
        let root = temp_root("cli");
        fs::create_dir_all(root.join("source").join("nested")).unwrap();
        fs::create_dir_all(root.join("source").join("private")).unwrap();
        fs::write(root.join("source").join("nested").join("keep.txt"), "keep").unwrap();
        fs::write(root.join("source").join("nested").join("skip.log"), "skip").unwrap();
        fs::write(
            root.join("source").join("private").join("secret.log"),
            "secret",
        )
        .unwrap();
        let prepared = prepare(&CompressionConfig {
            source: root.join("source").display().to_string(),
            output_folder: root.display().to_string(),
            mode: Mode::Strong,
            excluded_paths: vec![],
            regex_exclusions: vec![r"\.log$".into()],
        })
        .unwrap();
        assert!(!prepared
            .list_entries
            .iter()
            .any(|entry| entry.contains("private")));
        let list = root.join("selection.lst");
        fs::write(&list, prepared.list_entries.join("\n")).unwrap();
        let result = Command::new(&seven_zip)
            .current_dir(&prepared.source_parent)
            .args(["a", "-t7z", prepared.level, "-scsUTF-8", "-snl", "-bsp0"])
            .arg(&prepared.archive)
            .arg(format!("@{}", list.display()))
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        let listed = Command::new(&seven_zip)
            .args(["l", "-slt"])
            .arg(&prepared.archive)
            .output()
            .unwrap();
        assert!(listed.status.success());
        let listing = String::from_utf8_lossy(&listed.stdout);
        assert!(listing.contains("keep.txt"));
        assert!(!listing.contains("skip.log"));
        assert!(!listing.contains("secret.log"));
        assert!(!listing.contains("private"));
        fs::remove_dir_all(root).unwrap();
    }
}
