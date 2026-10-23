//! Windows-first Codex environment management. Authentication remains entirely in Codex CLI state.
use crate::{
    config::{self, AppConfiguration},
    fs_safety,
};
use serde::{Deserialize, Serialize};
use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

const BEGIN: &str = "<!-- BEGIN DEV-COMPANION CODEX MANAGED RULES -->";
const END: &str = "<!-- END DEV-COMPANION CODEX MANAGED RULES -->";
pub const DEFAULT_INSTRUCTIONS: &str = "## Dev Companion managed rules\n\n- Respond professionally and concisely. In Vietnamese, refer to yourself as \"Em\" and address the user as \"Anh\".\n- Ask when important information is missing; never invent project requirements.\n- Inspect project documentation and source before substantial changes, including repository AGENTS.md.\n- Prefer small, safe, maintainable changes; avoid unrelated refactoring.\n- Respect the project language and framework conventions; preserve TypeScript strict typing and do not hide errors with unsafe casts or blanket suppressions.\n- Run relevant lint, type-check, tests and build when practical; never claim a check passed unless it ran successfully.\n- Preserve unrelated local changes. Do not commit, push, reset, rebase or rewrite history unless explicitly requested.\n- Summarize changes, relevant files, validation, and remaining risks when completing significant work.";

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Environment {
    pub id: String,
    pub display_name: String,
    pub command_alias: String,
    pub codex_home: String,
    pub description: Option<String>,
    #[serde(default)]
    pub manages_instructions: bool,
    pub launcher_path: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentInput {
    pub id: Option<String>,
    pub display_name: String,
    pub command_alias: String,
    pub codex_home: String,
    pub description: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CliStatus {
    pub installed: bool,
    pub path: Option<String>,
    pub version: Option<String>,
    pub error: Option<String>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentStatus {
    #[serde(flatten)]
    pub environment: Environment,
    pub is_default: bool,
    pub codex_home_exists: bool,
    pub agents_exists: bool,
    pub login_status: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Overview {
    pub cli: CliStatus,
    pub launcher_dir: String,
    pub launcher_dir_exists: bool,
    pub launcher_dir_in_user_path: bool,
    pub environments: Vec<EnvironmentStatus>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionResult {
    pub message: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Instructions {
    pub content: String,
    pub managed: bool,
}

fn now() -> Result<String, String> {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .map_err(|_| "Could not timestamp environment metadata.".into())
}
fn home_dir() -> Result<PathBuf, String> {
    dirs::home_dir().ok_or("Could not resolve the current user home directory.".into())
}
fn launcher_dir() -> Result<PathBuf, String> {
    Ok(home_dir()?.join("bin"))
}
fn default_home() -> Result<PathBuf, String> {
    Ok(home_dir()?.join(".codex"))
}
fn display(path: &Path) -> String {
    path.to_string_lossy().to_string()
}
fn background_command(program: impl AsRef<std::ffi::OsStr>) -> Command {
    let mut command = Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    command
}

fn expand_user_home(value: &str) -> Result<PathBuf, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.contains('\0') {
        return Err("CODEX_HOME is required.".into());
    }
    let home = home_dir()?;
    let expanded = if trimmed.eq_ignore_ascii_case("%USERPROFILE%") {
        home
    } else if trimmed
        .get(..13)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("%USERPROFILE%"))
        && trimmed
            .as_bytes()
            .get(13)
            .is_some_and(|byte| matches!(byte, b'\\' | b'/'))
    {
        home.join(&trimmed[14..])
    } else {
        PathBuf::from(trimmed)
    };
    if !expanded.is_absolute()
        || expanded
            .components()
            .any(|part| matches!(part, std::path::Component::ParentDir))
    {
        return Err("CODEX_HOME must be an absolute path without '..'.".into());
    }
    Ok(expanded)
}

fn normal(path: &Path) -> String {
    path.to_string_lossy()
        .trim_end_matches(['\\', '/'])
        .to_ascii_lowercase()
}
fn validate_alias(value: &str) -> Result<(), String> {
    let bytes = value.as_bytes();
    if value.eq_ignore_ascii_case("codex")
        || value.len() < 3
        || value.len() > 64
        || !bytes.first().is_some_and(u8::is_ascii_alphabetic)
        || !bytes
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err("Command alias must be 3-64 letters, numbers, hyphens, or underscores and start with a letter.".into());
    }
    Ok(())
}
fn validate_home(path: &Path) -> Result<(), String> {
    let home = home_dir()?;
    let value = normal(path);
    let root = path.ancestors().last().unwrap_or(path);
    let blocked =
        ["Windows", "Program Files", "Program Files (x86)"].map(|name| normal(&root.join(name)));
    if path == home
        || path.parent().is_none()
        || blocked
            .iter()
            .any(|prefix| value == *prefix || value.starts_with(&format!("{prefix}\\")))
    {
        return Err(
            "CODEX_HOME cannot be a user home, drive root, Windows, or Program Files path.".into(),
        );
    }
    fs_safety::check(path)
}
fn canonical_text(path: PathBuf) -> Result<String, String> {
    validate_home(&path)?;
    Ok(display(&path))
}
fn default_environment() -> Result<Environment, String> {
    let home = default_home()?;
    Ok(Environment {
        id: "default".into(),
        display_name: "Personal / Default".into(),
        command_alias: "codex".into(),
        codex_home: display(&home),
        description: Some("Externally managed default Codex environment.".into()),
        manages_instructions: false,
        launcher_path: None,
        created_at: String::new(),
        updated_at: String::new(),
    })
}

fn effective_path() -> String {
    let mut value = String::new();
    #[cfg(windows)]
    for scope in ["Machine", "User"] {
        if let Ok(saved) = registry_path(scope) {
            if !saved.is_empty() {
                value.push(';');
                value.push_str(&expand_user_path(&saved));
            }
        }
    }
    value.push(';');
    value.push_str(&env::var("PATH").unwrap_or_default());
    value
}

fn resolve_command(alias: &str, path: &str) -> Option<PathBuf> {
    let extensions = env::var("PATHEXT").unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".into());
    path.split(';')
        .map(|part| PathBuf::from(expand_user_path(part.trim().trim_matches('"'))))
        .filter(|directory| directory.is_absolute())
        .flat_map(|directory| {
            extensions
                .split(';')
                .filter(|extension| {
                    [".exe", ".cmd", ".bat", ".com"]
                        .iter()
                        .any(|supported| extension.eq_ignore_ascii_case(supported))
                })
                .map(move |extension| {
                    directory.join(format!("{alias}{}", extension.to_ascii_lowercase()))
                })
        })
        .find(|candidate| candidate.is_file())
}

fn batch_value(value: &str) -> Result<(), String> {
    if value.contains(['"', '%', '!', '\r', '\n', '\0']) {
        return Err("Launcher paths cannot contain quotes, %, !, or control characters.".into());
    }
    Ok(())
}

fn cli_command(path: &Path, args: &[&str]) -> Result<Command, String> {
    #[cfg(windows)]
    if path.extension().is_some_and(|extension| {
        extension.eq_ignore_ascii_case("cmd") || extension.eq_ignore_ascii_case("bat")
    }) {
        use std::os::windows::process::CommandExt;
        batch_value(&display(path))?;
        let mut command = background_command("cmd.exe");
        // Pass the path through the environment so CMD receives one quoted batch target.
        command.env("DEV_COMPANION_CODEX_CLI", path);
        command.raw_arg(format!(
            "/d /v:off /s /c \"\"%DEV_COMPANION_CODEX_CLI%\" {}\"",
            args.join(" ")
        ));
        return Ok(command);
    }
    let mut command = background_command(path);
    command.args(args);
    Ok(command)
}

fn probe_version(path: &Path, search_path: &str, home: Option<&Path>) -> Result<String, String> {
    let mut command = cli_command(path, &["--version"])?;
    command.env("PATH", search_path);
    if let Some(home) = home {
        command.env("CODEX_HOME", home);
    }
    let output = command
        .output()
        .map_err(|_| "Could not execute Codex CLI. Check Node.js and the CLI installation.")?;
    let version = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    if !output.status.success() || !version.starts_with("codex-cli ") {
        return Err(
            "Codex CLI was found but --version failed. Check Node.js and the CLI installation."
                .into(),
        );
    }
    Ok(version)
}

fn cli_status(search_path: &str) -> CliStatus {
    let path = resolve_command("codex", search_path);
    let result = path
        .as_deref()
        .map(|path| probe_version(path, search_path, None));
    let (version, error) = match result {
        Some(Ok(version)) => (Some(version), None),
        Some(Err(error)) => (None, Some(error)),
        None => (None, None),
    };
    CliStatus {
        installed: path.is_some(),
        path: path.as_deref().map(display),
        version,
        error,
    }
}
fn login_status(home: &Path, cli: &CliStatus, search_path: &str) -> String {
    if cli.version.is_none() {
        return "unknown".into();
    }
    let Ok(mut command) = cli_command(
        Path::new(cli.path.as_deref().unwrap()),
        &["login", "status"],
    ) else {
        return "unknown".into();
    };
    match command
        .env("PATH", search_path)
        .env("CODEX_HOME", home)
        .output()
    {
        Ok(output) if output.status.success() => "loggedIn".into(),
        Ok(output)
            if String::from_utf8_lossy(&output.stdout)
                .to_ascii_lowercase()
                .contains("not logged in")
                || String::from_utf8_lossy(&output.stderr)
                    .to_ascii_lowercase()
                    .contains("not logged in") =>
        {
            "notLoggedIn".into()
        }
        _ => "unknown".into(),
    }
}
fn status(
    environment: Environment,
    is_default: bool,
    cli: &CliStatus,
    search_path: &str,
) -> EnvironmentStatus {
    let home = PathBuf::from(&environment.codex_home);
    EnvironmentStatus {
        codex_home_exists: home.is_dir(),
        agents_exists: home.join("AGENTS.md").is_file(),
        login_status: login_status(&home, cli, search_path),
        environment,
        is_default,
    }
}

pub fn overview(configuration: &AppConfiguration) -> Result<Overview, String> {
    let search_path = effective_path();
    let cli = cli_status(&search_path);
    let default = default_environment()?;
    let mut environments = vec![status(default.clone(), true, &cli, &search_path)];
    environments.extend(
        configuration
            .codex_environments
            .iter()
            .cloned()
            .map(|environment| status(environment, false, &cli, &search_path)),
    );
    let directory = launcher_dir()?;
    let user_path = user_path().unwrap_or_default();
    Ok(Overview {
        launcher_dir: display(&directory),
        launcher_dir_exists: directory.is_dir(),
        launcher_dir_in_user_path: path_contains(&user_path, &directory),
        cli,
        environments,
    })
}

fn save_environment(
    configuration: &mut AppConfiguration,
    input: EnvironmentInput,
) -> Result<Environment, String> {
    validate_alias(input.command_alias.trim())?;
    let home = expand_user_home(&input.codex_home)?;
    let codex_home = canonical_text(home.clone())?;
    let name = input.display_name.trim();
    if name.is_empty() || name.len() > 80 || name.contains(['\r', '\n', '\0']) {
        return Err("Display name must be 1-80 printable characters.".into());
    }
    let description = input
        .description
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned);
    if description
        .as_deref()
        .is_some_and(|value| value.len() > 500 || value.contains('\0'))
    {
        return Err("Description must be at most 500 characters.".into());
    }
    let default = default_environment()?;
    if normal(&home) == normal(Path::new(&default.codex_home)) {
        return Err("The default Codex environment is externally managed and cannot be added as a custom environment.".into());
    }
    let alias = input.command_alias.trim().to_owned();
    let id = input
        .id
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| format!("env-{}", OffsetDateTime::now_utc().unix_timestamp_nanos()));
    if id == "default" || id.contains(['\\', '/', '\0']) {
        return Err("Invalid environment identifier.".into());
    }
    if configuration.codex_environments.iter().any(|item| {
        item.id != id
            && (item.command_alias.eq_ignore_ascii_case(&alias)
                || normal(Path::new(&item.codex_home)) == normal(&home))
    }) {
        return Err("Command alias and CODEX_HOME must be unique.".into());
    }
    fs::create_dir_all(&home).map_err(|_| "Could not create CODEX_HOME.")?;
    fs_safety::check(&home)?;
    let stamp = now()?;
    let environment = if let Some(index) = configuration
        .codex_environments
        .iter()
        .position(|item| item.id == id)
    {
        let existing = &configuration.codex_environments[index];
        Environment {
            id,
            display_name: name.to_owned(),
            command_alias: alias,
            codex_home,
            description,
            manages_instructions: existing.manages_instructions,
            launcher_path: existing.launcher_path.clone(),
            created_at: existing.created_at.clone(),
            updated_at: stamp,
        }
    } else {
        Environment {
            id,
            display_name: name.to_owned(),
            command_alias: alias,
            codex_home,
            description,
            manages_instructions: false,
            launcher_path: None,
            created_at: stamp.clone(),
            updated_at: stamp,
        }
    };
    if let Some(index) = configuration
        .codex_environments
        .iter()
        .position(|item| item.id == environment.id)
    {
        configuration.codex_environments[index] = environment.clone();
    } else {
        configuration.codex_environments.push(environment.clone());
    }
    Ok(environment)
}
pub fn create_or_update(input: EnvironmentInput) -> Result<Environment, String> {
    let mut configuration = config::load().map_err(|error| error.to_string())?;
    let environment = save_environment(&mut configuration, input)?;
    config::save(&configuration).map_err(|error| error.to_string())?;
    Ok(environment)
}
fn configured(id: &str) -> Result<(AppConfiguration, usize), String> {
    let configuration = config::load().map_err(|error| error.to_string())?;
    let index = configuration
        .codex_environments
        .iter()
        .position(|item| item.id == id)
        .ok_or("Codex environment was not found.")?;
    Ok((configuration, index))
}
fn environment(id: &str) -> Result<Environment, String> {
    if id == "default" {
        default_environment()
    } else {
        let (configuration, index) = configured(id)?;
        Ok(configuration.codex_environments[index].clone())
    }
}

fn launcher_content(home: &str, cli: &str) -> Result<String, String> {
    batch_value(home)?;
    batch_value(cli)?;
    let directory = Path::new(cli).parent().ok_or("Invalid Codex CLI path.")?;
    Ok(format!("@echo off\r\nsetlocal DisableDelayedExpansion\r\nset \"CODEX_HOME={home}\"\r\nset \"PATH={};%PATH%\"\r\ncall \"{cli}\" %*\r\nset \"EXIT_CODE=%ERRORLEVEL%\"\r\nendlocal & exit /b %EXIT_CODE%\r\n", display(directory)))
}
pub fn regenerate_launcher(id: &str) -> Result<ActionResult, String> {
    let (mut configuration, index) = configured(id)?;
    let environment = configuration.codex_environments[index].clone();
    validate_alias(&environment.command_alias)?;
    let cli = cli_status(&effective_path());
    let cli_path = cli
        .path
        .as_deref()
        .ok_or("Codex CLI was not found in PATH. Install it before creating a launcher.")?;
    if let Some(error) = cli.error {
        return Err(error);
    }
    let content = launcher_content(&environment.codex_home, cli_path)?;
    let directory = launcher_dir()?;
    fs_safety::check(&directory)?;
    fs::create_dir_all(&directory).map_err(|_| "Could not create the launcher directory.")?;
    let path = directory.join(format!("{}.cmd", environment.command_alias));
    fs_safety::check(&path)?;
    if path.exists()
        && environment.launcher_path.as_deref() != Some(&display(&path))
        && fs::read_to_string(&path).ok().as_deref() != Some(&content)
    {
        return Err("A different launcher already uses this alias; choose another alias or remove it manually.".into());
    }
    fs::write(&path, content).map_err(|_| "Could not write the launcher.")?;
    configuration.codex_environments[index].launcher_path = Some(display(&path));
    configuration.codex_environments[index].updated_at = now()?;
    config::save(&configuration).map_err(|error| error.to_string())?;
    let path_result = add_launcher_dir_to_user_path()
        .map_err(|error| format!("Launcher created, but PATH setup failed: {error}"))?;
    Ok(ActionResult {
        message: format!(
            "Launcher created: {}. {} Restart terminals opened before this change.",
            display(&path),
            path_result.message
        ),
    })
}
pub fn remove_launcher(id: &str, confirmation: &str) -> Result<ActionResult, String> {
    if confirmation != "REMOVE" {
        return Err("Type REMOVE to remove the launcher.".into());
    }
    let (mut configuration, index) = configured(id)?;
    let Some(value) = configuration.codex_environments[index]
        .launcher_path
        .clone()
    else {
        return Ok(ActionResult {
            message: "No managed launcher exists.".into(),
        });
    };
    let directory = launcher_dir()?;
    let path = PathBuf::from(value);
    if path.parent() != Some(directory.as_path())
        || path
            .extension()
            .is_none_or(|extension| !extension.eq_ignore_ascii_case("cmd"))
    {
        return Err("Refusing to remove a launcher outside the managed launcher directory.".into());
    }
    if path.exists() {
        fs_safety::check(&path)?;
        fs::remove_file(&path).map_err(|_| "Could not remove the launcher.")?;
    }
    configuration.codex_environments[index].launcher_path = None;
    configuration.codex_environments[index].updated_at = now()?;
    config::save(&configuration).map_err(|error| error.to_string())?;
    Ok(ActionResult {
        message: "Launcher removed; CODEX_HOME was not changed.".into(),
    })
}

fn user_path() -> Result<String, String> {
    #[cfg(windows)]
    {
        registry_path("User")
    }
    #[cfg(not(windows))]
    {
        Ok(env::var("PATH").unwrap_or_default())
    }
}
#[cfg(windows)]
fn registry_path(scope: &str) -> Result<String, String> {
    let script = format!("$ErrorActionPreference='Stop'; [Console]::OutputEncoding=[Text.UTF8Encoding]::new(); [Environment]::GetEnvironmentVariable('Path','{scope}')");
    let output = background_command("powershell.exe")
        .args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            &script,
        ])
        .output()
        .map_err(|_| "Could not read the saved PATH.")?;
    if !output.status.success() {
        return Err("Could not read the saved PATH; no PATH changes were made.".into());
    }
    String::from_utf8(output.stdout)
        .map(|value| value.trim().to_owned())
        .map_err(|_| "Could not decode the saved PATH.".into())
}
fn expand_user_path(value: &str) -> String {
    let mut expanded = String::new();
    let mut remaining = value;
    while let Some(start) = remaining.find('%') {
        expanded.push_str(&remaining[..start]);
        let Some(end) = remaining[start + 1..].find('%') else {
            expanded.push_str(&remaining[start..]);
            return expanded;
        };
        let end = start + 1 + end;
        let name = &remaining[start + 1..end];
        expanded.push_str(
            &env::vars()
                .find(|(key, _)| key.eq_ignore_ascii_case(name))
                .map(|(_, value)| value)
                .unwrap_or_else(|| remaining[start..=end].into()),
        );
        remaining = &remaining[end + 1..];
    }
    expanded.push_str(remaining);
    expanded
}
fn path_contains(value: &str, directory: &Path) -> bool {
    let expected = normal(directory);
    value
        .split(';')
        .map(|part| {
            normal(Path::new(
                expand_user_path(part.trim().trim_matches('"')).as_str(),
            ))
        })
        .any(|part| part == expected)
}
pub fn add_launcher_dir_to_user_path() -> Result<ActionResult, String> {
    let directory = launcher_dir()?;
    fs_safety::check(&directory)?;
    fs::create_dir_all(&directory).map_err(|_| "Could not create the launcher directory.")?;
    let current = user_path()?;
    if path_contains(&current, &directory) {
        return Ok(ActionResult {
            message: "Launcher directory is already in the User PATH.".into(),
        });
    }
    #[cfg(windows)]
    {
        let next = if current.is_empty() {
            display(&directory)
        } else {
            format!("{};{}", current.trim_end_matches(';'), display(&directory))
        };
        let status = background_command("reg.exe")
            .args([
                "add",
                "HKCU\\Environment",
                "/v",
                "Path",
                "/t",
                "REG_EXPAND_SZ",
                "/d",
                &next,
                "/f",
            ])
            .status()
            .map_err(|_| "Could not update the User PATH.")?;
        if !status.success() {
            return Err("Could not update the User PATH.".into());
        }
        // Notify Explorer so newly opened terminals inherit the saved User PATH.
        #[link(name = "user32")]
        extern "system" {
            fn SendMessageTimeoutW(
                hwnd: isize,
                msg: u32,
                wparam: usize,
                lparam: isize,
                flags: u32,
                timeout: u32,
                result: *mut usize,
            ) -> isize;
        }
        let setting: Vec<u16> = "Environment\0".encode_utf16().collect();
        unsafe {
            SendMessageTimeoutW(
                0xffff,
                0x001a,
                0,
                setting.as_ptr() as isize,
                2,
                1000,
                std::ptr::null_mut(),
            );
        }
        Ok(ActionResult {
            message: "Added the launcher directory to the User PATH. Restart terminals to use it."
                .into(),
        })
    }
    #[cfg(not(windows))]
    {
        let _ = current;
        Err("Updating PATH is supported on Windows only.".into())
    }
}

pub fn check_alias(id: &str) -> Result<ActionResult, String> {
    let environment = environment(id)?;
    check_environment_alias(&environment, &effective_path(), &launcher_dir()?)
}

fn check_environment_alias(
    environment: &Environment,
    search_path: &str,
    directory: &Path,
) -> Result<ActionResult, String> {
    if !environment.id.eq("default") {
        validate_alias(&environment.command_alias)?;
    }
    let resolved = resolve_command(&environment.command_alias, search_path).ok_or(
        "Alias was not found in PATH. Regenerate the launcher, then restart existing terminals.",
    )?;
    if !environment.id.eq("default") {
        let expected = directory.join(format!("{}.cmd", environment.command_alias));
        if normal(&resolved) != normal(&expected) {
            return Err(
                "Another command shadows this alias in PATH. Choose a different alias.".into(),
            );
        }
        fs_safety::check(&resolved)?;
        let cli = cli_status(search_path);
        let content = launcher_content(
            &environment.codex_home,
            cli.path
                .as_deref()
                .ok_or("Codex CLI was not found in PATH.")?,
        )?;
        if fs::read_to_string(&resolved).ok().as_deref() != Some(&content) {
            return Err(
                "Launcher is outdated or changed. Regenerate it before checking the alias.".into(),
            );
        }
    }
    let version = probe_version(
        &resolved,
        search_path,
        Some(Path::new(&environment.codex_home)),
    )?;
    Ok(ActionResult { message: format!("Alias {} works: {version}. Checked using updated PATH; restart terminals opened before PATH changes.", environment.command_alias) })
}

pub fn instructions(id: &str) -> Result<Instructions, String> {
    let environment = environment(id)?;
    let path = Path::new(&environment.codex_home).join("AGENTS.md");
    if !path.exists() {
        return Ok(Instructions {
            content: DEFAULT_INSTRUCTIONS.into(),
            managed: false,
        });
    }
    fs_safety::check(&path)?;
    let existing =
        fs::read_to_string(path).map_err(|_| "Could not read AGENTS.md as UTF-8 text.")?;
    match managed_content(&existing)? {
        Some(content) => Ok(Instructions {
            content,
            managed: true,
        }),
        None => Ok(Instructions {
            content: DEFAULT_INSTRUCTIONS.into(),
            managed: false,
        }),
    }
}
fn managed_content(existing: &str) -> Result<Option<String>, String> {
    let starts: Vec<_> = existing.match_indices(BEGIN).collect();
    let ends: Vec<_> = existing.match_indices(END).collect();
    if starts.is_empty() && ends.is_empty() {
        return Ok(None);
    }
    if starts.len() != 1 || ends.len() != 1 || starts[0].0 > ends[0].0 {
        return Err(
            "AGENTS.md has missing or corrupted Dev Companion markers; it was not changed.".into(),
        );
    }
    Ok(Some(
        existing[starts[0].0 + BEGIN.len()..ends[0].0]
            .trim_matches(['\r', '\n'])
            .to_owned(),
    ))
}
fn replace_managed_block(existing: &str, content: &str) -> Result<String, String> {
    let block = format!("{BEGIN}\n{content}\n{END}");
    let starts: Vec<_> = existing.match_indices(BEGIN).collect();
    let ends: Vec<_> = existing.match_indices(END).collect();
    if starts.is_empty() && ends.is_empty() {
        return Ok(if existing.trim().is_empty() {
            format!("{block}\n")
        } else {
            format!("{}\n\n{block}\n", existing.trim_end())
        });
    }
    if starts.len() != 1 || ends.len() != 1 || starts[0].0 > ends[0].0 {
        return Err(
            "AGENTS.md has missing or corrupted Dev Companion markers; it was not changed.".into(),
        );
    }
    Ok(format!(
        "{}{}{}",
        &existing[..starts[0].0],
        block,
        &existing[ends[0].0 + END.len()..]
    ))
}
fn backup(path: &Path) -> Result<(), String> {
    let stamp = OffsetDateTime::now_utc().unix_timestamp_nanos();
    let backup = path.with_file_name(format!("AGENTS.md.dev-companion-{stamp}.bak"));
    fs::copy(path, backup).map_err(|_| "Could not back up AGENTS.md before updating it.")?;
    Ok(())
}
pub fn update_instructions(id: &str, content: &str) -> Result<ActionResult, String> {
    if content.trim().is_empty()
        || content.len() > 20_000
        || content.contains('\0')
        || content.contains(BEGIN)
        || content.contains(END)
    {
        return Err(
            "Managed instructions must be non-empty text without Dev Companion markers.".into(),
        );
    }
    let (mut configuration, index) = configured(id)?;
    let path = Path::new(&configuration.codex_environments[index].codex_home).join("AGENTS.md");
    fs_safety::check(path.parent().ok_or("Invalid CODEX_HOME.")?)?;
    fs::create_dir_all(path.parent().unwrap()).map_err(|_| "Could not create CODEX_HOME.")?;
    let existed = path.exists();
    let existing = if existed {
        fs_safety::check(&path)?;
        fs::read_to_string(&path).map_err(|_| "Could not read AGENTS.md as UTF-8 text.")?
    } else {
        String::new()
    };
    let updated = replace_managed_block(&existing, content.trim())?;
    if existed {
        backup(&path)?;
    }
    fs::write(&path, updated).map_err(|_| "Could not update AGENTS.md.")?;
    configuration.codex_environments[index].manages_instructions = true;
    configuration.codex_environments[index].updated_at = now()?;
    config::save(&configuration).map_err(|error| error.to_string())?;
    Ok(ActionResult {
        message: "Managed AGENTS.md rules saved; content outside the markers was preserved.".into(),
    })
}

pub fn delete_environment(id: &str, confirmation: &str) -> Result<ActionResult, String> {
    if confirmation != "DELETE" {
        return Err("Type DELETE to remove the environment entry.".into());
    }
    let (mut configuration, _) = configured(id)?;
    remove_entry(&mut configuration, id)?;
    config::save(&configuration).map_err(|error| error.to_string())?;
    Ok(ActionResult { message: "Environment entry removed. Its CODEX_HOME, AGENTS.md, launcher, and Codex credentials were not deleted.".into() })
}
fn remove_entry(configuration: &mut AppConfiguration, id: &str) -> Result<Environment, String> {
    let index = configuration
        .codex_environments
        .iter()
        .position(|item| item.id == id)
        .ok_or("Codex environment was not found.")?;
    Ok(configuration.codex_environments.remove(index))
}
fn open(path: &Path, editor: bool) -> Result<ActionResult, String> {
    #[cfg(windows)]
    {
        let mut command = if editor {
            let mut command = Command::new("notepad.exe");
            command.arg(path);
            command
        } else {
            let mut command = Command::new("explorer.exe");
            command.arg(path);
            command
        };
        command
            .spawn()
            .map_err(|_| "Could not open the requested location.")?;
        Ok(ActionResult {
            message: "Opened the requested location.".into(),
        })
    }
    #[cfg(not(windows))]
    {
        let _ = (path, editor);
        Err("This action is supported on Windows only.".into())
    }
}
pub fn open_home(id: &str) -> Result<ActionResult, String> {
    let environment = environment(id)?;
    let path = PathBuf::from(environment.codex_home);
    fs::create_dir_all(&path).map_err(|_| "Could not create CODEX_HOME.")?;
    fs_safety::check(&path)?;
    open(&path, false)
}
pub fn open_agents(id: &str) -> Result<ActionResult, String> {
    let environment = environment(id)?;
    let path = PathBuf::from(environment.codex_home).join("AGENTS.md");
    if !path.exists() {
        return Err("AGENTS.md does not exist yet. Save managed instructions first.")?;
    }
    fs_safety::check(&path)?;
    open(&path, true)
}
pub fn run_cli(id: &str, action: &str) -> Result<ActionResult, String> {
    let environment = environment(id)?;
    let search_path = effective_path();
    let cli = cli_status(&search_path);
    let path = cli
        .path
        .as_deref()
        .ok_or("Codex CLI was not found in PATH.")?;
    if let Some(error) = cli.error {
        return Err(error);
    }
    let args: &[&str] = match action {
        "launch" => &[],
        "login" => &["login"],
        "logout" => &["logout"],
        _ => return Err("Unsupported Codex action.".into()),
    };
    let mut command = cli_command(Path::new(path), args)?;
    command
        .env("PATH", &search_path)
        .env("CODEX_HOME", &environment.codex_home);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x00000010);
    }
    command.spawn().map_err(|_| "Could not start Codex CLI.")?;
    Ok(ActionResult {
        message: "Codex started in a separate terminal using this environment.".into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn resolver_skips_extensionless_shell_shims_and_obeys_path_order() {
        let root = env::temp_dir().join(format!(
            "companion-resolver-{}",
            OffsetDateTime::now_utc().unix_timestamp_nanos()
        ));
        let first = root.join("first");
        let second = root.join("second");
        fs::create_dir_all(&first).unwrap();
        fs::create_dir_all(&second).unwrap();
        fs::write(first.join("codex"), "#!/bin/sh").unwrap();
        fs::write(first.join("codex.cmd"), "@echo off").unwrap();
        fs::write(second.join("codex.exe"), "fixture").unwrap();
        assert_eq!(
            resolve_command(
                "codex",
                &format!("{};{}", display(&first), display(&second))
            ),
            Some(first.join("codex.cmd"))
        );
        fs::remove_file(first.join("codex.cmd")).unwrap();
        assert_eq!(resolve_command("codex", &display(&first)), None);
        fs::write(first.join("codex.cmd"), "fixture").unwrap();
        fs::write(first.join("codex.bat"), "shadow").unwrap();
        assert_eq!(
            resolve_command("codex", &display(&first)),
            Some(first.join("codex.bat"))
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn batch_probe_and_launcher_run_in_paths_with_spaces_and_forward_home_args_and_exit() {
        let root = env::temp_dir().join(format!(
            "companion batch & probe {}",
            OffsetDateTime::now_utc().unix_timestamp_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        let cli = root.join("codex.cmd");
        let home = root.join(".codex-fixture");
        fs::write(&cli, "@echo off\r\nif \"%~1\"==\"--version\" (echo codex-cli fixture& exit /b 0)\r\necho \"%CODEX_HOME%\"\r\necho %~1\r\nexit /b 7\r\n").unwrap();
        let path = env::var("PATH").unwrap_or_default();
        assert_eq!(
            probe_version(&cli, &path, Some(&home)).unwrap(),
            "codex-cli fixture"
        );
        let launcher = root.join("codex-fixture.cmd");
        fs::write(
            &launcher,
            launcher_content(&display(&home), &display(&cli)).unwrap(),
        )
        .unwrap();
        assert_eq!(
            probe_version(&launcher, &path, None).unwrap(),
            "codex-cli fixture"
        );
        let environment = Environment {
            id: "fixture".into(),
            display_name: "Fixture".into(),
            command_alias: "codex-fixture".into(),
            codex_home: display(&home),
            description: None,
            manages_instructions: false,
            launcher_path: Some(display(&launcher)),
            created_at: String::new(),
            updated_at: String::new(),
        };
        let search_path = format!("{};{path}", display(&root));
        assert!(check_environment_alias(&environment, &search_path, &root)
            .unwrap()
            .message
            .contains("works"));
        let shadow = root.join("codex-fixture.bat");
        fs::write(&shadow, "@echo off\r\nexit /b 0\r\n").unwrap();
        assert!(check_environment_alias(&environment, &search_path, &root)
            .err()
            .unwrap()
            .contains("shadows"));
        fs::remove_file(shadow).unwrap();
        fs::write(&launcher, "@echo off\r\nexit /b 0\r\n").unwrap();
        assert!(check_environment_alias(&environment, &search_path, &root)
            .err()
            .unwrap()
            .contains("outdated"));
        fs::write(
            &launcher,
            launcher_content(&display(&home), &display(&cli)).unwrap(),
        )
        .unwrap();
        let output = cli_command(&launcher, &["hello"])
            .unwrap()
            .env("PATH", &path)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(7));
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains(&display(&home)));
        assert!(stdout.contains("hello"));
        fs::write(&cli, "@echo off\r\nexit /b 1\r\n").unwrap();
        let status = cli_status(&display(&root));
        assert!(status.installed);
        assert!(status.version.is_none());
        assert!(status.error.is_some());
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn validates_command_aliases() {
        assert!(validate_alias("codex-cus").is_ok());
        assert!(validate_alias("-bad").is_err());
        assert!(validate_alias("bad.cmd").is_err());
        assert!(validate_alias("CODEX").is_err());
    }
    #[cfg(windows)]
    #[test]
    #[ignore = "Opt-in installed CLI smoke; --version only, isolated CODEX_HOME, no registry writes"]
    fn installed_cli_version_probe_smoke() {
        let root = env::temp_dir().join(format!(
            "companion-cli-smoke-{}",
            OffsetDateTime::now_utc().unix_timestamp_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        let search_path = effective_path();
        let path = resolve_command("codex", &search_path)
            .expect("Codex CLI must be installed for this smoke test");
        let version = probe_version(&path, &search_path, Some(&root)).unwrap();
        assert!(version.starts_with("codex-cli "));
        println!("Resolved {}: {version}", display(&path));
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn launcher_forwards_arguments_and_exit_code() {
        let script = launcher_content(
            "C:\\Users\\A\\.codex-cus",
            "C:\\Program Files\\nodejs\\codex.cmd",
        )
        .unwrap();
        assert!(script.contains("call \"C:\\Program Files\\nodejs\\codex.cmd\" %*"));
        assert!(script.contains("set \"PATH=C:\\Program Files\\nodejs;%PATH%\""));
        assert!(launcher_content("C:\\%TEMP%\\home", "C:\\codex.cmd").is_err());
        assert!(script.contains("endlocal & exit /b %EXIT_CODE%"));
    }
    #[test]
    fn user_path_deduplication_handles_case_and_userprofile() {
        let directory = Path::new("C:\\Users\\A\\bin");
        let saved = env::var("USERPROFILE").ok();
        env::set_var("USERPROFILE", "C:\\Users\\A");
        assert!(path_contains("C:\\tools;%USERPROFILE%\\bin", directory));
        if let Some(value) = saved {
            env::set_var("USERPROFILE", value);
        } else {
            env::remove_var("USERPROFILE");
        }
    }
    #[test]
    fn managed_block_preserves_user_content() {
        let original = "# Mine\n\ncustom rule\n";
        let replaced = replace_managed_block(original, "managed rule").unwrap();
        assert!(replaced.contains("# Mine"));
        assert!(managed_content(&replaced)
            .unwrap()
            .unwrap()
            .contains("managed rule"));
        assert!(
            replace_managed_block("<!-- BEGIN DEV-COMPANION CODEX MANAGED RULES -->", "x").is_err()
        );
    }
    #[test]
    fn environment_metadata_has_no_credentials_and_removal_keeps_other_data_out_of_scope() {
        let value = serde_json::to_string(&Environment {
            id: "a".into(),
            display_name: "A".into(),
            command_alias: "codex-a".into(),
            codex_home: "C:\\Users\\A\\.codex-a".into(),
            description: None,
            manages_instructions: false,
            launcher_path: None,
            created_at: "now".into(),
            updated_at: "now".into(),
        })
        .unwrap();
        assert!(!value.to_ascii_lowercase().contains("token"));
        assert!(!value.to_ascii_lowercase().contains("credential"));
    }
    #[test]
    fn metadata_crud_and_removal_leave_codex_home_in_place() {
        let root = std::env::temp_dir().join(format!(
            "companion-environment-test-{}",
            OffsetDateTime::now_utc().unix_timestamp_nanos()
        ));
        let home = root.join(".codex-work");
        let mut configuration = AppConfiguration::default();
        let created = save_environment(
            &mut configuration,
            EnvironmentInput {
                id: None,
                display_name: "Work".into(),
                command_alias: "codex-work".into(),
                codex_home: display(&home),
                description: Some("Company account".into()),
            },
        )
        .unwrap();
        let updated = save_environment(
            &mut configuration,
            EnvironmentInput {
                id: Some(created.id.clone()),
                display_name: "Work updated".into(),
                command_alias: "codex-work".into(),
                codex_home: display(&home),
                description: None,
            },
        )
        .unwrap();
        assert_eq!(updated.display_name, "Work updated");
        assert!(home.is_dir());
        remove_entry(&mut configuration, &created.id).unwrap();
        assert!(configuration.codex_environments.is_empty());
        assert!(home.is_dir());
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn agents_backup_is_created_before_replacement() {
        let root = std::env::temp_dir().join(format!(
            "companion-agents-test-{}",
            OffsetDateTime::now_utc().unix_timestamp_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        let path = root.join("AGENTS.md");
        fs::write(&path, "user rule").unwrap();
        backup(&path).unwrap();
        assert_eq!(fs::read_dir(&root).unwrap().count(), 2);
        fs::remove_dir_all(root).unwrap();
    }
}
