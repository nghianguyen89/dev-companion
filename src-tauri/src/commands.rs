use crate::{
    codex, codex_content, codex_environment, compression,
    config::{self, AppConfiguration, ConfigurationError},
    file_transfer, local_delete, session_storage,
};
use tauri_plugin_dialog::DialogExt;

#[tauri::command]
pub async fn get_compression_readiness() -> Result<compression::Readiness, String> {
    run_blocking(compression::readiness).await
}

#[tauri::command]
pub async fn scan_compression_source(source: String) -> Result<compression::SourceTree, String> {
    run_blocking(move || compression::scan_source(&source)).await?
}

#[tauri::command]
pub async fn pick_compression_folder(
    app: tauri::AppHandle,
    title: String,
) -> Result<Option<String>, String> {
    run_blocking(move || {
        Ok(app
            .dialog()
            .file()
            .set_title(title)
            .blocking_pick_folder()
            .and_then(|folder| folder.into_path().ok())
            .map(|path| compression::display_path(&path)))
    })
    .await?
}

#[tauri::command]
pub async fn preview_compression(
    config: compression::CompressionConfig,
) -> Result<compression::CommandPreview, String> {
    run_blocking(move || compression::preview(&config)).await?
}

#[tauri::command]
pub async fn start_compression(
    app: tauri::AppHandle,
    config: compression::CompressionConfig,
) -> Result<compression::Started, String> {
    run_blocking(move || compression::start(app, config)).await?
}

#[tauri::command]
pub async fn cancel_compression() -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(compression::cancel)
        .await
        .map_err(|_| "Cancellation did not complete.")?
}

#[tauri::command]
pub async fn get_skills() -> Result<codex_content::SkillsOverview, String> {
    run_blocking(codex_content::skills).await?
}
#[tauri::command]
pub async fn import_skill(
    app: tauri::AppHandle,
) -> Result<Option<codex_content::ActionResult>, String> {
    run_blocking(move || {
        let Some(folder) = app
            .dialog()
            .file()
            .set_title("Select a skill folder")
            .blocking_pick_folder()
        else {
            return Ok(None);
        };
        codex_content::import_skill(
            &folder
                .into_path()
                .map_err(|_| "The selected skill folder does not have a readable local path.")?,
        )
        .map(Some)
    })
    .await?
}
#[tauri::command]
pub async fn export_skill(
    app: tauri::AppHandle,
    id: String,
) -> Result<Option<codex_content::ActionResult>, String> {
    run_blocking(move || {
        let Some(folder) = app
            .dialog()
            .file()
            .set_title("Select a folder for the exported skill")
            .blocking_pick_folder()
        else {
            return Ok(None);
        };
        codex_content::export_skill(
            &id,
            &folder
                .into_path()
                .map_err(|_| "The selected destination does not have a readable local path.")?,
        )
        .map(Some)
    })
    .await?
}
#[tauri::command]
pub async fn get_pets() -> Result<codex_content::PetsOverview, String> {
    run_blocking(codex_content::pets).await?
}
#[tauri::command]
pub async fn install_pet(
    app: tauri::AppHandle,
) -> Result<Option<codex_content::ActionResult>, String> {
    run_blocking(move || {
        let Some(folder) = app
            .dialog()
            .file()
            .set_title("Select a pet folder")
            .blocking_pick_folder()
        else {
            return Ok(None);
        };
        codex_content::install_pet(
            &folder
                .into_path()
                .map_err(|_| "The selected pet folder does not have a readable local path.")?,
        )
        .map(Some)
    })
    .await?
}
#[tauri::command]
pub async fn remove_pet(
    id: String,
    confirmation: String,
) -> Result<codex_content::ActionResult, String> {
    run_blocking(move || codex_content::remove_pet(&id, &confirmation)).await?
}

// ponytail: one global gate; split by proven independent operation classes if this limits responsiveness.
static COMMAND_GATE: tauri::async_runtime::Mutex<()> = tauri::async_runtime::Mutex::const_new(());

async fn run_blocking<T: Send + 'static>(
    operation: impl FnOnce() -> T + Send + 'static,
) -> Result<T, String> {
    let gate = COMMAND_GATE.lock().await;
    tauri::async_runtime::spawn_blocking(move || {
        let _gate = gate;
        operation()
    })
    .await
    .map_err(|_| "The background operation did not complete.".into())
}

#[tauri::command]
pub async fn get_beyond_compare_readiness() -> Result<crate::beyond_compare::Readiness, String> {
    run_blocking(crate::beyond_compare::readiness).await
}
#[tauri::command]
pub async fn preview_beyond_compare(
    app: tauri::AppHandle,
    credentials_included: bool,
) -> Result<Option<crate::personal_bundle::BundlePreview>, String> {
    run_blocking(move || {
        let Some(file) = app
            .dialog()
            .file()
            .set_title("Choose a Beyond Compare settings package")
            .add_filter("Beyond Compare package", &["bcpkg"])
            .blocking_pick_file()
        else {
            return Ok(None);
        };
        crate::personal_bundle::preview_create(
            file.into_path()
                .map_err(|_| "The selected package does not have a readable local path.")?,
            credentials_included,
        )
        .map(Some)
    })
    .await?
}
#[tauri::command]
pub async fn create_beyond_compare_bundle(
    token: String,
) -> Result<crate::personal_bundle::BundleCreated, String> {
    run_blocking(move || crate::personal_bundle::create(&token)).await?
}
#[tauri::command]
pub async fn inspect_beyond_compare_bundle(
    app: tauri::AppHandle,
) -> Result<Option<crate::personal_bundle::BundleInspection>, String> {
    run_blocking(move || {
        let Some(file) = app
            .dialog()
            .file()
            .set_title("Choose a personal bundle")
            .add_filter("Personal bundle", &["zip"])
            .blocking_pick_file()
        else {
            return Ok(None);
        };
        crate::personal_bundle::inspect(
            file.into_path()
                .map_err(|_| "The selected bundle does not have a readable local path.")?,
        )
        .map(Some)
    })
    .await?
}
#[tauri::command]
pub async fn preview_beyond_compare_recovery(
    token: String,
) -> Result<crate::personal_bundle::RecoveryPreview, String> {
    run_blocking(move || crate::personal_bundle::preview_recovery(&token)).await?
}
#[tauri::command]
pub async fn recover_beyond_compare(
    token: String,
    confirmation: String,
) -> Result<crate::personal_bundle::RecoveryResult, String> {
    run_blocking(move || crate::personal_bundle::recover(&token, &confirmation)).await?
}

#[tauri::command]
pub async fn get_sourcetree_readiness() -> Result<crate::sourcetree::Readiness, String> {
    run_blocking(crate::sourcetree::readiness).await
}
#[tauri::command]
pub async fn preview_sourcetree() -> Result<crate::sourcetree::Preview, String> {
    run_blocking(crate::sourcetree::preview).await?
}
#[tauri::command]
pub async fn create_sourcetree_bundle(token: String) -> Result<crate::sourcetree::Created, String> {
    run_blocking(move || crate::sourcetree::create(&token)).await?
}
#[tauri::command]
pub async fn inspect_sourcetree_bundle(
    app: tauri::AppHandle,
) -> Result<Option<crate::sourcetree::Inspection>, String> {
    run_blocking(move || {
        let Some(file) = app
            .dialog()
            .file()
            .set_title("Choose a SourceTree personal bundle")
            .add_filter("Personal bundle", &["zip"])
            .blocking_pick_file()
        else {
            return Ok(None);
        };
        crate::sourcetree::inspect(
            file.into_path()
                .map_err(|_| "The selected bundle does not have a readable local path.")?,
        )
        .map(Some)
    })
    .await?
}
#[tauri::command]
pub async fn preview_sourcetree_recovery(
    token: String,
) -> Result<crate::sourcetree::RecoveryPreview, String> {
    run_blocking(move || crate::sourcetree::preview_recovery(&token)).await?
}
#[tauri::command]
pub async fn recover_sourcetree(
    token: String,
    confirmation: String,
) -> Result<crate::sourcetree::RecoveryResult, String> {
    run_blocking(move || crate::sourcetree::recover(&token, &confirmation)).await?
}
#[tauri::command]
pub async fn preview_sourcetree_config() -> Result<crate::sourcetree_config::Preview, String> {
    run_blocking(crate::sourcetree_config::preview).await?
}
#[tauri::command]
pub async fn create_sourcetree_config_bundle(
    token: String,
    password: String,
) -> Result<crate::sourcetree_config::Created, String> {
    run_blocking(move || crate::sourcetree_config::create(&token, &password)).await?
}
#[tauri::command]
pub async fn open_sourcetree_config_bundle_folder() -> Result<(), String> {
    run_blocking(crate::sourcetree_config::open_bundle_folder).await?
}
#[tauri::command]
pub async fn inspect_sourcetree_config_bundle(
    app: tauri::AppHandle,
    password: String,
) -> Result<Option<crate::sourcetree_config::Inspection>, String> {
    run_blocking(move || {
        let Some(file) = app
            .dialog()
            .file()
            .set_title("Choose a complete SourceTree configuration bundle")
            .add_filter("SourceTree configuration bundle", &["zip"])
            .blocking_pick_file()
        else {
            return Ok(None);
        };
        crate::sourcetree_config::inspect(
            file.into_path()
                .map_err(|_| "The selected bundle does not have a readable local path.")?,
            &password,
        )
        .map(Some)
    })
    .await?
}
#[tauri::command]
pub async fn preview_sourcetree_config_recovery(
    token: String,
    password: String,
) -> Result<crate::sourcetree_config::RecoveryPreview, String> {
    run_blocking(move || crate::sourcetree_config::preview_recovery(&token, &password)).await?
}
#[tauri::command]
pub async fn recover_sourcetree_config(
    token: String,
    confirmation: String,
    password: String,
) -> Result<crate::sourcetree_config::RecoveryResult, String> {
    run_blocking(move || crate::sourcetree_config::recover(&token, &confirmation, &password))
        .await?
}
#[tauri::command]
pub async fn delete_sourcetree_config_bundle(
    token: String,
    confirmation: String,
) -> Result<(), String> {
    run_blocking(move || crate::sourcetree_config::delete(&token, &confirmation)).await?
}

#[tauri::command]
pub async fn get_xampp_readiness() -> Result<crate::xampp::Readiness, String> {
    run_blocking(crate::xampp::readiness).await
}

#[tauri::command]
pub async fn get_codex_environment_overview() -> Result<codex_environment::Overview, String> {
    run_blocking(|| {
        let configuration = config::load().map_err(configuration_error)?;
        codex_environment::overview(&configuration)
    })
    .await?
}
#[tauri::command]
pub async fn save_codex_environment(
    input: codex_environment::EnvironmentInput,
) -> Result<codex_environment::Environment, String> {
    run_blocking(move || codex_environment::create_or_update(input)).await?
}
#[tauri::command]
pub async fn regenerate_codex_launcher(
    id: String,
) -> Result<codex_environment::ActionResult, String> {
    run_blocking(move || codex_environment::regenerate_launcher(&id)).await?
}
#[tauri::command]
pub async fn remove_codex_launcher(
    id: String,
    confirmation: String,
) -> Result<codex_environment::ActionResult, String> {
    run_blocking(move || codex_environment::remove_launcher(&id, &confirmation)).await?
}
#[tauri::command]
pub async fn add_codex_launcher_dir_to_user_path() -> Result<codex_environment::ActionResult, String>
{
    run_blocking(codex_environment::add_launcher_dir_to_user_path).await?
}
#[tauri::command]
pub async fn get_codex_environment_instructions(
    id: String,
) -> Result<codex_environment::Instructions, String> {
    run_blocking(move || codex_environment::instructions(&id)).await?
}
#[tauri::command]
pub async fn update_codex_environment_instructions(
    id: String,
    content: String,
) -> Result<codex_environment::ActionResult, String> {
    run_blocking(move || codex_environment::update_instructions(&id, &content)).await?
}
#[tauri::command]
pub async fn delete_codex_environment(
    id: String,
    confirmation: String,
) -> Result<codex_environment::ActionResult, String> {
    run_blocking(move || codex_environment::delete_environment(&id, &confirmation)).await?
}
#[tauri::command]
pub async fn open_codex_environment_home(
    id: String,
) -> Result<codex_environment::ActionResult, String> {
    run_blocking(move || codex_environment::open_home(&id)).await?
}
#[tauri::command]
pub async fn open_codex_environment_agents(
    id: String,
) -> Result<codex_environment::ActionResult, String> {
    run_blocking(move || codex_environment::open_agents(&id)).await?
}
#[tauri::command]
pub async fn run_codex_environment_action(
    id: String,
    action: String,
) -> Result<codex_environment::ActionResult, String> {
    run_blocking(move || codex_environment::run_cli(&id, &action)).await?
}
#[tauri::command]
pub async fn preview_xampp(app: tauri::AppHandle) -> Result<Option<crate::xampp::Preview>, String> {
    run_blocking(move || {
        let Some(folders) = app
            .dialog()
            .file()
            .set_title("Select direct XAMPP htdocs project folders")
            .blocking_pick_folders()
        else {
            return Ok(None);
        };
        let paths = folders
            .into_iter()
            .map(|folder| {
                folder
                    .into_path()
                    .map_err(|_| "The selected XAMPP project does not have a readable local path.")
            })
            .collect::<Result<Vec<_>, _>>()?;
        crate::xampp::preview(paths).map(Some)
    })
    .await?
}
#[tauri::command]
pub async fn create_xampp_bundle(token: String) -> Result<crate::xampp::Created, String> {
    run_blocking(move || crate::xampp::create(&token)).await?
}
#[tauri::command]
pub async fn inspect_xampp_bundle(
    app: tauri::AppHandle,
) -> Result<Option<crate::xampp::Inspection>, String> {
    run_blocking(move || {
        let Some(file) = app
            .dialog()
            .file()
            .set_title("Choose an XAMPP personal bundle")
            .add_filter("Personal bundle", &["zip"])
            .blocking_pick_file()
        else {
            return Ok(None);
        };
        crate::xampp::inspect(
            file.into_path()
                .map_err(|_| "The selected bundle does not have a readable local path.")?,
        )
        .map(Some)
    })
    .await?
}
#[tauri::command]
pub async fn preview_xampp_recovery(
    token: String,
) -> Result<crate::xampp::RecoveryPreview, String> {
    run_blocking(move || crate::xampp::preview_recovery(&token)).await?
}
#[tauri::command]
pub async fn recover_xampp(
    token: String,
    confirmation: String,
) -> Result<crate::xampp::RecoveryResult, String> {
    run_blocking(move || crate::xampp::recover(&token, &confirmation)).await?
}

#[tauri::command]
pub async fn preview_environment(
    groups: Vec<String>,
) -> Result<crate::environment::Preview, String> {
    run_blocking(move || crate::environment::preview(groups)).await?
}
#[tauri::command]
pub async fn create_environment(token: String) -> Result<crate::environment::Created, String> {
    run_blocking(move || crate::environment::create(&token)).await?
}
#[tauri::command]
pub async fn inspect_environment(
    app: tauri::AppHandle,
) -> Result<Option<crate::environment::Preview>, String> {
    run_blocking(move || {
        let Some(file) = app
            .dialog()
            .file()
            .add_filter("Environment ZIP", &["zip"])
            .blocking_pick_file()
        else {
            return Ok(None);
        };
        crate::environment::inspect(file.into_path().map_err(|e| e.to_string())?).map(Some)
    })
    .await?
}
#[tauri::command]
pub async fn preview_environment_restore(
    token: String,
) -> Result<crate::environment::RestorePreview, String> {
    run_blocking(move || crate::environment::preview_restore(&token)).await?
}
#[tauri::command]
pub async fn restore_environment(
    token: String,
    confirmation: String,
) -> Result<crate::environment::Restored, String> {
    run_blocking(move || crate::environment::restore(&token, &confirmation)).await?
}
#[tauri::command]
pub async fn get_codex_migration_overview() -> Result<crate::codex_migration::Overview, String> {
    run_blocking(crate::codex_migration::overview).await?
}
#[tauri::command]
pub async fn preview_codex_migration(
    account_ids: Vec<String>,
    groups: Vec<String>,
) -> Result<crate::codex_migration::Preview, String> {
    run_blocking(move || crate::codex_migration::preview(account_ids, groups)).await?
}
#[tauri::command]
pub async fn create_codex_migration(
    token: String,
) -> Result<crate::codex_migration::Created, String> {
    run_blocking(move || crate::codex_migration::create(&token)).await?
}
#[tauri::command]
pub async fn list_codex_migration_archives() -> Result<crate::codex_migration::Archives, String> {
    run_blocking(crate::codex_migration::list_archives).await?
}
#[tauri::command]
pub async fn open_codex_migration_archive(name: String) -> Result<(), String> {
    run_blocking(move || crate::codex_migration::open_archive(&name)).await?
}
#[tauri::command]
pub async fn delete_codex_migration_archive(name: String) -> Result<(), String> {
    run_blocking(move || crate::codex_migration::delete_archive(&name)).await?
}
#[tauri::command]
pub async fn inspect_codex_migration(
    app: tauri::AppHandle,
) -> Result<Option<crate::codex_migration::Preview>, String> {
    run_blocking(move || {
        let Some(file) = app
            .dialog()
            .file()
            .add_filter("Codex migration ZIP", &["zip"])
            .blocking_pick_file()
        else {
            return Ok(None);
        };
        crate::codex_migration::inspect(file.into_path().map_err(|error| error.to_string())?)
            .map(Some)
    })
    .await?
}
#[tauri::command]
pub async fn preview_codex_migration_restore(
    token: String,
) -> Result<crate::codex_migration::RestorePreview, String> {
    run_blocking(move || crate::codex_migration::preview_restore(&token)).await?
}
#[tauri::command]
pub async fn restore_codex_migration(
    token: String,
    confirmation: String,
) -> Result<crate::codex_migration::Restored, String> {
    run_blocking(move || crate::codex_migration::restore(&token, &confirmation)).await?
}
#[tauri::command]
pub async fn scan_cleanup() -> Result<crate::cleanup::Scan, String> {
    run_blocking(crate::cleanup::scan).await?
}
#[tauri::command]
pub async fn execute_cleanup(
    token: String,
    confirmation: String,
) -> Result<crate::cleanup::Outcome, String> {
    run_blocking(move || crate::cleanup::execute(&token, &confirmation)).await?
}
#[tauri::command]
pub async fn get_file_transfer_readiness() -> Result<file_transfer::Readiness, String> {
    run_blocking(file_transfer::readiness).await
}
#[tauri::command]
pub async fn list_file_transfer_directory(
    path: String,
) -> Result<file_transfer::DirectoryListing, String> {
    run_blocking(move || file_transfer::list_directory(path)).await?
}
#[tauri::command]
pub async fn preview_file_transfer(
    config: file_transfer::TransferConfig,
) -> Result<file_transfer::CommandPreview, String> {
    run_blocking(move || file_transfer::preview(&config)).await?
}
#[tauri::command]
pub async fn pick_file_transfer_folder(
    app: tauri::AppHandle,
    title: String,
) -> Result<Option<String>, String> {
    Ok(app
        .dialog()
        .file()
        .set_title(title)
        .blocking_pick_folder()
        .and_then(|folder| folder.into_path().ok())
        .map(|path| path.display().to_string()))
}
#[tauri::command]
pub async fn start_file_transfer(
    app: tauri::AppHandle,
    config: file_transfer::TransferConfig,
    analyze: bool,
    progress_total_bytes: Option<u64>,
) -> Result<file_transfer::Started, String> {
    run_blocking(move || file_transfer::start(app, config, analyze, progress_total_bytes)).await?
}
#[tauri::command]
pub async fn cancel_file_transfer() -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(file_transfer::cancel)
        .await
        .map_err(|_| "The cancellation did not complete.")?
}
#[tauri::command]
pub async fn get_file_transfer_history() -> Result<Vec<file_transfer::TransferHistoryEntry>, String>
{
    run_blocking(|| {
        let configuration = config::load().map_err(configuration_error)?;
        file_transfer::list_history(&configuration)
    })
    .await?
}
#[tauri::command]
pub async fn open_file_transfer_log(path: String) -> Result<(), String> {
    run_blocking(move || {
        let configuration = config::load().map_err(configuration_error)?;
        file_transfer::open_log(&configuration, &path)
    })
    .await?
}
#[tauri::command]
pub async fn delete_file_transfer_log(id: String) -> Result<(), String> {
    run_blocking(move || {
        let configuration = config::load().map_err(configuration_error)?;
        file_transfer::delete_log(&configuration, &id)
    })
    .await?
}
#[tauri::command]
pub async fn open_file_transfer_logs_folder() -> Result<(), String> {
    run_blocking(|| {
        let configuration = config::load().map_err(configuration_error)?;
        file_transfer::open_logs_folder(&configuration)
    })
    .await?
}

fn configuration_error(error: ConfigurationError) -> String {
    tracing::warn!("configuration error: {error}");
    error.to_string()
}
#[tauri::command]
pub async fn get_diagnostics() -> Result<codex::DiagnosticsSnapshot, String> {
    run_blocking(|| {
        let configuration = config::load().map_err(configuration_error)?;
        Ok(codex::diagnostics(&configuration))
    })
    .await?
}
#[tauri::command]
pub async fn get_backup_storage() -> Result<codex::BackupStorageOverview, String> {
    run_blocking(|| {
        let configuration = config::load().map_err(configuration_error)?;
        Ok(codex::backup_storage(&configuration))
    })
    .await?
}
#[tauri::command]
pub async fn get_codex_paths() -> Result<codex::CodexPaths, String> {
    run_blocking(|| {
        let configuration = config::load().map_err(configuration_error)?;
        Ok(codex::paths(&configuration))
    })
    .await?
}
#[tauri::command]
pub async fn discover_conversations() -> Result<session_storage::ConversationDiscovery, String> {
    run_blocking(session_storage::discover_sessions).await
}
#[tauri::command]
pub async fn preview_local_delete(
    selected_ids: Vec<String>,
) -> Result<local_delete::DeletePreview, String> {
    run_blocking(move || {
        let configuration = config::load().map_err(configuration_error)?;
        local_delete::preview(&configuration, &selected_ids)
    })
    .await?
}
#[tauri::command]
pub async fn execute_local_delete(
    selected_ids: Vec<String>,
    confirmation: String,
) -> Result<local_delete::DeleteResult, String> {
    run_blocking(move || {
        let configuration = config::load().map_err(configuration_error)?;
        local_delete::execute(&configuration, &selected_ids, &confirmation)
    })
    .await?
}
#[tauri::command]
pub async fn get_configuration() -> Result<AppConfiguration, String> {
    run_blocking(|| config::load().map_err(configuration_error)).await?
}
#[tauri::command]
pub async fn save_configuration(configuration: AppConfiguration) -> Result<(), String> {
    run_blocking(move || config::save(&configuration).map_err(configuration_error)).await?
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        sync::{
            atomic::{AtomicBool, Ordering},
            mpsc, Arc,
        },
        thread,
        time::Duration,
    };

    #[test]
    fn runs_work_on_a_blocking_thread() {
        let (runtime, worker) = tauri::async_runtime::block_on(async {
            (
                thread::current().id(),
                run_blocking(|| thread::current().id()).await.unwrap(),
            )
        });
        assert_ne!(worker, runtime);
    }

    #[test]
    fn serializes_blocking_work() {
        let (entered_tx, entered_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let first = tauri::async_runtime::spawn(run_blocking(move || {
            entered_tx.send(()).unwrap();
            release_rx.recv().unwrap();
        }));
        entered_rx.recv_timeout(Duration::from_secs(1)).unwrap();
        let (waiting_tx, waiting_rx) = mpsc::channel();
        let (second_started_tx, second_started_rx) = mpsc::channel();
        let second_started = Arc::new(AtomicBool::new(false));
        let started = Arc::clone(&second_started);
        let second = tauri::async_runtime::spawn(async move {
            waiting_tx.send(()).unwrap();
            run_blocking(move || {
                started.store(true, Ordering::SeqCst);
                second_started_tx.send(()).unwrap();
            })
            .await
        });
        waiting_rx.recv_timeout(Duration::from_secs(1)).unwrap();
        assert!(second_started_rx
            .recv_timeout(Duration::from_millis(100))
            .is_err());
        assert!(!second_started.load(Ordering::SeqCst));
        release_tx.send(()).unwrap();
        tauri::async_runtime::block_on(async {
            first.await.unwrap().unwrap();
            second.await.unwrap().unwrap();
        });
        assert!(second_started.load(Ordering::SeqCst));
    }

    #[test]
    fn returns_a_stable_error_after_panic_and_reuses_the_gate() {
        let error =
            tauri::async_runtime::block_on(run_blocking(|| panic!("test panic"))).unwrap_err();
        assert_eq!(error, "The background operation did not complete.");
        assert_eq!(tauri::async_runtime::block_on(run_blocking(|| 7)), Ok(7));
    }
}
