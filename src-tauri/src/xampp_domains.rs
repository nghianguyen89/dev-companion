//! Windows XAMPP domains; private keys are excluded from ordinary migration bundles.
use crate::{fs_safety, platform};
use regex::Regex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    fs,
    io::{Read, Write},
    net::{SocketAddr, TcpStream},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
type Result<T> = std::result::Result<T, String>;
const BEGIN: &str = "# BEGIN DEV COMPANION XAMPP";
const END: &str = "# END DEV COMPANION XAMPP";
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Domain {
    pub name: String,
    pub folder: String,
    pub www: bool,
    pub redirect_https: bool,
    pub directory_listing: bool,
    pub lan: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaInput {
    pub common_name: String,
    pub organization: String,
    pub unit: String,
    pub country: String,
    pub state: String,
    pub city: String,
    pub email: String,
    pub days: u32,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Settings {
    version: u32,
    xampp_path: String,
    initialized: bool,
    domains: Vec<Domain>,
    #[serde(default = "default_keep_recent")]
    backup_keep_recent: u32,
}
fn default_keep_recent() -> u32 {
    10
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            version: 1,
            xampp_path: String::new(),
            initialized: false,
            domains: vec![],
            backup_keep_recent: default_keep_recent(),
        }
    }
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Overview {
    pub installation_path: Option<String>,
    pub config_directory: String,
    pub administrator: bool,
    pub initialized: bool,
    pub ca_ready: bool,
    pub ca_trusted: bool,
    pub domains: Vec<Domain>,
    pub lan_addresses: Vec<String>,
    pub message: String,
    pub backups: BackupSummary,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Action {
    pub message: String,
    pub safety_path: Option<String>,
}
fn io(e: std::io::Error) -> String {
    e.to_string()
}
fn stamp() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
        .to_string()
}
fn text(path: &Path) -> Result<String> {
    fs_safety::check(path)?;
    let m = fs::metadata(path).map_err(io)?;
    if !m.is_file() || m.len() > 2 * 1024 * 1024 {
        return Err("Unsupported configuration file.".into());
    }
    fs::read_to_string(path).map_err(io)
}
fn write(path: &Path, data: &[u8]) -> Result<()> {
    fs_safety::check(path)?;
    let parent = path.parent().ok_or("Missing parent directory.")?;
    fs::create_dir_all(parent).map_err(io)?;
    let tmp = path.with_extension(format!("{}.tmp", stamp()));
    let result = (|| {
        let mut f = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp)
            .map_err(io)?;
        f.write_all(data).map_err(io)?;
        f.sync_all().map_err(io)?;
        drop(f);
        fs::rename(&tmp, path).map_err(io)
    })();
    if result.is_err() {
        let _ = fs::remove_file(tmp);
    }
    result
}
fn state_dir() -> PathBuf {
    platform::portable_root()
        .unwrap_or_else(platform::app_data_dir)
        .join("configs/xampp")
}
fn load_at(dir: &Path) -> Result<Settings> {
    let path = dir.join("settings.json");
    if !path.exists() {
        return Ok(Settings {
            version: 1,
            ..Default::default()
        });
    }
    let s: Settings = serde_json::from_str(&text(&path)?).map_err(|_| {
        "Invalid XAMPP settings; restore a safety copy instead of overwriting.".to_string()
    })?;
    if s.version != 1 {
        return Err("Unsupported XAMPP settings version.".into());
    }
    validate_domains(&s.domains)?;
    validate_keep_recent(s.backup_keep_recent)?;
    Ok(s)
}
fn load() -> Result<Settings> {
    load_at(&state_dir())
}
fn validate_root(path: &Path) -> Result<PathBuf> {
    fs_safety::check(path)?;
    for p in [
        "apache/bin/httpd.exe",
        "apache/bin/openssl.exe",
        "apache/conf/httpd.conf",
        "htdocs",
        "xampp-control.exe",
    ] {
        fs_safety::check(&path.join(p))?;
        if !path.join(p).exists() {
            return Err(format!("Not a usable XAMPP installation: missing {p}"));
        }
    }
    let conf = text(&path.join("apache/conf/httpd.conf"))?;
    let server = directive(&conf, "ServerRoot").ok_or("Apache ServerRoot is missing.")?;
    let server = if server == concat!("$", "{SRVROOT}") {
        conf.lines()
            .find_map(|line| {
                line.trim()
                    .strip_prefix("Define SRVROOT ")
                    .map(|v| v.trim_matches('"').to_owned())
            })
            .ok_or("Apache SRVROOT definition is missing.")?
    } else {
        server
    };
    let configured = PathBuf::from(server);
    let configured = fs::canonicalize(&configured).map_err(|_| {
        "Apache points to a missing installation. Run XAMPP setup_xampp.bat for this folder first."
    })?;
    let expected = fs::canonicalize(path.join("apache")).map_err(io)?;
    if !pstr(&configured).eq_ignore_ascii_case(&pstr(&expected)) {
        return Err("Apache points to another XAMPP installation. Run setup_xampp.bat for the selected folder first.".into());
    }
    let path = fs::canonicalize(path).map_err(io)?;
    Ok(PathBuf::from(
        path.to_string_lossy().trim_start_matches(r"\\?\"),
    ))
}
pub fn installation() -> Result<PathBuf> {
    let s = load()?;
    let p = if !s.xampp_path.is_empty() {
        PathBuf::from(s.xampp_path)
    } else {
        std::env::var_os("XAMPP_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(r"C:\xampp"))
    };
    validate_root(&p)
}
fn quoted(path: &Path) -> Result<String> {
    let p = path.to_string_lossy().replace('\\', "/");
    if p.contains(['"', '\n', '\r', '\0', '$', '<', '>']) {
        return Err("Unsupported characters in folder path.".into());
    }
    Ok(p)
}
fn name(s: &str) -> Result<()> {
    if s != s.to_ascii_lowercase()
        || s.len() > 253
        || (s != "localhost" && !s.contains('.'))
        || s.parse::<std::net::IpAddr>().is_ok()
        || s.split('.').any(|x| {
            x.is_empty()
                || x.len() > 63
                || x.starts_with('-')
                || x.ends_with('-')
                || !x
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        })
    {
        return Err("Enter a lowercase domain such as project.test, without scheme, port, path or wildcard.".into());
    }
    fs_safety::relative(s)?;
    Ok(())
}
fn hostnames(d: &Domain) -> Vec<String> {
    let mut v = vec![d.name.clone()];
    if d.www {
        v.push(format!("www.{}", d.name));
    }
    v
}
fn validate_domains(domains: &[Domain]) -> Result<()> {
    if domains.len() > 100 {
        return Err("At most 100 domains are supported.".into());
    }
    let mut hosts = HashSet::new();
    for d in domains {
        name(&d.name)?;
        quoted(Path::new(&d.folder))?;
        fs_safety::check(Path::new(&d.folder))?;
        for h in hostnames(d) {
            if !hosts.insert(h) {
                return Err("Domain or www alias is already in use.".into());
            }
        }
    }
    Ok(())
}
fn command(exe: &Path) -> Command {
    let mut c = Command::new(exe);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        c.creation_flags(0x08000000);
    }
    c.stdin(Stdio::null());
    c
}
fn output(c: &mut Command) -> Result<String> {
    let o = c.output().map_err(io)?;
    if !o.status.success() {
        return Err(format!(
            "Command failed: {} {}",
            String::from_utf8_lossy(&o.stderr).trim(),
            String::from_utf8_lossy(&o.stdout).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&o.stdout).into_owned())
}
fn ssl(root: &Path, args: &[&str]) -> Result<String> {
    output(
        command(&root.join("apache/bin/openssl.exe"))
            .args(args)
            .env("OPENSSL_CONF", root.join("apache/conf/openssl.cnf"))
            .current_dir(root.join("apache")),
    )
}
fn pstr(p: &Path) -> String {
    p.to_string_lossy().into_owned()
}
fn secure_dir(dir: &Path) -> Result<()> {
    fs_safety::check(dir)?;
    fs::create_dir_all(dir).map_err(io)?;
    #[cfg(windows)]
    {
        let sid = output(command(Path::new("powershell.exe")).args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "[Security.Principal.WindowsIdentity]::GetCurrent().User.Value",
        ]))?;
        output(command(Path::new("icacls.exe")).arg(dir).args([
            "/inheritance:r",
            "/grant:r",
            "*S-1-5-18:(OI)(CI)F",
            "*S-1-5-32-544:(OI)(CI)F",
            &format!("*{}:(OI)(CI)F", sid.trim()),
        ]))?;
    }
    Ok(())
}
fn keys(root: &Path) -> PathBuf {
    root.join("apache/conf/dev-companion")
}
fn ca_paths(root: &Path) -> (PathBuf, PathBuf) {
    let dir = keys(root);
    if dir.join("ca.pem").is_file() {
        (dir.join("ca.pem"), dir.join("ca.key"))
    } else {
        (
            root.join("apache/conf/ssl.ca/cacert.pem"),
            root.join("apache/conf/ssl.ca/cakey.pem"),
        )
    }
}
fn ca_valid(root: &Path, cert: &Path, key: &Path) -> bool {
    (|| -> Result<()> {
        fs_safety::check(cert)?;
        fs_safety::check(key)?;
        ssl(root, &["verify", "-CAfile", &pstr(cert), &pstr(cert)])?;
        let x = ssl(
            root,
            &[
                "x509",
                "-in",
                &pstr(cert),
                "-noout",
                "-ext",
                "basicConstraints,keyUsage",
            ],
        )?;
        if !x.contains("CA:TRUE") || !x.contains("Certificate Sign") {
            return Err("Not a signing CA.".into());
        }
        let public = ssl(root, &["x509", "-in", &pstr(cert), "-pubkey", "-noout"])?;
        let k = ssl(
            root,
            &["pkey", "-in", &pstr(key), "-passin", "pass:", "-pubout"],
        )?;
        if public.trim() != k.trim() {
            return Err("CA key does not match.".into());
        }
        Ok(())
    })()
    .is_ok()
}
fn fingerprint(root: &Path, cert: &Path) -> Result<String> {
    let s = ssl(
        root,
        &[
            "x509",
            "-in",
            &pstr(cert),
            "-noout",
            "-fingerprint",
            "-sha1",
        ],
    )?;
    Ok(s.split('=')
        .nth(1)
        .ok_or("Missing CA fingerprint.")?
        .trim()
        .replace(':', ""))
}
fn trusted(root: &Path, cert: &Path) -> bool {
    fingerprint(root, cert)
        .and_then(|f| output(command(Path::new("certutil.exe")).args(["-store", "Root", &f])))
        .is_ok()
}
#[cfg(windows)]
pub fn administrator() -> bool {
    #[link(name = "shell32")]
    extern "system" {
        fn IsUserAnAdmin() -> i32;
    }
    unsafe { IsUserAnAdmin() != 0 }
}
#[cfg(not(windows))]
pub fn administrator() -> bool {
    false
}
fn require_admin() -> Result<()> {
    if !cfg!(windows) {
        return Err("Domain management supports Windows only.".into());
    }
    if !administrator() {
        return Err(
            "Open Dev Companion as Administrator to change Apache, hosts and machine CA trust."
                .into(),
        );
    }
    Ok(())
}
pub fn elevate() -> Result<Action> {
    #[cfg(windows)]
    {
        use std::{ffi::c_void, os::windows::ffi::OsStrExt};
        #[link(name = "shell32")]
        extern "system" {
            fn ShellExecuteW(
                hwnd: *mut c_void,
                operation: *const u16,
                file: *const u16,
                parameters: *const u16,
                directory: *const u16,
                show: i32,
            ) -> isize;
        }
        let exe = std::env::current_exe().map_err(io)?;
        let op: Vec<u16> = "runas\0".encode_utf16().collect();
        let file: Vec<u16> = exe.as_os_str().encode_wide().chain(Some(0)).collect();
        let directory: Vec<u16> = exe
            .parent()
            .ok_or("Missing application directory.")?
            .as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect();
        if unsafe {
            ShellExecuteW(
                std::ptr::null_mut(),
                op.as_ptr(),
                file.as_ptr(),
                std::ptr::null(),
                directory.as_ptr(),
                1, // SW_SHOWNORMAL: the user requested an interactive elevated app.
            )
        } <= 32
        {
            return Err("Administrator launch was cancelled or failed.".into());
        }
        return Ok(Action {
            message: "Opened Administrator instance.".into(),
            safety_path: None,
        });
    }
    #[cfg(not(windows))]
    Err("Windows only.".into())
}
pub fn overview() -> Result<Overview> {
    let s = load()?;
    let root = installation().ok();
    let (ready, trust) = root
        .as_ref()
        .map(|r| {
            let (c, k) = ca_paths(r);
            let good = ca_valid(r, &c, &k);
            (good, good && trusted(r, &c))
        })
        .unwrap_or((false, false));
    let ips = if cfg!(windows) {
        output(command(Path::new("powershell.exe")).args(["-NoProfile","-NonInteractive","-Command","Get-NetIPAddress -AddressFamily IPv4 | Where-Object { $_.IPAddress -notlike '127.*' -and $_.IPAddress -notlike '169.254.*' -and $_.AddressState -eq 'Preferred' } | Select-Object -ExpandProperty IPAddress -Unique"])).unwrap_or_default().lines().map(str::trim).filter(|x|!x.is_empty()).map(str::to_owned).collect()
    } else {
        vec![]
    };
    let missing = s
        .domains
        .iter()
        .filter(|d| !Path::new(&d.folder).is_dir())
        .map(|d| d.name.clone())
        .collect::<Vec<_>>();
    let backups = match &root {
        Some(root) => backup_summary(root, s.backup_keep_recent)?,
        None => BackupSummary {
            keep_recent: s.backup_keep_recent,
            ..Default::default()
        },
    };
    Ok(Overview {
        backups,
        installation_path: root.map(|p| pstr(&p)),
        config_directory: pstr(&state_dir()),
        administrator: administrator(),
        initialized: s.initialized,
        ca_ready: ready,
        ca_trusted: trust,
        domains: s.domains,
        lan_addresses: ips,
        message: if missing.is_empty() {
            String::new()
        } else {
            format!(
                "Choose a new folder for migrated domains: {}",
                missing.join(", ")
            )
        },
    })
}
pub fn set_installation(path: String) -> Result<Overview> {
    let root = validate_root(Path::new(&path))?;
    let mut s = load()?;
    if s.initialized && !s.xampp_path.eq_ignore_ascii_case(&pstr(&root)) {
        return Err("Existing domains belong to a different installation. Keep its configs folder before switching installations.".into());
    }
    s.xampp_path = pstr(&root);
    write(
        &state_dir().join("settings.json"),
        &serde_json::to_vec_pretty(&s).map_err(|e| e.to_string())?,
    )?;
    overview()
}
fn replace_block(original: &str, body: &str) -> Result<String> {
    let a = original.matches(BEGIN).count();
    let b = original.matches(END).count();
    if a > 1 || b > 1 || a != b {
        return Err("Managed markers are damaged; restore the safety copy.".into());
    }
    let cleaned = if a == 1 {
        let start = original.find(BEGIN).unwrap();
        let end = original.find(END).unwrap();
        if end < start {
            return Err("Managed markers are reversed.".into());
        }
        format!("{}{}", &original[..start], &original[end + END.len()..])
    } else {
        original.into()
    };
    Ok(format!(
        "{}\r\n{BEGIN}\r\n{body}\r\n{END}\r\n",
        cleaned.trim_end()
    ))
}
fn unmanaged(original: &str) -> Result<String> {
    let a = original.matches(BEGIN).count();
    let b = original.matches(END).count();
    if a == 0 && b == 0 {
        return Ok(original.into());
    }
    if a != 1 || b != 1 {
        return Err("Managed markers are damaged.".into());
    }
    let start = original.find(BEGIN).unwrap();
    let end = original.find(END).unwrap();
    if end < start {
        return Err("Managed markers are reversed.".into());
    }
    Ok(format!(
        "{}{}",
        &original[..start],
        &original[end + END.len()..]
    ))
}
fn render_hosts(original: &str, domains: &[Domain]) -> Result<String> {
    let other = unmanaged(original)?;
    let names: HashSet<String> = domains.iter().flat_map(hostnames).collect();
    for line in other.lines() {
        let words: Vec<_> = line
            .split('#')
            .next()
            .unwrap_or("")
            .split_whitespace()
            .collect();
        for h in words.iter().skip(1) {
            if names.contains(&h.to_ascii_lowercase()) {
                return Err(format!(
                    "hosts already has an unmanaged entry for {h}; no entry was overwritten."
                ));
            }
        }
    }
    replace_block(
        original,
        &domains
            .iter()
            .flat_map(hostnames)
            .map(|h| format!("127.0.0.1 {h}"))
            .collect::<Vec<_>>()
            .join("\r\n"),
    )
}
fn render(root: &Path, domains: &[Domain], revision: &str) -> Result<String> {
    validate_domains(domains)?;
    let mut s = format!("# Generated by Dev Companion; revision {revision}\r\n");
    for d in domains {
        let folder = quoted(Path::new(&d.folder))?;
        let alias = if d.www {
            format!("    ServerAlias www.{}\r\n", d.name)
        } else {
            String::new()
        };
        let access = if d.lan {
            "Require ip 127.0.0.0/8 ::1 10.0.0.0/8 172.16.0.0/12 192.168.0.0/16 fc00::/7 fe80::/10"
        } else {
            "Require local"
        };
        let options = if d.directory_listing {
            "+Indexes +FollowSymLinks"
        } else {
            "-Indexes +FollowSymLinks"
        };
        for port in [80, 443] {
            s.push_str(&format!("<VirtualHost *:{port}>\r\n    ServerName {}\r\n{alias}    DocumentRoot \"{folder}\"\r\n    Header always set X-Dev-Companion-Revision \"{revision}\"\r\n    ErrorLog \"logs/{}-error.log\"\r\n    CustomLog \"logs/{}-access.log\" combined\r\n    <Directory \"{folder}\">\r\n        Options {options}\r\n        AllowOverride All\r\n        {access}\r\n    </Directory>\r\n",d.name,d.name,d.name));
            if port == 80 && d.redirect_https {
                s.push_str(&format!("    Redirect permanent / https://{}/\r\n", d.name));
            }
            if port == 443 {
                s.push_str(&format!("    SSLEngine on\r\n    SSLCertificateFile \"{}\"\r\n    SSLCertificateKeyFile \"{}\"\r\n",quoted(&keys(root).join(format!("{}.pem",d.name)))?,quoted(&keys(root).join(format!("{}.key",d.name)))?));
            }
            s.push_str("</VirtualHost>\r\n");
        }
    }
    Ok(s)
}
fn hosts_path() -> PathBuf {
    PathBuf::from(std::env::var_os("SystemRoot").unwrap_or_else(|| r"C:\Windows".into()))
        .join("System32/drivers/etc/hosts")
}

#[derive(Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupSummary {
    pub count: usize,
    pub total_bytes: u64,
    pub prunable_count: usize,
    pub keep_recent: u32,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct BackupOperation {
    version: u32,
    kind: String,
    id: String,
    initial: bool,
    status: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RecoveryRecord {
    target: PathBuf,
    safety_file: Option<PathBuf>,
    existed: bool,
}
#[derive(PartialEq)]
struct BackupFile {
    path: PathBuf,
    bytes: u64,
    modified: SystemTime,
}
struct SafetyBackup {
    folder: PathBuf,
    id: u128,
    bytes: u64,
    files: Vec<BackupFile>,
    fingerprint: Vec<u8>,
    eligible: bool,
}
fn validate_keep_recent(keep: u32) -> Result<()> {
    if !(1..=100).contains(&keep) {
        return Err("Keep between 1 and 100 recent completed backups.".into());
    }
    Ok(())
}
fn backup_id(folder: &Path) -> Result<u128> {
    let name = folder
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or("Invalid backup folder.")?;
    let id = name
        .strip_prefix("dev-companion-")
        .ok_or("Not a managed backup folder.")?;
    let number = id
        .parse::<u128>()
        .map_err(|_| "Invalid backup timestamp.")?;
    if number.to_string() != id {
        return Err("Invalid backup timestamp.".into());
    }
    Ok(number)
}
fn write_operation(folder: &Path, initial: bool, status: &str) -> Result<()> {
    let operation = BackupOperation {
        version: 1,
        kind: "dev-companion-xampp".into(),
        id: backup_id(folder)?.to_string(),
        initial,
        status: status.into(),
    };
    write(
        &folder.join("operation.json"),
        &serde_json::to_vec_pretty(&operation).map_err(|e| e.to_string())?,
    )
}
fn backup_files(folder: &Path, depth: usize, files: &mut Vec<BackupFile>) -> Result<()> {
    if depth > 4 {
        return Err("Unexpected backup directory depth.".into());
    }
    fs_safety::check(folder)?;
    for entry in fs::read_dir(folder).map_err(io)? {
        let path = entry.map_err(io)?.path();
        fs_safety::check(&path)?;
        let metadata = fs::symlink_metadata(&path).map_err(io)?;
        if metadata.is_dir() {
            backup_files(&path, depth + 1, files)?;
        } else if metadata.is_file() {
            if files.len() >= 1000 {
                return Err("Unexpected backup file count.".into());
            }
            files.push(BackupFile {
                path,
                bytes: metadata.len(),
                modified: metadata.modified().map_err(io)?,
            });
        } else {
            return Err("Unsupported backup entry.".into());
        }
    }
    Ok(())
}
fn inspect_backup(base: &Path, folder: &Path) -> Result<SafetyBackup> {
    fs_safety::check(base)?;
    fs_safety::check(folder)?;
    if folder.parent() != Some(base) || !fs::symlink_metadata(folder).map_err(io)?.is_dir() {
        return Err("Backup must be a direct directory under XAMPP backup.".into());
    }
    let id = backup_id(folder)?;
    let manifest = text(&folder.join("recovery.json"))?;
    let records: Vec<RecoveryRecord> =
        serde_json::from_str(&manifest).map_err(|_| "Invalid recovery manifest.")?;
    if records.is_empty() || records.len() > 500 {
        return Err("Unexpected recovery manifest size.".into());
    }
    let mut targets = HashSet::new();
    let mut expected = HashSet::from([folder.join("recovery.json"), folder.join("operation.json")]);
    for (index, record) in records.iter().enumerate() {
        // Targets are metadata only: cleanup never operates on an original target.
        if !record.target.is_absolute()
            || record
                .target
                .components()
                .any(|c| matches!(c, std::path::Component::ParentDir))
            || !targets.insert(record.target.clone())
            || record.existed != record.safety_file.is_some()
        {
            return Err("Invalid recovery target.".into());
        }
        if let Some(file) = &record.safety_file {
            let name = record
                .target
                .file_name()
                .ok_or("Missing recovery filename.")?
                .to_string_lossy();
            let owned = folder.join(format!("{index:03}-{name}"));
            if *file != owned {
                return Err("Recovery file is outside its owned backup.".into());
            }
            fs_safety::check(file)?;
            if !fs::symlink_metadata(file).map_err(io)?.is_file() {
                return Err("Missing regular recovery file.".into());
            }
            expected.insert(file.clone());
        }
    }
    let mut files = vec![];
    backup_files(folder, 0, &mut files)?;
    files.sort_by(|a, b| a.path.cmp(&b.path));
    let operation_text = text(&folder.join("operation.json")).unwrap_or_default();
    let operation: Option<BackupOperation> = serde_json::from_str(&operation_text).ok();
    let completed = operation.is_some_and(|op| {
        op.version == 1
            && op.kind == "dev-companion-xampp"
            && op.id == id.to_string()
            && !op.initial
            && op.status == "completed"
    });
    let flat = files
        .iter()
        .all(|f| f.path.parent() == Some(folder) && expected.contains(&f.path));
    // Empty/unexpected directories also block deletion; never recursively remove a candidate.
    let only_files = fs::read_dir(folder).map_err(io)?.all(|entry| {
        entry
            .ok()
            .and_then(|e| e.file_type().ok())
            .is_some_and(|t| t.is_file())
    });
    let bytes = files.iter().map(|f| f.bytes).sum();
    let fingerprint = Sha256::digest(format!("{manifest}\n{operation_text}").as_bytes()).to_vec();
    Ok(SafetyBackup {
        folder: folder.into(),
        id,
        bytes,
        files,
        fingerprint,
        eligible: completed && flat && only_files,
    })
}
fn scan_backups(root: &Path) -> Result<Vec<SafetyBackup>> {
    let base = root.join("backup");
    fs_safety::check(&base)?;
    if !base.exists() {
        return Ok(vec![]);
    }
    let mut backups = vec![];
    for entry in fs::read_dir(&base).map_err(io)? {
        let path = entry.map_err(io)?.path();
        if backup_id(&path).is_err() {
            continue;
        }
        // Unknown, malformed or linked archives are preserved, never cleanup candidates.
        if let Ok(backup) = inspect_backup(&base, &path) {
            backups.push(backup);
        }
    }
    backups.sort_by_key(|b| std::cmp::Reverse(b.id));
    if let Some(first) = backups.last_mut() {
        first.eligible = false;
    }
    Ok(backups)
}
fn backup_summary(root: &Path, keep: u32) -> Result<BackupSummary> {
    validate_keep_recent(keep)?;
    let backups = scan_backups(root)?;
    Ok(BackupSummary {
        count: backups.len(),
        total_bytes: backups.iter().map(|b| b.bytes).sum(),
        prunable_count: backups
            .iter()
            .filter(|b| b.eligible)
            .count()
            .saturating_sub(keep as usize),
        keep_recent: keep,
    })
}
fn prune_at(root: &Path, keep: u32) -> Result<usize> {
    validate_keep_recent(keep)?;
    let base = root.join("backup");
    let backups = scan_backups(root)?;
    let candidates: Vec<_> = backups
        .iter()
        .filter(|b| b.eligible)
        .skip(keep as usize)
        .collect();
    let mut removed = 0;
    for backup in candidates {
        let current = inspect_backup(&base, &backup.folder)?;
        if !current.eligible
            || current.fingerprint != backup.fingerprint
            || current.files != backup.files
        {
            return Err(format!(
                "Backup changed; stopped cleanup after {removed} removals: {}",
                backup.folder.display()
            ));
        }
        // Preflight the whole flat directory; delete exact validated files, never a tree or target.
        for file in &current.files {
            fs_safety::check(&file.path)?;
        }
        for file in &current.files {
            fs::remove_file(&file.path)
                .map_err(|e| format!("Cleanup stopped after {removed} backups: {e}"))?;
        }
        fs::remove_dir(&current.folder).map_err(io)?;
        removed += 1;
    }
    Ok(removed)
}
pub fn set_backup_retention(keep_recent: u32) -> Result<Overview> {
    require_admin()?;
    validate_keep_recent(keep_recent)?;
    installation()?;
    let mut settings = load()?;
    settings.backup_keep_recent = keep_recent;
    write(
        &state_dir().join("settings.json"),
        &serde_json::to_vec_pretty(&settings).map_err(|e| e.to_string())?,
    )?;
    overview()
}
pub fn prune_backups() -> Result<Action> {
    require_admin()?;
    let root = installation()?;
    let count = prune_at(&root, load()?.backup_keep_recent)?;
    Ok(Action { message: format!("Removed {count} old completed backups. Initial, failed, pending and legacy backups were kept."), safety_path: None })
}
struct Transaction {
    folder: PathBuf,
    saved: Vec<(PathBuf, Option<PathBuf>)>,
}
impl Transaction {
    fn new(root: &Path) -> Result<Self> {
        let folder = root
            .join("backup")
            .join(format!("dev-companion-{}", stamp()));
        secure_dir(&folder)?;
        Ok(Self {
            folder,
            saved: vec![],
        })
    }
    fn save(&mut self, p: &Path) -> Result<()> {
        if self.saved.iter().any(|(x, _)| x == p) {
            return Ok(());
        }
        fs_safety::check(p)?;
        let backup = if p.exists() {
            if !fs::metadata(p).map_err(io)?.is_file() {
                return Err("Cannot replace a non-file target.".into());
            }
            let b = self.folder.join(format!(
                "{:03}-{}",
                self.saved.len(),
                p.file_name().unwrap_or_default().to_string_lossy()
            ));
            fs::copy(p, &b).map_err(io)?;
            if fs::read(p).map_err(io)? != fs::read(&b).map_err(io)? {
                return Err("Safety copy verification failed.".into());
            }
            Some(b)
        } else {
            None
        };
        self.saved.push((p.into(), backup));
        let manifest: Vec<_> = self
            .saved
            .iter()
            .map(|(p, b)| serde_json::json!({"target":p,"safetyFile":b,"existed":b.is_some()}))
            .collect();
        write(
            &self.folder.join("recovery.json"),
            &serde_json::to_vec_pretty(&manifest).map_err(|e| e.to_string())?,
        )?;
        Ok(())
    }
    fn put(&mut self, p: &Path, data: &[u8]) -> Result<()> {
        self.save(p)?;
        write(p, data)
    }
    fn remove(&mut self, p: &Path) -> Result<()> {
        if p.exists() {
            self.save(p)?;
            fs::remove_file(p).map_err(io)?;
        }
        Ok(())
    }
    fn rollback(&self) -> Vec<String> {
        let mut errors = vec![];
        for (p, b) in self.saved.iter().rev() {
            let r = if let Some(b) = b {
                fs::read(b).map_err(io).and_then(|data| write(p, &data))
            } else if p.exists() {
                fs::remove_file(p).map_err(io)
            } else {
                Ok(())
            };
            if let Err(e) = r {
                errors.push(format!("{}: {e}", p.display()));
            }
        }
        errors
    }
}
fn ca_subject(c: &CaInput) -> Result<String> {
    if c.common_name.trim().is_empty()
        || c.country.len() != 2
        || !c.country.bytes().all(|b| b.is_ascii_alphabetic())
        || c.days < 30
        || c.days > 3650
    {
        return Err("CA needs a name, two-letter country and 30–3650 days.".into());
    }
    let mut s = String::new();
    for (k, v) in [
        ("CN", &c.common_name),
        ("O", &c.organization),
        ("OU", &c.unit),
        ("C", &c.country),
        ("ST", &c.state),
        ("L", &c.city),
        ("emailAddress", &c.email),
    ] {
        if v.len() > 128 || v.chars().any(|x| x.is_control()) {
            return Err("Invalid CA identity value.".into());
        }
        if !v.is_empty() {
            s.push_str(&format!(
                "/{k}={}",
                v.replace('\\', "\\\\")
                    .replace('/', "\\/")
                    .replace('+', "\\+")
            ));
        }
    }
    Ok(s)
}
fn make_ca(root: &Path, c: &CaInput, stage: &Path) -> Result<()> {
    let subject = ca_subject(c)?;
    ssl(
        root,
        &[
            "req",
            "-x509",
            "-newkey",
            "rsa:4096",
            "-sha256",
            "-nodes",
            "-days",
            &c.days.to_string(),
            "-subj",
            &subject,
            "-keyout",
            &pstr(&stage.join("ca.key")),
            "-out",
            &pstr(&stage.join("ca.pem")),
            "-addext",
            "basicConstraints=critical,CA:TRUE",
            "-addext",
            "keyUsage=critical,keyCertSign,cRLSign",
        ],
    )?;
    Ok(())
}
fn leaf(root: &Path, d: &Domain, stage: &Path, ca: &Path, cakey: &Path) -> Result<()> {
    let (mut low, mut high) = (0u32, 366u32);
    while high - low > 1 {
        let mid = (low + high) / 2;
        if ssl(
            root,
            &[
                "x509",
                "-in",
                &pstr(ca),
                "-noout",
                "-checkend",
                &(u64::from(mid) * 86400 + 60).to_string(),
            ],
        )
        .is_ok()
        {
            low = mid;
        } else {
            high = mid;
        }
    }
    let days = low;
    if days < 1 {
        return Err("CA expires too soon; renew it before creating domains.".into());
    }
    let key = stage.join(format!("{}.key", d.name));
    let cert = stage.join(format!("{}.pem", d.name));
    let csr = stage.join(format!("{}.csr", d.name));
    let ext = stage.join(format!("{}.cnf", d.name));
    let mut san = hostnames(d)
        .into_iter()
        .map(|h| format!("DNS:{h}"))
        .collect::<Vec<_>>();
    if d.name == "localhost" {
        san.extend(["IP:127.0.0.1".into(), "IP:::1".into()]);
    }
    write(&ext,format!("basicConstraints=critical,CA:FALSE\nkeyUsage=critical,digitalSignature,keyEncipherment\nextendedKeyUsage=serverAuth\nsubjectAltName={}\n",san.join(",")).as_bytes())?;
    ssl(
        root,
        &[
            "req",
            "-new",
            "-newkey",
            "rsa:2048",
            "-sha256",
            "-nodes",
            "-subj",
            &format!("/CN={}", d.name),
            "-keyout",
            &pstr(&key),
            "-out",
            &pstr(&csr),
        ],
    )?;
    ssl(
        root,
        &[
            "x509",
            "-req",
            "-in",
            &pstr(&csr),
            "-CA",
            &pstr(ca),
            "-CAkey",
            &pstr(cakey),
            "-set_serial",
            &format!("0x{}", ssl(root, &["rand", "-hex", "16"])?.trim()),
            "-days",
            &days.to_string(),
            "-sha256",
            "-extfile",
            &pstr(&ext),
            "-out",
            &pstr(&cert),
        ],
    )?;
    ssl(
        root,
        &[
            "verify",
            "-purpose",
            "sslserver",
            "-verify_hostname",
            &d.name,
            "-CAfile",
            &pstr(ca),
            &pstr(&cert),
        ],
    )?;
    Ok(())
}
fn enable_modules(main: &str) -> Result<String> {
    let mut lines = vec![];
    for line in main.lines() {
        let t = line.trim_start_matches('#').trim();
        if t.starts_with("LoadModule ")
            && [
                "ssl_module ",
                "socache_shmcb_module ",
                "headers_module ",
                "rewrite_module ",
            ]
            .iter()
            .any(|x| t.starts_with(&format!("LoadModule {x}")))
        {
            lines.push(t.to_string());
        } else {
            lines.push(line.into());
        }
    }
    let mut s = lines.join("\r\n");
    for include in ["conf/extra/httpd-ssl.conf", "conf/extra/httpd-vhosts.conf"] {
        let mut found = false;
        s = s
            .lines()
            .map(|line| {
                let plain = line.trim_start_matches('#').trim();
                if plain.starts_with("Include ") && plain[8..].trim_matches('"') == include {
                    found = true;
                    plain.to_string()
                } else {
                    line.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\r\n");
        if !found {
            return Err(format!(
                "Required XAMPP configuration include is missing: {include}"
            ));
        }
    }
    for m in [
        "ssl_module",
        "socache_shmcb_module",
        "headers_module",
        "rewrite_module",
    ] {
        if !s
            .lines()
            .any(|x| x.starts_with(&format!("LoadModule {m} ")))
        {
            return Err(format!("Required Apache module is missing: {m}"));
        }
    }
    Ok(s)
}
fn vhost_blocks(content: &str) -> Vec<(String, String)> {
    let re = Regex::new(r"(?is)<VirtualHost\s+[^>]+>(.*?)</VirtualHost>").unwrap();
    re.captures_iter(content)
        .map(|c| (c[0].to_string(), c[1].to_string()))
        .collect()
}
fn directive(body: &str, key: &str) -> Option<String> {
    body.lines().find_map(|l| {
        let mut x = l.trim().splitn(2, char::is_whitespace);
        if x.next()?.eq_ignore_ascii_case(key) {
            Some(x.next()?.trim().trim_matches('"').to_owned())
        } else {
            None
        }
    })
}
fn import_domains(root: &Path) -> Result<Vec<Domain>> {
    let http = text(&root.join("apache/conf/extra/httpd-vhosts.conf"))?;
    let https = text(&root.join("apache/conf/extra/httpd-ssl.conf"))?;
    let mut out = Vec::<Domain>::new();
    for (block, body) in vhost_blocks(&http) {
        if !block.lines().next().unwrap_or("").contains(":80>") {
            continue;
        }
        let Some(n) = directive(&body, "ServerName") else {
            continue;
        };
        if n == "localhost" {
            continue;
        }
        name(&n)?;
        let folder = directive(&body, "DocumentRoot")
            .ok_or("Cannot import a VirtualHost without DocumentRoot.")?;
        let www = matches_alias(&body, &n)?;
        let options = directive(&body, "Options").unwrap_or_default();
        out.push(Domain {
            name: n,
            folder,
            www,
            redirect_https: false,
            directory_listing: options
                .split_whitespace()
                .any(|x| x == "Indexes" || x == "+Indexes"),
            lan: false,
        });
    }
    for (_, body) in vhost_blocks(&https) {
        if let Some(n) = directive(&body, "ServerName") {
            if let Some(d) = out.iter_mut().find(|d| d.name == n) {
                if directive(&body, "DocumentRoot").as_deref() != Some(&d.folder) {
                    return Err(format!(
                        "HTTP/HTTPS folders differ for {n}; resolve before import."
                    ));
                }
                d.www |= matches_alias(&body, &n)?;
            }
        }
    }
    validate_domains(&out)?;
    Ok(out)
}
fn matches_alias(body: &str, n: &str) -> Result<bool> {
    let mut www = false;
    for line in body.lines() {
        if let Some(a) = directive(line, "ServerAlias") {
            for alias in a.split_whitespace() {
                if alias == format!("www.{n}") {
                    www = true;
                } else {
                    return Err(format!("Unsupported legacy alias {alias}; keep the original configuration until resolved."));
                }
            }
        }
    }
    Ok(www)
}
fn unmanaged_names(root: &Path, domains: &[Domain]) -> Result<()> {
    let names: HashSet<_> = domains.iter().flat_map(hostnames).collect();
    for folder in [root.join("apache/conf"), root.join("apache/conf/extra")] {
        for entry in fs::read_dir(folder).map_err(io)? {
            let p = entry.map_err(io)?.path();
            if p.extension().and_then(|x| x.to_str()) != Some("conf") {
                continue;
            }
            for (_, body) in vhost_blocks(&text(&p)?) {
                for key in ["ServerName", "ServerAlias"] {
                    if let Some(value) = directive(&body, key) {
                        for h in value.split_whitespace() {
                            if names
                                .contains(&h.split(':').next().unwrap_or("").to_ascii_lowercase())
                            {
                                return Err(format!(
                                    "Domain {h} already exists outside managed configuration: {}",
                                    p.display()
                                ));
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(())
}
fn strip_imported(content: &str, domains: &[Domain]) -> String {
    let mut s = content.to_string();
    for (block, body) in vhost_blocks(content) {
        if let Some(n) = directive(&body, "ServerName") {
            let n = n.split(':').next().unwrap_or("");
            if domains.iter().any(|d| d.name.eq_ignore_ascii_case(n)) {
                s = s.replace(&block, "");
            }
        }
    }
    s
}
fn apache_check(root: &Path) -> Result<()> {
    output(
        command(&root.join("apache/bin/httpd.exe"))
            .current_dir(root.join("apache"))
            .args(["-t", "-f", &pstr(&root.join("apache/conf/httpd.conf"))]),
    )?;
    Ok(())
}
fn apache_pid(root: &Path) -> Option<u32> {
    text(&root.join("apache/logs/httpd.pid"))
        .ok()?
        .trim()
        .parse()
        .ok()
}
#[cfg(windows)]
fn signal(root: &Path, pid: u32, event_kind: &str) -> Result<bool> {
    use std::{ffi::c_void, os::windows::ffi::OsStrExt};
    #[link(name = "kernel32")]
    extern "system" {
        fn OpenProcess(access: u32, inherit: i32, pid: u32) -> *mut c_void;
        fn QueryFullProcessImageNameW(
            h: *mut c_void,
            flags: u32,
            path: *mut u16,
            size: *mut u32,
        ) -> i32;
        fn OpenEventW(access: u32, inherit: i32, name: *const u16) -> *mut c_void;
        fn SetEvent(h: *mut c_void) -> i32;
        fn CloseHandle(h: *mut c_void) -> i32;
    }
    unsafe {
        let process = OpenProcess(0x1000, 0, pid);
        if process.is_null() {
            return if std::io::Error::last_os_error().raw_os_error() == Some(87) {
                Ok(false)
            } else {
                Err("Apache process is inaccessible.".into())
            };
        }
        let mut buffer = vec![0u16; 32768];
        let mut len = buffer.len() as u32;
        let ok = QueryFullProcessImageNameW(process, 0, buffer.as_mut_ptr(), &mut len);
        CloseHandle(process);
        if ok == 0 {
            return Err("Cannot verify the selected Apache process.".into());
        }
        let image = PathBuf::from(String::from_utf16_lossy(&buffer[..len as usize]));
        let expected = root.join("apache/bin/httpd.exe");
        if !image
            .as_os_str()
            .to_string_lossy()
            .replace('/', "\\")
            .eq_ignore_ascii_case(&expected.as_os_str().to_string_lossy().replace('/', "\\"))
        {
            return Ok(false);
        }
        if event_kind.is_empty() {
            return Ok(true);
        }
        let event: Vec<u16> = std::ffi::OsStr::new(&format!("ap{pid}_{event_kind}"))
            .encode_wide()
            .chain(Some(0))
            .collect();
        let h = OpenEventW(2, 0, event.as_ptr());
        if h.is_null() {
            return Err("Cannot signal this Apache instance to restart.".into());
        }
        let success = SetEvent(h);
        CloseHandle(h);
        if success == 0 {
            return Err("Apache restart signal failed.".into());
        }
    }
    Ok(true)
}
#[cfg(not(windows))]
fn signal(_root: &Path, _pid: u32, _event_kind: &str) -> Result<bool> {
    Err("Windows only.".into())
}
fn served(revision: &str) -> bool {
    let address: SocketAddr = "127.0.0.1:80".parse().unwrap();
    let Ok(mut stream) = TcpStream::connect_timeout(&address, Duration::from_millis(200)) else {
        return false;
    };
    let _ = stream.set_read_timeout(Some(Duration::from_millis(300)));
    let _ = stream.set_write_timeout(Some(Duration::from_millis(300)));
    if stream
        .write_all(b"GET / HTTP/1.0\r\nHost: localhost\r\n\r\n")
        .is_err()
    {
        return false;
    }
    let mut data = Vec::new();
    let mut buffer = [0u8; 1024];
    while data.len() < 16384 {
        match stream.read(&mut buffer) {
            Ok(0) | Err(_) => break,
            Ok(n) => data.extend_from_slice(&buffer[..n]),
        }
        if data.windows(4).any(|w| w == b"\r\n\r\n") {
            break;
        }
    }
    String::from_utf8_lossy(&data).lines().any(|line| {
        line.split_once(':').is_some_and(|(key, value)| {
            key.eq_ignore_ascii_case("X-Dev-Companion-Revision") && value.trim() == revision
        })
    })
}
fn restart(root: &Path, revision: &str) -> Result<()> {
    let running = match apache_pid(root) {
        Some(pid) => signal(root, pid, "restart")?,
        None => false,
    };
    if !running {
        let mut c = command(&root.join("apache/bin/httpd.exe"));
        c.current_dir(root.join("apache"))
            .args(["-f", &pstr(&root.join("apache/conf/httpd.conf"))])
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        c.spawn().map_err(io)?;
    }
    for _ in 0..60 {
        if served(revision) {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(200));
    }
    Err("Apache did not serve the applied configuration within 30 seconds. Check port 80/443 and Apache error.log.".into())
}
fn firewall(root: &Path, lan: bool) -> Result<()> {
    let hash = format!("{:x}", Sha256::digest(pstr(root).to_lowercase().as_bytes()));
    let n = format!("name=Dev Companion XAMPP {}", &hash[..12]);
    let result = output(command(Path::new("netsh.exe")).args([
        "advfirewall",
        "firewall",
        "delete",
        "rule",
        &n,
    ]));
    if !lan {
        return result.map(|_| ()).or_else(|_| Ok(()));
    }
    output(command(Path::new("netsh.exe")).args([
        "advfirewall",
        "firewall",
        "add",
        "rule",
        &n,
        "dir=in",
        "action=allow",
        "profile=private",
        "remoteip=localsubnet",
        "protocol=TCP",
        "localport=80,443",
        &format!("program={}", pstr(&root.join("apache/bin/httpd.exe"))),
    ]))?;
    Ok(())
}
fn legacy_files(root: &Path, domains: &[Domain]) -> Vec<PathBuf> {
    let mut names = vec![
        "apache/makecert.ca.bat".into(),
        "apache/conf/openssl-ca.cnf".into(),
    ];
    for d in domains {
        names.push(format!("apache/makecert.{}.bat", d.name));
        names.push(format!("apache/conf/openssl-{}.cnf", d.name));
        for p in [
            format!("apache/conf/ssl.pem/{}.pem", d.name),
            format!("apache/conf/ssl.csr/{}.csr", d.name),
            format!("apache/conf/ssl.key/{}.key", d.name),
            format!("apache/conf/ssl.key/{}.key.pem", d.name),
        ] {
            names.push(p);
        }
    }
    for n in ["ca.key", "cacert.crt", "cacert.pem", "cakey.pem"] {
        names.push(format!("apache/conf/ssl.ca/{n}"));
    }
    names
        .into_iter()
        .map(|p| root.join(p))
        .filter(|p| p.is_file())
        .collect()
}
fn apply_at(
    root: &Path,
    dir: &Path,
    mut settings: Settings,
    ca: Option<&CaInput>,
    renew: Option<&Domain>,
    hosts: &Path,
    live: bool,
) -> Result<Action> {
    validate_domains(&settings.domains)?;
    if !settings.domains.iter().any(|d| d.name == "localhost") {
        return Err("localhost is required as the default host.".into());
    }
    let initial = !settings.initialized;
    let old = load_at(dir)?;
    let mut tx = Transaction::new(root)?;
    write_operation(&tx.folder, initial, "pending")?;
    if initial {
        for path in legacy_files(root, &settings.domains) {
            tx.save(&path)?;
        }
        for rel in [
            "apache/conf/httpd.conf",
            "apache/conf/extra/httpd-vhosts.conf",
            "apache/conf/extra/httpd-ssl.conf",
        ] {
            tx.save(&root.join(rel))?;
        }
        tx.save(hosts)?;
    }
    let stage = tx.folder.join("staging");
    secure_dir(&stage)?;
    secure_dir(&keys(root))?;
    let revision = stamp();
    let main_path = root.join("apache/conf/httpd.conf");
    let main_before = text(&main_path)?;
    let old_revision = Regex::new(r"revision (\d+)")
        .unwrap()
        .captures(&text(&dir.join("httpd-domains.conf")).unwrap_or_default())
        .map(|c| c[1].to_string());
    let running_before = if live {
        match apache_pid(root) {
            Some(p) => signal(root, p, "")?,
            None => false,
        }
    } else {
        false
    };
    let mut runtime_changed = false;
    let mut added_trust: Option<String> = None;
    let result = (|| -> Result<()> {
        let (old_ca, old_key) = ca_paths(root);
        let cert = keys(root).join("ca.pem");
        let key = keys(root).join("ca.key");
        if ca_valid(root, &old_ca, &old_key) {
            if old_ca != cert {
                tx.save(&cert)?;
                fs::copy(&old_ca, &cert).map_err(io)?;
                tx.save(&key)?;
                fs::copy(&old_key, &key).map_err(io)?;
            }
        } else {
            if cert.exists() || key.exists() || old_ca.exists() || old_key.exists() {
                return Err("Existing CA is incomplete, expired or mismatched. Its files were preserved; repair or explicitly archive them before creating a replacement.".into());
            }
            make_ca(
                root,
                ca.ok_or("CA information is required on first setup.")?,
                &stage,
            )?;
            for n in ["ca.pem", "ca.key"] {
                tx.save(&keys(root).join(n))?;
                fs::copy(stage.join(n), keys(root).join(n)).map_err(io)?;
            }
        }
        for d in &settings.domains {
            if initial || renew.is_some_and(|r| r.name == d.name) {
                leaf(root, d, &stage, &cert, &key)?;
                for ext in ["pem", "key"] {
                    let n = format!("{}.{ext}", d.name);
                    tx.save(&keys(root).join(&n))?;
                    fs::copy(stage.join(&n), keys(root).join(n)).map_err(io)?;
                }
            }
        }
        let mut main = enable_modules(&main_before)?;
        let include = format!("Include \"{}\"", quoted(&dir.join("httpd-domains.conf"))?);
        main = replace_block(&main, &include)?;
        if initial {
            for rel in [
                "apache/conf/extra/httpd-vhosts.conf",
                "apache/conf/extra/httpd-ssl.conf",
            ] {
                let p = root.join(rel);
                let content = text(&p)?;
                tx.put(&p, strip_imported(&content, &settings.domains).as_bytes())?;
            }
        }
        let managed = main.find(BEGIN).unwrap();
        let block = main[managed..].to_string();
        let prefix = main[..managed].trim_end();
        let insert = prefix
            .find("# Supplemental configuration")
            .unwrap_or(prefix.len());
        main = format!("{}\r\n{block}\r\n{}", &prefix[..insert], &prefix[insert..]);
        unmanaged_names(root, &settings.domains)?;
        let hosts_before = text(hosts)?;
        let adopted = if initial {
            adopt_loopback(&hosts_before, &settings.domains)?
        } else {
            hosts_before
        };
        let hosts_text = render_hosts(&adopted, &settings.domains)?;
        tx.put(
            &dir.join("httpd-domains.conf"),
            render(root, &settings.domains, &revision)?.as_bytes(),
        )?;
        tx.put(&main_path, main.as_bytes())?;
        apache_check(root)?;
        tx.put(hosts, hosts_text.as_bytes())?;
        ssl(
            root,
            &[
                "x509",
                "-in",
                &pstr(&cert),
                "-outform",
                "DER",
                "-out",
                &pstr(&stage.join("ca.crt")),
            ],
        )?;
        tx.put(
            &dir.join("ca.crt"),
            &fs::read(stage.join("ca.crt")).map_err(io)?,
        )?;
        if live {
            if !trusted(root, &cert) {
                let f = fingerprint(root, &cert)?;
                output(command(Path::new("certutil.exe")).args([
                    "-addstore",
                    "Root",
                    &pstr(&dir.join("ca.crt")),
                ]))?;
                added_trust = Some(f);
            }
            firewall(root, settings.domains.iter().any(|d| d.lan))?;
            runtime_changed = true;
            restart(root, &revision)?;
        }
        settings.initialized = true;
        tx.put(
            &dir.join("settings.json"),
            &serde_json::to_vec_pretty(&settings).map_err(|e| e.to_string())?,
        )?;
        if initial {
            let keep_ca = surviving_ca_reference(root)?;
            for p in legacy_files(root, &settings.domains) {
                if !keep_ca || !p.starts_with(root.join("apache/conf/ssl.ca")) {
                    tx.remove(&p)?;
                }
            }
        }
        for d in &old.domains {
            if !settings.domains.iter().any(|n| n.name == d.name) {
                for ext in ["pem", "key"] {
                    tx.remove(&keys(root).join(format!("{}.{ext}", d.name)))?;
                }
            }
        }
        apache_check(root)?;
        Ok(())
    })();
    if let Err(e) = result {
        let mut recovery = tx.rollback();
        if let Err(e) = write_operation(&tx.folder, initial, "failed") {
            recovery.push(format!("Could not mark failed backup: {e}"));
        }
        if live {
            if let Some(f) = added_trust {
                if let Err(e) =
                    output(command(Path::new("certutil.exe")).args(["-delstore", "Root", &f]))
                {
                    recovery.push(e);
                }
            }
            if let Err(e) = firewall(root, old.domains.iter().any(|d| d.lan)) {
                recovery.push(e);
            }
            if runtime_changed {
                if running_before && old_revision.is_some() {
                    let r = old_revision.unwrap();
                    if let Err(e) = restart(root, &r) {
                        recovery.push(e);
                    }
                } else if let Some(pid) = apache_pid(root) {
                    let kind = if running_before {
                        "restart"
                    } else {
                        "shutdown"
                    };
                    match signal(root,pid,kind) {
                        Err(e)=>recovery.push(e),
                        Ok(true) if !running_before=>{for _ in 0..40 {if !signal(root,pid,"").unwrap_or(true){break;}thread::sleep(Duration::from_millis(100));}if signal(root,pid,"").unwrap_or(true){recovery.push("Apache shutdown after rollback is unconfirmed.".into());}},
                        Ok(true)=>recovery.push("Original Apache configuration restored and reload requested; verify its sites.".into()),
                        Ok(false)=>(),
                    }
                }
            }
        }
        return Err(format!(
            "{e}\nSafety copy: {}\n{}",
            tx.folder.display(),
            if recovery.is_empty() {
                "File changes rolled back.".into()
            } else {
                format!("Recovery needs attention: {}", recovery.join("; "))
            }
        ));
    }
    let _ = fs::remove_dir_all(&stage);
    let mut message = if live {
        "Saved, CA trusted and Apache configuration verified after restart.".to_string()
    } else {
        "Saved and Apache syntax verified (no machine changes).".to_string()
    };
    match write_operation(&tx.folder, initial, "completed") {
        Err(e) => message.push_str(&format!(
            "\nBackup retained; status could not be saved: {e}"
        )),
        Ok(()) if live => match prune_at(root, settings.backup_keep_recent) {
            Ok(count) if count > 0 => {
                message.push_str(&format!("\nRemoved {count} old completed backups."))
            }
            Err(e) => message.push_str(&format!(
                "\nConfiguration is applied; backup cleanup needs attention: {e}"
            )),
            _ => (),
        },
        _ => (),
    }
    Ok(Action {
        message,
        safety_path: Some(pstr(&tx.folder)),
    })
}
pub fn initialize(ca: CaInput) -> Result<Action> {
    require_admin()?;
    let root = installation()?;
    let mut s = load()?;
    if s.initialized {
        return apply_at(&root, &state_dir(), s, None, None, &hosts_path(), true);
    }
    let (c, k) = ca_paths(&root);
    if !ca_valid(&root, &c, &k) {
        ca_subject(&ca)?;
    }
    s.xampp_path = pstr(&root);
    let mut domains = vec![Domain {
        name: "localhost".into(),
        folder: pstr(&root.join("htdocs")),
        www: true,
        redirect_https: false,
        directory_listing: false,
        lan: false,
    }];
    domains.extend(import_domains(&root)?);
    s.domains = domains;
    apply_at(&root, &state_dir(), s, Some(&ca), None, &hosts_path(), true)
}
fn saved_domain<'a>(settings: &'a Settings, domain_name: &str) -> Result<&'a Domain> {
    name(domain_name)?;
    settings
        .domains
        .iter()
        .find(|d| d.name == domain_name)
        .ok_or_else(|| "Domain no longer exists; refresh.".into())
}
pub fn open_domain(domain_name: String, https: bool) -> Result<()> {
    let settings = load()?;
    let domain = saved_domain(&settings, &domain_name)?;
    let url = format!(
        "{}://{}/",
        if https { "https" } else { "http" },
        domain.name
    );
    #[cfg(windows)]
    {
        Command::new("explorer.exe")
            .arg(url)
            .spawn()
            .map(|_| ())
            .map_err(|_| "Cannot open the domain in the default browser.".into())
    }
    #[cfg(not(windows))]
    {
        let _ = url;
        Err("Windows only.".into())
    }
}
pub fn open_domain_folder(domain_name: String) -> Result<()> {
    let settings = load()?;
    let domain = saved_domain(&settings, &domain_name)?;
    let folder = Path::new(&domain.folder);
    fs_safety::check(folder)?;
    if !folder.is_dir() {
        return Err("The domain folder no longer exists.".into());
    }
    #[cfg(windows)]
    {
        Command::new("explorer.exe")
            .arg(folder)
            .spawn()
            .map(|_| ())
            .map_err(|_| "Cannot open the domain folder.".into())
    }
    #[cfg(not(windows))]
    {
        Err("Windows only.".into())
    }
}
pub fn save_domain(mut d: Domain, previous: Option<String>) -> Result<Action> {
    require_admin()?;
    let root = installation()?;
    let mut s = load()?;
    if !s.initialized {
        return Err("Initialize CA and localhost first.".into());
    }
    d.name = d.name.trim().to_ascii_lowercase();
    name(&d.name)?;
    fs_safety::check(Path::new(&d.folder))?;
    if !Path::new(&d.folder).is_dir() {
        return Err("Select an existing web folder.".into());
    }
    d.folder = pstr(&fs::canonicalize(&d.folder).map_err(io)?)
        .trim_start_matches(r"\\?\")
        .to_owned();
    if previous.as_deref() == Some("localhost") && d.name != "localhost" {
        return Err("localhost cannot be renamed.".into());
    }
    if let Some(p) = previous {
        let index = s
            .domains
            .iter()
            .position(|x| x.name == p)
            .ok_or("Domain no longer exists; refresh.")?;
        s.domains[index] = d.clone();
    } else {
        s.domains.push(d.clone());
    }
    apply_at(&root, &state_dir(), s, None, Some(&d), &hosts_path(), true)
}
pub fn delete_domain(n: String, confirmation: String) -> Result<Action> {
    require_admin()?;
    name(&n)?;
    if n == "localhost" || n != confirmation {
        return Err("Type the domain name to delete; localhost cannot be deleted.".into());
    }
    let root = installation()?;
    let mut s = load()?;
    if !s.initialized {
        return Err("Initialize XAMPP domains first.".into());
    }
    let before = s.domains.len();
    s.domains.retain(|d| d.name != n);
    if s.domains.len() == before {
        return Err("Domain no longer exists; refresh.".into());
    }
    apply_at(&root, &state_dir(), s, None, None, &hosts_path(), true)
}
pub fn export_ca(destination: String) -> Result<Action> {
    let root = installation()?;
    let (c, k) = ca_paths(&root);
    if !ca_valid(&root, &c, &k) {
        return Err("No valid local CA is available.".into());
    }
    let folder = Path::new(&destination);
    fs_safety::check(folder)?;
    if !folder.is_dir() {
        return Err("Choose an existing destination folder.".into());
    }
    let target = folder.join("dev-companion-local-ca.crt");
    if target.exists() {
        return Err("CA export already exists; choose another folder.".into());
    }
    ssl(
        &root,
        &[
            "x509",
            "-in",
            &pstr(&c),
            "-outform",
            "DER",
            "-out",
            &pstr(&target),
        ],
    )?;
    Ok(Action{message:format!("Public CA exported: {}. Install it in Trusted Root Certification Authorities on LAN clients and map domains to this PC's LAN IP. No private key was exported.",target.display()),safety_path:None})
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn opens_only_saved_primary_domain_names() {
        let settings = Settings {
            version: 1,
            initialized: true,
            domains: vec![Domain {
                name: "a.test".into(),
                folder: r"D:\web".into(),
                www: true,
                redirect_https: false,
                directory_listing: false,
                lan: false,
            }],
            ..Default::default()
        };
        assert_eq!(saved_domain(&settings, "a.test").unwrap().folder, r"D:\web");
        for target in [
            "www.a.test",
            "unknown.test",
            "https://a.test/",
            "../a.test",
            "a.test & calc.exe",
        ] {
            assert!(saved_domain(&settings, target).is_err(), "{target}");
        }
    }
    #[test]
    fn validates_names_aliases_and_paths() {
        for n in ["a.test", "localhost", "cus-gitlab.local"] {
            assert!(name(n).is_ok());
        }
        for n in [
            "https://x.test",
            "*.x.test",
            "../foo",
            "a.test:80",
            "A.test",
            "127.0.0.1",
            "con.test",
            "x\n.test",
        ] {
            assert!(name(n).is_err(), "{n}");
        }
        let d = Domain {
            name: "a.test".into(),
            folder: r"C:\web".into(),
            www: true,
            redirect_https: true,
            directory_listing: false,
            lan: false,
        };
        let mut a = d.clone();
        a.name = "www.a.test".into();
        assert!(validate_domains(&[d, a]).is_err());
    }
    #[test]
    fn preserves_hosts_and_rejects_conflicts() {
        let d = Domain {
            name: "a.test".into(),
            folder: r"C:\web".into(),
            www: true,
            redirect_https: false,
            directory_listing: false,
            lan: false,
        };
        let h = render_hosts("# user\r\n10.0.0.2 other.test", &[d.clone()]).unwrap();
        assert!(h.contains("10.0.0.2 other.test"));
        assert_eq!(h, render_hosts(&h, &[d.clone()]).unwrap());
        assert!(render_hosts("1.2.3.4 a.test", &[d]).is_err());
        assert!(replace_block(&format!("{END}\n{BEGIN}"), "x").is_err());
    }
    #[test]
    fn generates_access_options_and_tls() {
        let d = Domain {
            name: "a.test".into(),
            folder: r"D:\projects\public".into(),
            www: true,
            redirect_https: true,
            directory_listing: false,
            lan: false,
        };
        let s = render(Path::new(r"C:\xampp"), &[d.clone()], "123").unwrap();
        assert!(s.contains("Require local"));
        assert!(s.contains("Options -Indexes +FollowSymLinks"));
        assert!(s.contains("https://a.test/"));
        assert!(s.contains("SSLCertificateKeyFile"));
        let mut lan = d;
        lan.lan = true;
        lan.directory_listing = true;
        let s = render(Path::new(r"C:\xampp"), &[lan], "124").unwrap();
        assert!(s.contains("Require ip 127.0.0.0/8"));
        assert!(s.contains("Options +Indexes"));
    }

    #[test]
    fn retention_keeps_initial_failed_pending_legacy_and_unknown_content() {
        let root = std::env::temp_dir().join(format!("xampp-retention-{}", stamp()));
        fs::create_dir_all(&root).unwrap();
        let source = root.join("source.conf");
        fs::write(&source, "working configuration").unwrap();
        let make = |status: Option<&str>, initial| {
            let mut tx = Transaction::new(&root).unwrap();
            tx.save(&source).unwrap();
            if let Some(status) = status {
                write_operation(&tx.folder, initial, status).unwrap();
            }
            tx.folder
        };
        let first = make(Some("completed"), false); // Even unflagged earliest archive is protected.
        let old = make(Some("completed"), false);
        let legacy = make(None, false);
        let failed = make(Some("failed"), false);
        let pending = make(Some("pending"), false);
        let initial = make(Some("completed"), true);
        let latest = make(Some("completed"), false);
        let extra = make(Some("completed"), false);
        fs::write(extra.join("user-notes.txt"), "retain me").unwrap();
        let nested = make(Some("completed"), false);
        fs::create_dir(nested.join("staging")).unwrap();
        let malformed = make(Some("completed"), false);
        let mut manifest: serde_json::Value =
            serde_json::from_str(&text(&malformed.join("recovery.json")).unwrap()).unwrap();
        manifest[0]["safetyFile"] = serde_json::json!(source);
        fs::write(malformed.join("recovery.json"), manifest.to_string()).unwrap();
        let summary = backup_summary(&root, 1).unwrap();
        assert_eq!(summary.count, 9);
        assert_eq!(summary.prunable_count, 1);
        assert!(summary.total_bytes > 0);
        assert_eq!(prune_at(&root, 1).unwrap(), 1);
        assert!(!old.exists());
        for protected in [
            &first, &legacy, &failed, &pending, &initial, &latest, &extra, &nested, &malformed,
        ] {
            assert!(protected.exists(), "{}", protected.display());
        }
        assert_eq!(text(&source).unwrap(), "working configuration");
        assert_eq!(prune_at(&root, 1).unwrap(), 0);
        assert_eq!(backup_summary(&root, 1).unwrap().prunable_count, 0);
        #[cfg(windows)]
        {
            let outside = root.join("outside");
            fs::create_dir(&outside).unwrap();
            fs::write(outside.join("keep.txt"), "untouched").unwrap();
            let linked = make(Some("completed"), false);
            let junction = linked.join("junction");
            output(command(Path::new("cmd.exe")).args([
                "/c",
                "mklink",
                "/J",
                &pstr(&junction),
                &pstr(&outside),
            ]))
            .unwrap();
            assert!(inspect_backup(&root.join("backup"), &linked).is_err());
            assert_eq!(prune_at(&root, 1).unwrap(), 0);
            assert_eq!(text(&outside.join("keep.txt")).unwrap(), "untouched");
            fs::remove_dir(&junction).unwrap();
        }
        fs::remove_dir_all(&root).unwrap();
    }
    #[test]
    fn old_settings_default_to_ten_backups_and_reject_invalid_limits() {
        let root = std::env::temp_dir().join(format!("xampp-retention-settings-{}", stamp()));
        fs::create_dir_all(&root).unwrap();
        fs::write(
            root.join("settings.json"),
            r#"{"version":1,"xamppPath":"","initialized":false,"domains":[]}"#,
        )
        .unwrap();
        assert_eq!(load_at(&root).unwrap().backup_keep_recent, 10);
        assert_eq!(Settings::default().backup_keep_recent, 10);
        for keep in [0, 101, u32::MAX] {
            assert!(validate_keep_recent(keep).is_err());
        }
        for keep in [1, 10, 100] {
            assert!(validate_keep_recent(keep).is_ok());
        }
        fs::write(
            root.join("settings.json"),
            r#"{"version":1,"xamppPath":"","initialized":false,"domains":[],"backupKeepRecent":0}"#,
        )
        .unwrap();
        assert!(load_at(&root).is_err());
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn transaction_restores_written_and_deleted_files() {
        let root = std::env::temp_dir().join(format!("xampp-domain-test-{}", stamp()));
        fs::create_dir_all(&root).unwrap();
        let old = root.join("old");
        let new = root.join("new");
        fs::write(&old, "original").unwrap();
        let mut t = Transaction::new(&root).unwrap();
        t.put(&old, b"changed").unwrap();
        t.put(&new, b"new").unwrap();
        t.remove(&old).unwrap();
        assert!(t.rollback().is_empty());
        assert_eq!(fs::read_to_string(old).unwrap(), "original");
        assert!(!new.exists());
        fs::remove_dir_all(root).unwrap();
    }
}

fn adopt_loopback(original: &str, domains: &[Domain]) -> Result<String> {
    let names: HashSet<_> = domains.iter().flat_map(hostnames).collect();
    let mut lines = vec![];
    for line in original.lines() {
        let mut parts = line.splitn(2, '#');
        let data = parts.next().unwrap_or("");
        let comment = parts.next();
        let mut words = data.split_whitespace();
        let Some(ip) = words.next() else {
            lines.push(line.to_string());
            continue;
        };
        let hosts: Vec<_> = words.collect();
        if hosts
            .iter()
            .any(|h| names.contains(&h.to_ascii_lowercase()))
            && ["127.0.0.1", "::1"].contains(&ip)
        {
            let remaining: Vec<_> = hosts
                .into_iter()
                .filter(|h| !names.contains(&h.to_ascii_lowercase()))
                .collect();
            let mut l = if remaining.is_empty() {
                String::new()
            } else {
                format!("{ip} {}", remaining.join(" "))
            };
            if let Some(c) = comment {
                l.push_str(&format!(" #{c}"));
            }
            if !l.is_empty() {
                lines.push(l);
            }
        } else {
            lines.push(line.to_string());
        }
    }
    Ok(lines.join("\r\n"))
}
fn surviving_ca_reference(root: &Path) -> Result<bool> {
    for dir in [root.join("apache/conf"), root.join("apache/conf/extra")] {
        for entry in fs::read_dir(dir).map_err(io)? {
            let p = entry.map_err(io)?.path();
            if p.extension().and_then(|x| x.to_str()) == Some("conf")
                && text(&p)?.lines().any(|l| {
                    !l.trim_start().starts_with('#') && l.to_ascii_lowercase().contains("ssl.ca")
                })
            {
                return Ok(true);
            }
        }
    }
    Ok(false)
}
#[cfg(test)]
mod integration_tests {
    use super::*;
    #[test]
    fn imports_https_alias_and_preserves_other_hosts() {
        let r = std::env::temp_dir().join(format!("xampp-import-{}", stamp()));
        fs::create_dir_all(r.join("apache/conf/extra")).unwrap();
        fs::write(
            r.join("apache/conf/extra/httpd-vhosts.conf"),
            "<VirtualHost *:80>\nServerName a.test\nDocumentRoot \"C:/web\"\n</VirtualHost>",
        )
        .unwrap();
        fs::write(r.join("apache/conf/extra/httpd-ssl.conf"),"<VirtualHost *:443>\nServerName a.test\nServerAlias www.a.test\nDocumentRoot \"C:/web\"\n</VirtualHost>").unwrap();
        let ds = import_domains(&r).unwrap();
        assert!(ds[0].www);
        let h = adopt_loopback(
            "127.0.0.1 a.test unrelated.test # keep\n::1 www.a.test\n10.0.0.2 other.test",
            &ds,
        )
        .unwrap();
        let h = render_hosts(&h, &ds).unwrap();
        assert!(h.contains("127.0.0.1 unrelated.test # keep"));
        assert!(h.contains("10.0.0.2 other.test"));
        assert_eq!(h.matches("127.0.0.1 a.test").count(), 1);
        fs::write(
            r.join("apache/conf/extra/remaining.conf"),
            "SSLCACertificateFile conf/ssl.ca/cacert.pem",
        )
        .unwrap();
        assert!(surviving_ca_reference(&r).unwrap());
        fs::remove_dir_all(r).unwrap();
    }
    #[test]
    #[ignore = "Opt-in real OpenSSL/Apache binaries; isolated temporary configuration only"]
    fn real_openssl_apache_isolated() {
        let real = PathBuf::from(r"C:\xampp");
        assert!(real.join("apache/bin/httpd.exe").is_file());
        let r = std::env::temp_dir().join(format!("xampp-isolated-{}", stamp()));
        secure_dir(&r).unwrap();
        for dir in [
            "apache/bin",
            "apache/conf/extra",
            "apache/logs",
            "htdocs",
            "public",
        ] {
            fs::create_dir_all(r.join(dir)).unwrap();
        }
        for entry in fs::read_dir(real.join("apache/bin")).unwrap() {
            let p = entry.unwrap().path();
            if p.is_file() {
                fs::copy(&p, r.join("apache/bin").join(p.file_name().unwrap())).unwrap();
            }
        }
        fs::copy(
            real.join("apache/conf/openssl.cnf"),
            r.join("apache/conf/openssl.cnf"),
        )
        .unwrap();
        let modules = quoted(&real.join("apache/modules")).unwrap();
        let main=format!("ServerRoot \"{}\"\nServerName localhost\nListen 80\nLogFormat \"%h %l %u %t %r %>s %b\" combined\nLoadModule authz_core_module \"{modules}/mod_authz_core.so\"\nLoadModule authz_host_module \"{modules}/mod_authz_host.so\"\nLoadModule log_config_module \"{modules}/mod_log_config.so\"\n#LoadModule ssl_module \"{modules}/mod_ssl.so\"\n#LoadModule socache_shmcb_module \"{modules}/mod_socache_shmcb.so\"\n#LoadModule headers_module \"{modules}/mod_headers.so\"\n#LoadModule rewrite_module \"{modules}/mod_rewrite.so\"\nLoadModule alias_module \"{modules}/mod_alias.so\"\nLoadModule dir_module \"{modules}/mod_dir.so\"\nInclude conf/extra/httpd-vhosts.conf\nInclude conf/extra/httpd-ssl.conf\n",quoted(&r.join("apache")).unwrap());
        fs::write(r.join("apache/conf/httpd.conf"), main).unwrap();
        fs::write(r.join("apache/conf/extra/httpd-vhosts.conf"), "").unwrap();
        fs::write(r.join("apache/conf/extra/httpd-ssl.conf"), "Listen 443\n").unwrap();
        let dir = r.join("configs");
        let hosts = r.join("hosts");
        fs::write(&hosts, "# untouched hosts\n").unwrap();
        let localhost = Domain {
            name: "localhost".into(),
            folder: pstr(&r.join("htdocs")),
            www: true,
            redirect_https: false,
            directory_listing: false,
            lan: false,
        };
        let ca = CaInput {
            common_name: "Isolated Test CA".into(),
            organization: "".into(),
            unit: "".into(),
            country: "VN".into(),
            state: "".into(),
            city: "".into(),
            email: "".into(),
            days: 30,
        };
        let s = Settings {
            version: 1,
            xampp_path: pstr(&r),
            initialized: false,
            domains: vec![localhost.clone()],
            ..Default::default()
        };
        apply_at(&r, &dir, s, Some(&ca), None, &hosts, false).unwrap();
        let mut s = load_at(&dir).unwrap();
        let d = Domain {
            name: "project.test".into(),
            folder: pstr(&r.join("public")),
            www: true,
            redirect_https: true,
            directory_listing: true,
            lan: true,
        };
        s.domains.push(d.clone());
        apply_at(&r, &dir, s, None, Some(&d), &hosts, false).unwrap();
        let s = load_at(&dir).unwrap();
        assert_eq!(s.domains.len(), 2);
        assert!(text(&hosts).unwrap().contains("www.project.test"));
        ssl(
            &r,
            &[
                "verify",
                "-purpose",
                "sslserver",
                "-verify_hostname",
                "www.project.test",
                "-CAfile",
                &pstr(&keys(&r).join("ca.pem")),
                &pstr(&keys(&r).join("project.test.pem")),
            ],
        )
        .unwrap();
        let revision = Regex::new(r"revision (\d+)")
            .unwrap()
            .captures(&text(&dir.join("httpd-domains.conf")).unwrap())
            .unwrap()[1]
            .to_owned();
        struct ServerGuard(PathBuf);
        impl Drop for ServerGuard {
            fn drop(&mut self) {
                if let Some(pid) = apache_pid(&self.0) {
                    let _ = signal(&self.0, pid, "shutdown");
                    for _ in 0..50 {
                        if !signal(&self.0, pid, "").unwrap_or(true) {
                            break;
                        }
                        thread::sleep(Duration::from_millis(100));
                    }
                }
            }
        }
        let server = if std::env::var("DEV_COMPANION_XAMPP_RESTART_SMOKE").as_deref() == Ok("YES") {
            let guard = ServerGuard(r.clone());
            restart(&r, &revision).unwrap();
            let mut edited = load_at(&dir).unwrap();
            edited.domains[1].directory_listing = false;
            apply_at(&r, &dir, edited, None, None, &hosts, false).unwrap();
            let new_revision = Regex::new(r"revision (\d+)")
                .unwrap()
                .captures(&text(&dir.join("httpd-domains.conf")).unwrap())
                .unwrap()[1]
                .to_owned();
            restart(&r, &new_revision).unwrap();
            Some(guard)
        } else {
            None
        };
        let before = text(&r.join("apache/conf/httpd.conf")).unwrap();
        let old_cert = fs::read(keys(&r).join("localhost.pem")).unwrap();
        let mut broken = s.clone();
        broken.domains.push(Domain {
            name: "collision.test".into(),
            ..d.clone()
        });
        fs::write(r.join("apache/conf/extra/collision.conf"),format!("<VirtualHost *:80>\nServerName collision.test\nDocumentRoot \"{}\"\n</VirtualHost>",quoted(&r.join("public")).unwrap())).unwrap();
        assert!(apply_at(&r, &dir, broken, None, None, &hosts, false).is_err());
        assert_eq!(text(&r.join("apache/conf/httpd.conf")).unwrap(), before);
        assert_eq!(fs::read(keys(&r).join("localhost.pem")).unwrap(), old_cert);
        let mut s = s;
        s.domains.retain(|x| x.name != "project.test");
        apply_at(&r, &dir, s, None, None, &hosts, false).unwrap();
        assert!(!keys(&r).join("project.test.key").exists());
        assert!(!text(&hosts).unwrap().contains("project.test"));
        drop(server);
        fs::remove_dir_all(r).unwrap();
    }
    #[test]
    #[ignore = "Explicit user-approved initialization of copied C:/xampp"]
    fn initialize_copied_xampp_live() {
        assert_eq!(
            std::env::var("DEV_COMPANION_XAMPP_LIVE_SETUP").as_deref(),
            Ok("YES")
        );
        require_admin().unwrap();
        let root = PathBuf::from(r"C:\xampp");
        validate_root(&root).unwrap();
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("release/portable/configs/xampp");
        let mut s = load_at(&dir).unwrap();
        if !s.initialized {
            s.xampp_path = pstr(&root);
            s.domains = vec![Domain {
                name: "localhost".into(),
                folder: pstr(&root.join("htdocs")),
                www: true,
                redirect_https: false,
                directory_listing: false,
                lan: false,
            }];
            s.domains.extend(import_domains(&root).unwrap());
        }
        let ca = CaInput {
            common_name: "".into(),
            organization: "".into(),
            unit: "".into(),
            country: "".into(),
            state: "".into(),
            city: "".into(),
            email: "".into(),
            days: 3650,
        };
        let result = apply_at(&root, &dir, s, Some(&ca), None, &hosts_path(), true).unwrap();
        println!("{}", serde_json::to_string(&result).unwrap());
    }
}
