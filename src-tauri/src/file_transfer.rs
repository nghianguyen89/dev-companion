use std::{
    fs,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{Mutex, OnceLock},
    thread,
    time::Instant,
};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

use crate::{config, platform};

const MAX_HISTORY_ENTRIES: usize = 100;
static ACTIVE_TRANSFER: OnceLock<Mutex<Option<u32>>> = OnceLock::new();

fn active_transfer() -> &'static Mutex<Option<u32>> {
    ACTIVE_TRANSFER.get_or_init(|| Mutex::new(None))
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum TransferMode {
    SimpleCopy,
    FastCopy,
    ProjectMigration,
    Mirror,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferConfig {
    pub source: String,
    pub destination: String,
    pub mode: TransferMode,
    pub include_subfolders: bool,
    pub preserve_timestamps: bool,
    pub skip_junction_points: bool,
    pub restartable: bool,
    pub copy_empty_directories: bool,
    pub verify_destination: bool,
    pub save_log: bool,
    pub shutdown_when_finished: bool,
    pub threads: u8,
    pub retries: u8,
    pub retry_wait: u8,
    pub exclude_folders: Vec<String>,
    pub exclude_files: Vec<String>,
    #[serde(default)]
    pub selection_enabled: bool,
    #[serde(default)]
    pub selected_entries: Vec<String>,
    #[serde(default)]
    pub mirror_confirmed: bool,
    #[serde(default)]
    pub system_location_confirmed: bool,
    #[serde(default)]
    pub destination_data_confirmed: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferProfile {
    pub id: String,
    pub name: String,
    pub config: TransferConfig,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Readiness {
    pub available: bool,
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectoryEntry {
    pub name: String,
    pub path: String,
    pub is_directory: bool,
    pub is_hidden: bool,
    pub is_system: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectoryBreadcrumb {
    pub label: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectoryListing {
    pub path: String,
    pub parent: Option<String>,
    pub breadcrumbs: Vec<DirectoryBreadcrumb>,
    pub entries: Vec<DirectoryEntry>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandPreview {
    pub command: String,
    pub arguments: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Started {
    pub state: String,
    pub command: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputEvent {
    pub line: String,
    pub stream: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressEvent {
    pub bytes_copied: u64,
    pub total_bytes: u64,
    pub percent: u8,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Summary {
    pub files_copied: Option<u64>,
    pub files_skipped: Option<u64>,
    pub files_failed: Option<u64>,
    pub bytes_copied: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExitInterpretation {
    pub status: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompletionEvent {
    pub phase: String,
    pub state: String,
    pub exit_code: Option<i32>,
    pub interpretation: ExitInterpretation,
    pub summary: Summary,
    pub duration_seconds: u64,
    pub log_path: Option<String>,
    pub verification: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferHistoryEntry {
    pub id: String,
    pub started_at: String,
    pub completed_at: String,
    pub source: String,
    pub destination: String,
    pub preset: TransferMode,
    pub status: String,
    pub bytes_copied: Option<u64>,
    pub files_copied: Option<u64>,
    pub duration_seconds: u64,
    pub exit_code: Option<i32>,
    pub log_path: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct HistoryFile {
    format_version: u32,
    entries: Vec<TransferHistoryEntry>,
}

pub fn readiness() -> Readiness {
    let path = robocopy_path();
    Readiness {
        available: path.is_some(),
        path: path.map(|value| value.display().to_string()),
    }
}

pub fn list_directory(path: String) -> Result<DirectoryListing, String> {
    let root = PathBuf::from(path);
    if !root.is_dir() {
        return Err("The selected folder is unavailable.".into());
    }
    let root = root
        .canonicalize()
        .map_err(|_| "The selected folder is unavailable.")?;
    let mut entries = fs::read_dir(&root)
        .map_err(|_| "Unable to list the selected folder.")?
        .filter_map(Result::ok)
        .filter_map(|entry| directory_entry(entry).ok())
        .collect::<Vec<_>>();
    entries.sort_by(|left, right| {
        right.is_directory.cmp(&left.is_directory).then_with(|| {
            left.name
                .to_ascii_lowercase()
                .cmp(&right.name.to_ascii_lowercase())
        })
    });
    Ok(DirectoryListing {
        path: display_path(&root),
        parent: root.parent().map(display_path),
        breadcrumbs: directory_breadcrumbs(&root),
        entries,
    })
}

pub fn preview(config: &TransferConfig) -> Result<CommandPreview, String> {
    let arguments = command_arguments(config, false)?;
    Ok(CommandPreview {
        command: display_command(&arguments),
        arguments,
        warnings: warnings(config),
    })
}

pub fn start(
    app: AppHandle,
    config: TransferConfig,
    analyze: bool,
    progress_total_bytes: Option<u64>,
) -> Result<Started, String> {
    validate(&config)?;
    let robocopy = robocopy_path().ok_or("Robocopy is not available on this system.")?;
    let arguments = command_arguments(&config, analyze)?;
    let command = display_command(&arguments);
    let mut active = active_transfer()
        .lock()
        .map_err(|_| "Transfer state is unavailable.")?;
    if active.is_some() {
        return Err("A file transfer is already running.".into());
    }
    let mut child = hidden_command(robocopy)
        .args(&arguments)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("Unable to start Robocopy: {error}"))?;
    let pid = child.id();
    *active = Some(pid);
    drop(active);
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let background_command = command.clone();
    thread::spawn(move || {
        run_transfer(
            app,
            config,
            analyze,
            background_command,
            child,
            stdout,
            stderr,
            progress_total_bytes,
        )
    });
    Ok(Started {
        state: if analyze { "analyzing" } else { "running" }.into(),
        command,
    })
}

pub fn cancel() -> Result<(), String> {
    let pid = *active_transfer()
        .lock()
        .map_err(|_| "Transfer state is unavailable.")?;
    let pid = pid.ok_or("No file transfer is running.")?;
    let status = hidden_command("taskkill.exe")
        .args(["/PID", &pid.to_string(), "/T", "/F"])
        .status()
        .map_err(|error| format!("Unable to cancel Robocopy: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err("Robocopy could not be cancelled.".into())
    }
}

pub fn list_history(
    configuration: &config::AppConfiguration,
) -> Result<Vec<TransferHistoryEntry>, String> {
    read_history(&history_path(configuration))
}

pub fn open_log(configuration: &config::AppConfiguration, value: &str) -> Result<(), String> {
    let root = logs_dir(configuration)
        .canonicalize()
        .map_err(|_| "File transfer logs are unavailable.")?;
    let path = Path::new(value)
        .canonicalize()
        .map_err(|_| "The transfer log is unavailable.")?;
    if !path.starts_with(&root) || !path.is_file() {
        return Err("The transfer log is unavailable.".into());
    }
    Command::new("explorer.exe")
        .arg(format!("/select,{}", path.display()))
        .spawn()
        .map_err(|_| "Unable to open the transfer log.")?;
    Ok(())
}

pub fn open_logs_folder(configuration: &config::AppConfiguration) -> Result<(), String> {
    let path = logs_dir(configuration);
    fs::create_dir_all(&path).map_err(|_| "File transfer logs are unavailable.")?;
    Command::new("explorer.exe")
        .arg(path)
        .spawn()
        .map_err(|_| "Unable to open the logs folder.")?;
    Ok(())
}

fn run_transfer(
    app: AppHandle,
    transfer: TransferConfig,
    analyze: bool,
    command: String,
    mut child: std::process::Child,
    stdout: Option<std::process::ChildStdout>,
    stderr: Option<std::process::ChildStderr>,
    progress_total_bytes: Option<u64>,
) {
    let started = OffsetDateTime::now_utc();
    let timer = Instant::now();
    let output = std::sync::Arc::new(Mutex::new(String::new()));
    let progress = (!analyze)
        .then(|| progress_total_bytes.filter(|value| *value > 0))
        .flatten()
        .map(ProgressTracker::new);
    let stdout_thread = stdout.map(|stream| {
        stream_output(app.clone(), stream, "stdout", output.clone(), progress.clone())
    });
    let stderr_thread =
        stderr.map(|stream| stream_output(app.clone(), stream, "stderr", output.clone(), None));
    let status = child.wait();
    if let Some(thread) = stdout_thread {
        let _ = thread.join();
    }
    if let Some(thread) = stderr_thread {
        let _ = thread.join();
    }
    if let Ok(mut active) = active_transfer().lock() {
        *active = None;
    }
    let raw_output = output.lock().map(|value| value.clone()).unwrap_or_default();
    let exit_code = status.ok().and_then(|value| value.code());
    let interpretation = interpret_exit_code(exit_code.unwrap_or(16));
    let summary = parse_summary(&raw_output);
    let duration_seconds = timer.elapsed().as_secs();
    let configuration = config::load().unwrap_or_default();
    let log_path = if transfer.save_log {
        write_log(
            &configuration,
            &transfer,
            &command,
            &raw_output,
            exit_code,
            duration_seconds,
        )
        .ok()
        .flatten()
    } else {
        None
    };
    let verification =
        if !analyze && transfer.verify_destination && interpretation.status != "error" {
            Some(run_verification(&transfer).unwrap_or_else(|_| "Verification failed".into()))
        } else {
            None
        };
    let state = if interpretation.status == "error" {
        "failed"
    } else if interpretation.status == "warning" {
        "completed-with-warning"
    } else {
        "completed"
    }
    .to_owned();
    let completed_at = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_default();
    let entry = TransferHistoryEntry {
        id: format!("{}-{}", started.unix_timestamp(), std::process::id()),
        started_at: started.format(&Rfc3339).unwrap_or_default(),
        completed_at,
        source: transfer.source.clone(),
        destination: transfer.destination.clone(),
        preset: transfer.mode,
        status: state.clone(),
        bytes_copied: summary.bytes_copied,
        files_copied: summary.files_copied,
        duration_seconds,
        exit_code,
        log_path: log_path.clone(),
    };
    if let Err(error) = record_history(&history_path(&configuration), entry) {
        tracing::warn!("file transfer history could not be recorded: {error}");
    }
    let _ = app.emit(
        "file-transfer-completed",
        CompletionEvent {
            phase: if analyze { "analysis" } else { "transfer" }.into(),
            state,
            exit_code,
            interpretation,
            summary,
            duration_seconds,
            log_path,
            verification,
        },
    );
    if transfer.shutdown_when_finished && !analyze && exit_code.is_some_and(|code| code < 8) {
        let _ = hidden_command("shutdown.exe")
            .args(["/s", "/t", "30"])
            .spawn();
    }
}

fn stream_output<R: std::io::Read + Send + 'static>(
    app: AppHandle,
    stream: R,
    name: &'static str,
    output: std::sync::Arc<Mutex<String>>,
    progress: Option<ProgressTracker>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        for line in BufReader::new(stream).lines().map_while(Result::ok) {
            let copied_bytes = progress.as_ref().and_then(|_| copied_file_bytes(&line));
            if let Ok(mut collected) = output.lock() {
                collected.push_str(&line);
                collected.push('\n');
            }
            let _ = app.emit(
                "file-transfer-output",
                OutputEvent {
                    line,
                    stream: name.into(),
                },
            );
            if let (Some(progress), Some(bytes)) = (&progress, copied_bytes) {
                let _ = app.emit("file-transfer-progress", progress.add(bytes));
            }
        }
    })
}

fn run_verification(config: &TransferConfig) -> Result<String, String> {
    let path = robocopy_path().ok_or("Robocopy is not available on this system.")?;
    let output = hidden_command(path)
        .args(command_arguments(config, true)?)
        .output()
        .map_err(|_| "Verification failed.".to_owned())?;
    match output.status.code().unwrap_or(16) {
        0 => Ok("Verified".into()),
        1..=7 => Ok("Differences found".into()),
        _ => Ok("Verification failed".into()),
    }
}

fn command_arguments(config: &TransferConfig, dry_run: bool) -> Result<Vec<String>, String> {
    validate_values(config)?;
    let mut args = vec![
        display_path(Path::new(&config.source)),
        display_path(Path::new(&config.destination)),
    ];
    if config.mode == TransferMode::Mirror {
        args.push("/MIR".into());
    } else if config.include_subfolders {
        args.push(
            if config.copy_empty_directories {
                "/E"
            } else {
                "/S"
            }
            .into(),
        );
    }
    if config.preserve_timestamps {
        // Keep times without inheriting source Hidden/System directory attributes.
        args.extend(["/COPY:DT".into(), "/DCOPY:T".into()]);
    }
    args.extend([
        format!("/R:{}", config.retries),
        format!("/W:{}", config.retry_wait),
        // Stable byte units let the guarded English per-file parser report real progress.
        "/BYTES".into(),
    ]);
    if config.skip_junction_points {
        args.push("/XJ".into());
    }
    if config.restartable {
        args.push("/Z".into());
    }
    if config.copy_empty_directories && !args.iter().any(|value| value == "/E" || value == "/MIR") {
        args.push("/E".into());
    }
    if config.mode != TransferMode::SimpleCopy {
        args.push(format!("/MT:{}", config.threads));
    }
    let (selection_folders, selection_files) = selection_exclusions(config)?;
    let mut excluded_folders = config.exclude_folders.clone();
    excluded_folders.extend(selection_folders);
    if config.mode == TransferMode::ProjectMigration {
        for folder in project_exclusions() {
            if !excluded_folders
                .iter()
                .any(|value| value.eq_ignore_ascii_case(folder))
            {
                excluded_folders.push((*folder).into());
            }
        }
    }
    if !excluded_folders.is_empty() {
        args.push("/XD".into());
        args.extend(excluded_folders);
    }
    let mut excluded_files = config.exclude_files.clone();
    excluded_files.extend(selection_files);
    if !excluded_files.is_empty() {
        args.push("/XF".into());
        args.extend(excluded_files);
    }
    if dry_run {
        args.push("/L".into());
    }
    Ok(args)
}

fn validate(config: &TransferConfig) -> Result<(), String> {
    validate_values(config)?;
    let source = Path::new(&config.source);
    if !source.is_dir() {
        return Err("Source folder does not exist or is not a directory.".into());
    }
    let destination = Path::new(&config.destination);
    if !destination.is_absolute() {
        return Err("Destination must be an absolute folder path.".into());
    }
    let source = source
        .canonicalize()
        .map_err(|_| "Source folder is unavailable.")?;
    if config.selection_enabled && config.selected_entries.is_empty() {
        return Err("Select at least one source item to transfer.".into());
    }
    if config.selection_enabled {
        selection_exclusions(config)?;
    }
    let destination = canonical_or_planned(destination)?;
    if source == destination {
        return Err("Source and destination must be different folders.".into());
    }
    if destination.starts_with(&source) {
        return Err("Destination cannot be inside the source folder.".into());
    }
    if config.mode == TransferMode::Mirror && source.starts_with(&destination) {
        return Err("Mirror source cannot be inside the destination folder.".into());
    }
    let warnings = location_warnings(config);
    if config.mode == TransferMode::Mirror && !config.mirror_confirmed {
        return Err(
            "Mirror mode requires explicit confirmation because it can delete destination data."
                .into(),
        );
    }
    if !warnings.is_empty() && !config.system_location_confirmed {
        return Err("System locations require explicit confirmation.".into());
    }
    if config.mode == TransferMode::Mirror
        && destination_has_data(&destination)
        && !config.destination_data_confirmed
    {
        return Err("Mirror mode requires a second confirmation because the destination already contains data.".into());
    }
    Ok(())
}

fn directory_entry(entry: fs::DirEntry) -> Result<DirectoryEntry, String> {
    let path = entry.path();
    let metadata = entry
        .metadata()
        .map_err(|_| "Unable to inspect a source item.")?;
    #[cfg(windows)]
    let attributes = {
        use std::os::windows::fs::MetadataExt;
        metadata.file_attributes()
    };
    #[cfg(not(windows))]
    let attributes = 0;
    Ok(DirectoryEntry {
        name: entry.file_name().to_string_lossy().into_owned(),
        path: display_path(&path),
        is_directory: metadata.is_dir(),
        is_hidden: attributes & 0x2 != 0,
        is_system: attributes & 0x4 != 0,
    })
}

fn directory_breadcrumbs(root: &Path) -> Vec<DirectoryBreadcrumb> {
    let mut paths = Vec::new();
    let mut current = Some(root);
    while let Some(path) = current {
        paths.push(path);
        current = path.parent();
    }
    paths.reverse();
    paths
        .into_iter()
        .map(|path| DirectoryBreadcrumb {
            label: path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| display_path(path)),
            path: display_path(path),
        })
        .collect()
}

#[derive(Clone)]
struct ProgressTracker {
    total_bytes: u64,
    copied_bytes: std::sync::Arc<Mutex<u64>>,
}

impl ProgressTracker {
    fn new(total_bytes: u64) -> Self {
        Self {
            total_bytes,
            copied_bytes: std::sync::Arc::new(Mutex::new(0)),
        }
    }

    fn add(&self, bytes: u64) -> ProgressEvent {
        let copied_bytes = self
            .copied_bytes
            .lock()
            .map(|mut value| {
                *value = value.saturating_add(bytes).min(self.total_bytes);
                *value
            })
            .unwrap_or(0);
        ProgressEvent {
            bytes_copied: copied_bytes,
            total_bytes: self.total_bytes,
            percent: ((copied_bytes.saturating_mul(100)) / self.total_bytes) as u8,
        }
    }
}

fn copied_file_bytes(line: &str) -> Option<u64> {
    ["New File", "Newer", "Older", "Changed"]
        .into_iter()
        .find_map(|marker| {
            line.split_once(marker).and_then(|(prefix, tail)| {
                let prefix = prefix.trim();
                (prefix.is_empty() || prefix.ends_with('%'))
                    .then(|| tail.split_whitespace().next().and_then(number))
                    .flatten()
            })
        })
}

fn selection_exclusions(config: &TransferConfig) -> Result<(Vec<String>, Vec<String>), String> {
    if !config.selection_enabled {
        return Ok((Vec::new(), Vec::new()));
    }
    if config.selected_entries.is_empty() {
        return Err("Select at least one source item to transfer.".into());
    }
    let selected = config
        .selected_entries
        .iter()
        .collect::<std::collections::HashSet<_>>();
    let source = Path::new(&config.source);
    let mut folders = Vec::new();
    let mut files = Vec::new();
    let mut found = std::collections::HashSet::new();
    for entry in fs::read_dir(source)
        .map_err(|_| "Unable to list source items for the selected transfer.")?
    {
        let entry = entry.map_err(|_| "Unable to list source items for the selected transfer.")?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.is_empty() || name.contains(['\\', '/', '\0']) {
            continue;
        }
        if selected.contains(&name) {
            found.insert(name);
            continue;
        }
        if entry
            .metadata()
            .map_err(|_| "Unable to inspect source items for the selected transfer.")?
            .is_dir()
        {
            folders.push(display_path(&entry.path()));
        } else {
            files.push(display_path(&entry.path()));
        }
    }
    if found.len() != selected.len() {
        return Err(
            "A selected source item no longer exists. Refresh the source list and select it again."
                .into(),
        );
    }
    Ok((folders, files))
}

fn validate_values(config: &TransferConfig) -> Result<(), String> {
    if config.source.trim().is_empty() || config.destination.trim().is_empty() {
        return Err("Choose both source and destination folders.".into());
    }
    if !matches!(config.threads, 1 | 2 | 4 | 8 | 16 | 32) {
        return Err("Threads must be 1, 2, 4, 8, 16, or 32.".into());
    }
    if config
        .exclude_folders
        .iter()
        .chain(&config.exclude_files)
        .any(|value| value.trim().is_empty() || value.contains('\0'))
    {
        return Err("Exclusions cannot be empty or contain a null character.".into());
    }
    Ok(())
}

fn canonical_or_planned(path: &Path) -> Result<PathBuf, String> {
    if path.exists() {
        return path
            .canonicalize()
            .map_err(|_| "Destination folder is unavailable.".into());
    }
    let mut missing = Vec::new();
    let mut current = path;
    while !current.exists() {
        let name = current
            .file_name()
            .ok_or("Destination folder is invalid.")?;
        missing.push(name.to_os_string());
        current = current.parent().ok_or("Destination folder is invalid.")?;
    }
    let mut result = current
        .canonicalize()
        .map_err(|_| "Destination folder is unavailable.")?;
    for part in missing.into_iter().rev() {
        result.push(part);
    }
    Ok(result)
}

fn display_path(path: &Path) -> String {
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

fn location_warnings(config: &TransferConfig) -> Vec<String> {
    [config.source.as_str(), config.destination.as_str()]
        .into_iter()
        .filter_map(|value| dangerous_location(value))
        .collect()
}
fn warnings(config: &TransferConfig) -> Vec<String> {
    let mut values = location_warnings(config);
    if config.mode == TransferMode::Mirror
        && Path::new(&config.destination).is_dir()
        && destination_has_data(Path::new(&config.destination))
    {
        values.push(
            "Mirror destination already contains data; a second confirmation is required.".into(),
        );
    }
    values
}
fn destination_has_data(path: &Path) -> bool {
    fs::read_dir(path)
        .ok()
        .and_then(|mut entries| entries.next())
        .is_some()
}

fn dangerous_location(value: &str) -> Option<String> {
    let normalized = value.trim_end_matches(['\\', '/']).to_ascii_lowercase();
    let system_root = std::env::var("SystemRoot")
        .unwrap_or_else(|_| "C:\\Windows".into())
        .to_ascii_lowercase();
    let program_files = std::env::var("ProgramFiles")
        .unwrap_or_else(|_| "C:\\Program Files".into())
        .to_ascii_lowercase();
    let program_data = std::env::var("ProgramData")
        .unwrap_or_else(|_| "C:\\ProgramData".into())
        .to_ascii_lowercase();
    let drive_root = normalized.len() == 2 && normalized.as_bytes().get(1) == Some(&b':');
    let protected = [system_root, program_files, program_data];
    (drive_root
        || protected.iter().any(|root| {
            normalized == **root
                || normalized
                    .strip_prefix(root)
                    .is_some_and(|tail| tail.starts_with('\\'))
        }))
    .then(|| format!("System-sensitive location: {value}"))
}

pub fn interpret_exit_code(code: i32) -> ExitInterpretation {
    match code {
        0 => ExitInterpretation {
            status: "success".into(),
            message: "No files needed copying.".into(),
        },
        1 => ExitInterpretation {
            status: "success".into(),
            message: "Files copied successfully.".into(),
        },
        2..=7 => ExitInterpretation {
            status: "warning".into(),
            message: "Robocopy completed with differences or skipped items.".into(),
        },
        8..=15 => ExitInterpretation {
            status: "error".into(),
            message: "Some files or directories could not be copied.".into(),
        },
        _ => ExitInterpretation {
            status: "error".into(),
            message: "Robocopy encountered a serious error.".into(),
        },
    }
}

pub fn parse_summary(output: &str) -> Summary {
    let values = |label: &str| {
        output
            .lines()
            .find(|line| line.trim_start().starts_with(label))
            .map(|line| {
                line.split(':')
                    .nth(1)
                    .unwrap_or("")
                    .split_whitespace()
                    .filter_map(number)
                    .collect::<Vec<_>>()
            })
    };
    let files = values("Files").unwrap_or_default();
    let bytes = values("Bytes").unwrap_or_default();
    Summary {
        files_copied: files.get(1).copied(),
        files_skipped: files.get(2).copied(),
        files_failed: files.get(4).copied(),
        bytes_copied: bytes.get(1).copied(),
    }
}

fn number(value: &str) -> Option<u64> {
    value.replace([',', '.'], "").parse().ok()
}
fn quote(value: &str) -> String {
    if !value.is_empty() && !value.contains([' ', '\t', '"']) {
        return value.into();
    }
    let mut quoted = String::from("\"");
    let mut slashes = 0;
    for character in value.chars() {
        if character == '\\' {
            slashes += 1;
        } else if character == '"' {
            quoted.push_str(&"\\".repeat(slashes * 2 + 1));
            quoted.push('"');
            slashes = 0;
        } else {
            quoted.push_str(&"\\".repeat(slashes));
            quoted.push(character);
            slashes = 0;
        }
    }
    quoted.push_str(&"\\".repeat(slashes * 2));
    quoted.push('"');
    quoted
}
fn display_command(arguments: &[String]) -> String {
    std::iter::once("robocopy".to_owned())
        .chain(arguments.iter().map(|value| quote(value)))
        .collect::<Vec<_>>()
        .join(" ")
}

fn hidden_command(program: impl AsRef<std::ffi::OsStr>) -> Command {
    let mut command = Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // CREATE_NO_WINDOW keeps console applications such as Robocopy invisible.
        command.creation_flags(0x0800_0000);
    }
    command
}

fn robocopy_path() -> Option<PathBuf> {
    let direct = std::env::var_os("SystemRoot")
        .map(PathBuf::from)
        .map(|root| root.join("System32").join("robocopy.exe"));
    direct.filter(|path| path.is_file()).or_else(|| {
        hidden_command("where.exe")
            .arg("robocopy.exe")
            .output()
            .ok()
            .filter(|result| result.status.success())
            .and_then(|result| String::from_utf8(result.stdout).ok())
            .and_then(|value| value.lines().next().map(PathBuf::from))
    })
}
fn logs_dir(configuration: &config::AppConfiguration) -> PathBuf {
    platform::config_dir(configuration)
        .join("logs")
        .join("file-transfer")
}
fn history_path(configuration: &config::AppConfiguration) -> PathBuf {
    platform::config_dir(configuration).join("file-transfer-history-v1.json")
}

fn write_log(
    configuration: &config::AppConfiguration,
    config: &TransferConfig,
    command: &str,
    output: &str,
    exit_code: Option<i32>,
    duration_seconds: u64,
) -> Result<Option<String>, String> {
    let directory = logs_dir(configuration);
    fs::create_dir_all(&directory).map_err(|_| "Unable to save transfer log.")?;
    let timestamp = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .map_err(|_| "Unable to timestamp transfer log.")?
        .replace(['-', ':', 'T', 'Z'], "")
        .chars()
        .take(14)
        .collect::<String>();
    let name = match config.mode {
        TransferMode::SimpleCopy => "simple-copy",
        TransferMode::FastCopy => "fast-copy",
        TransferMode::ProjectMigration => "project-migration",
        TransferMode::Mirror => "mirror",
    };
    let path = directory.join(format!("{timestamp}-{name}.log"));
    fs::write(&path, format!("source: {}\ndestination: {}\npreset: {:?}\ncommand: {}\nexit code: {:?}\nduration seconds: {}\n\n{}", config.source, config.destination, config.mode, command, exit_code, duration_seconds, output)).map_err(|_| "Unable to save transfer log.")?;
    Ok(Some(path.display().to_string()))
}
fn read_history(path: &Path) -> Result<Vec<TransferHistoryEntry>, String> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let metadata =
        fs::symlink_metadata(path).map_err(|_| "File transfer history is unavailable.")?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err("File transfer history is unavailable.".into());
    }
    let history: HistoryFile = serde_json::from_slice(
        &fs::read(path).map_err(|_| "File transfer history is unavailable.")?,
    )
    .map_err(|_| "File transfer history is unavailable.")?;
    if history.format_version != 1 {
        return Err("File transfer history uses an unsupported format.".into());
    }
    Ok(history
        .entries
        .into_iter()
        .take(MAX_HISTORY_ENTRIES)
        .collect())
}
fn record_history(path: &Path, entry: TransferHistoryEntry) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or("File transfer history is unavailable.")?;
    fs::create_dir_all(parent).map_err(|_| "File transfer history is unavailable.")?;
    let mut entries = read_history(path)?;
    entries.insert(0, entry);
    entries.truncate(MAX_HISTORY_ENTRIES);
    fs::write(
        path,
        serde_json::to_vec_pretty(&HistoryFile {
            format_version: 1,
            entries,
        })
        .map_err(|_| "File transfer history is unavailable.")?,
    )
    .map_err(|_| "File transfer history is unavailable.".into())
}

fn project_exclusions() -> &'static [&'static str] {
    &[
        "node_modules",
        "dist",
        "build",
        "coverage",
        ".cache",
        ".next",
        ".nuxt",
        ".vite",
        ".tmp",
        "temp",
        "bin",
        "obj",
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    fn config(mode: TransferMode) -> TransferConfig {
        TransferConfig {
            source: "C:\\Source Folder".into(),
            destination: "D:\\Destination Folder".into(),
            mode,
            include_subfolders: true,
            preserve_timestamps: true,
            skip_junction_points: true,
            restartable: false,
            copy_empty_directories: true,
            verify_destination: false,
            save_log: true,
            shutdown_when_finished: false,
            threads: 8,
            retries: 1,
            retry_wait: 1,
            exclude_folders: vec![],
            exclude_files: vec![],
            selection_enabled: false,
            selected_entries: vec![],
            mirror_confirmed: true,
            system_location_confirmed: true,
            destination_data_confirmed: true,
        }
    }
    #[test]
    fn command_builder_matches_presets() {
        let simple = command_arguments(&config(TransferMode::SimpleCopy), false).unwrap();
        assert!(simple.contains(&"/E".into()));
        assert!(!simple.iter().any(|value| value.starts_with("/MT")));
        let fast = command_arguments(&config(TransferMode::FastCopy), false).unwrap();
        assert!(fast.contains(&"/MT:8".into()));
        let mut project = config(TransferMode::ProjectMigration);
        project.exclude_folders = vec!["node_modules".into(), "dist folder".into()];
        let project_args = command_arguments(&project, false).unwrap();
        assert!(project_args
            .windows(2)
            .any(|values| values == ["/XD", "node_modules"]));
        assert!(display_command(&project_args).contains("\"dist folder\""));
        assert!(command_arguments(&project, false)
            .unwrap()
            .contains(&".next".into()));
        assert!(command_arguments(&config(TransferMode::Mirror), false)
            .unwrap()
            .contains(&"/MIR".into()));
    }
    #[test]
    fn preserve_timestamps_does_not_copy_hidden_or_system_attributes() {
        let arguments = command_arguments(&config(TransferMode::FastCopy), false).unwrap();
        assert!(arguments.contains(&"/COPY:DT".into()));
        assert!(arguments.contains(&"/DCOPY:T".into()));
        assert!(!arguments.iter().any(|argument| argument.contains("COPY:DAT")));
    }
    #[test]
    fn exit_codes_are_not_generic_failures() {
        for code in [0, 1] {
            assert_eq!(interpret_exit_code(code).status, "success");
        }
        for code in [2, 3, 7] {
            assert_eq!(interpret_exit_code(code).status, "warning");
        }
        for code in [8, 16] {
            assert_eq!(interpret_exit_code(code).status, "error");
        }
    }
    #[test]
    fn parser_only_reports_present_summary_values() {
        let parsed = parse_summary(" Files : 10 4 5 0 1 0\n Bytes : 100 40 50 0 10 0");
        assert_eq!(parsed.files_copied, Some(4));
        assert_eq!(parsed.files_failed, Some(1));
        assert_eq!(parsed.bytes_copied, Some(40));
        assert_eq!(parse_summary("localized output").files_copied, None);
    }
    #[test]
    fn progress_parser_accepts_only_completed_english_file_lines() {
        assert_eq!(copied_file_bytes("100% New File 1048576 report.zip"), Some(1_048_576));
        assert_eq!(copied_file_bytes("Newer 42 update.txt"), Some(42));
        assert_eq!(copied_file_bytes("report named New File 99.txt"), None);
    }
    #[test]
    fn rejects_invalid_equal_and_nested_paths() {
        let mut invalid = config(TransferMode::SimpleCopy);
        invalid.source = "C:\\missing".into();
        assert!(validate(&invalid).is_err());
        let root = std::env::temp_dir().join(format!("file-transfer-{}", std::process::id()));
        let source = root.join("source");
        fs::create_dir_all(source.join("destination")).unwrap();
        let mut equal = config(TransferMode::SimpleCopy);
        equal.source = source.display().to_string();
        equal.destination = equal.source.clone();
        assert!(validate(&equal).is_err());
        let mut nested = config(TransferMode::Mirror);
        nested.source = source.display().to_string();
        nested.destination = source.join("destination").display().to_string();
        assert!(validate(&nested).is_err());
        let _ = fs::remove_dir_all(root);
    }
    #[test]
    fn selected_items_exclude_every_unselected_sibling() {
        let root =
            std::env::temp_dir().join(format!("file-transfer-selection-{}", std::process::id()));
        fs::create_dir_all(root.join("include-me")).unwrap();
        fs::write(root.join("skip-me.txt"), "skip").unwrap();
        let mut selected = config(TransferMode::FastCopy);
        selected.source = root.display().to_string();
        selected.selection_enabled = true;
        selected.selected_entries = vec!["include-me".into()];
        let args = command_arguments(&selected, false).unwrap();
        assert!(args.windows(2).any(|values| values
            == [
                "/XF",
                root.join("skip-me.txt").display().to_string().as_str()
            ]));
        assert!(!args
            .iter()
            .any(|value| value.contains("include-me") && value.starts_with("/")));
        let _ = fs::remove_dir_all(root);
    }
    #[test]
    fn command_arguments_remove_windows_extended_path_prefixes() {
        let mut prefixed = config(TransferMode::FastCopy);
        prefixed.source = r"\\?\C:\Source Folder".into();
        prefixed.destination = r"\\?\D:\Destination Folder".into();
        let args = command_arguments(&prefixed, false).unwrap();
        assert_eq!(args[0], r"C:\Source Folder");
        assert_eq!(args[1], r"D:\Destination Folder");
    }
}
