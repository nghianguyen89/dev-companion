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
    if value.len() < 3
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

fn cli_status() -> CliStatus {
    #[cfg(windows)]
    let path = Command::new("where.exe")
        .arg("codex")
        .output()
        .ok()
        .filter(|out| out.status.success())
        .and_then(|out| String::from_utf8(out.stdout).ok())
        .and_then(|out| {
            out.lines()
                .next()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .map(str::to_owned)
        });
    #[cfg(not(windows))]
    let path = None;
    let version = Command::new("codex")
        .arg("--version")
        .output()
        .ok()
        .filter(|out| out.status.success())
        .and_then(|out| String::from_utf8(out.stdout).ok())
        .map(|out| out.trim().to_owned())
        .filter(|value| !value.is_empty());
    CliStatus {
        installed: version.is_some(),
        path,
        version,
    }
}
fn login_status(home: &Path, installed: bool) -> String {
    if !installed {
        return "unknown".into();
    }
    match Command::new("codex")
        .args(["login", "status"])
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
fn status(environment: Environment, is_default: bool, installed: bool) -> EnvironmentStatus {
    let home = PathBuf::from(&environment.codex_home);
    EnvironmentStatus {
        codex_home_exists: home.is_dir(),
        agents_exists: home.join("AGENTS.md").is_file(),
        login_status: login_status(&home, installed),
        environment,
        is_default,
    }
}

pub fn overview(configuration: &AppConfiguration) -> Result<Overview, String> {
    let cli = cli_status();
    let default = default_environment()?;
    let mut environments = vec![status(default.clone(), true, cli.installed)];
    environments.extend(
        configuration
            .codex_environments
            .iter()
            .cloned()
            .map(|environment| status(environment, false, cli.installed)),
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

fn launcher_content(home: &str, alias: &str) -> String {
    format!("@echo off\r\nsetlocal\r\n\r\nset \"CODEX_HOME={home}\"\r\n\r\nif not exist \"%CODEX_HOME%\" (\r\n    mkdir \"%CODEX_HOME%\" >nul 2>&1\r\n)\r\n\r\nwhere codex >nul 2>&1\r\nif errorlevel 1 (\r\n    echo [{alias}] ERROR: \"codex\" was not found in PATH.\r\n    endlocal\r\n    exit /b 1\r\n)\r\n\r\ncall codex %*\r\nset \"EXIT_CODE=%ERRORLEVEL%\"\r\n\r\nendlocal & exit /b %EXIT_CODE%\r\n")
}
pub fn regenerate_launcher(id: &str) -> Result<ActionResult, String> {
    let (mut configuration, index) = configured(id)?;
    let environment = configuration.codex_environments[index].clone();
    let directory = launcher_dir()?;
    fs_safety::check(&directory)?;
    fs::create_dir_all(&directory).map_err(|_| "Could not create the launcher directory.")?;
    let path = directory.join(format!("{}.cmd", environment.command_alias));
    fs_safety::check(&path)?;
    let content = launcher_content(&environment.codex_home, &environment.command_alias);
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
    Ok(ActionResult {
        message: format!("Launcher created: {}", display(&path)),
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
        let output = Command::new("reg.exe")
            .args(["query", "HKCU\\Environment", "/v", "Path"])
            .output()
            .map_err(|_| "Could not read the User PATH.")?;
        if !output.status.success() {
            return Ok(String::new());
        }
        let rendered = String::from_utf8_lossy(&output.stdout);
        let line = rendered
            .lines()
            .find(|line| line.contains("REG_"))
            .unwrap_or("");
        let type_at = line.find("REG_").ok_or("Could not parse the User PATH.")?;
        let data_at = line[type_at..]
            .find(char::is_whitespace)
            .ok_or("Could not parse the User PATH.")?
            + type_at;
        Ok(line[data_at..].trim().to_owned())
    }
    #[cfg(not(windows))]
    {
        Ok(env::var("PATH").unwrap_or_default())
    }
}
fn expand_user_path(value: &str) -> String {
    value
        .replace(
            "%USERPROFILE%",
            &env::var("USERPROFILE").unwrap_or_default(),
        )
        .replace(
            "%userprofile%",
            &env::var("USERPROFILE").unwrap_or_default(),
        )
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
        let status = Command::new("reg.exe")
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
    if existed { backup(&path)?; }
    fs::write(&path, updated)
        .map_err(|_| "Could not update AGENTS.md.")?;
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
    if !cli_status().installed {
        return Err("Codex CLI was not found in PATH.".into());
    }
    let args: &[&str] = match action {
        "launch" => &[],
        "login" => &["login"],
        "logout" => &["logout"],
        _ => return Err("Unsupported Codex action.".into()),
    };
    let mut command = Command::new("codex");
    command
        .args(args)
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
    fn validates_command_aliases() {
        assert!(validate_alias("codex-cus").is_ok());
        assert!(validate_alias("-bad").is_err());
        assert!(validate_alias("bad.cmd").is_err());
    }
    #[test]
    fn launcher_forwards_arguments_and_exit_code() {
        let script = launcher_content("C:\\Users\\A\\.codex-cus", "codex-cus");
        assert!(script.contains("call codex %*"));
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
