use crate::{
    backup, codex,
    config::{self, AppConfiguration, ConfigurationError},
    file_transfer, local_delete, session_storage,
};
use tauri_plugin_dialog::DialogExt;

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
pub async fn get_xampp_readiness() -> Result<crate::xampp::Readiness, String> {
    run_blocking(crate::xampp::readiness).await
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
pub async fn list_file_transfer_directory(path: String) -> Result<file_transfer::DirectoryListing, String> {
    run_blocking(move || file_transfer::list_directory(path)).await?
}
#[tauri::command]
pub async fn preview_file_transfer(config: file_transfer::TransferConfig) -> Result<file_transfer::CommandPreview, String> {
    run_blocking(move || file_transfer::preview(&config)).await?
}
#[tauri::command]
pub async fn pick_file_transfer_folder(app: tauri::AppHandle, title: String) -> Result<Option<String>, String> {
    Ok(app.dialog().file().set_title(title).blocking_pick_folder().and_then(|folder| folder.into_path().ok()).map(|path| path.display().to_string()))
}
#[tauri::command]
pub async fn start_file_transfer(app: tauri::AppHandle, config: file_transfer::TransferConfig, analyze: bool) -> Result<file_transfer::Started, String> {
    run_blocking(move || file_transfer::start(app, config, analyze)).await?
}
#[tauri::command]
pub async fn cancel_file_transfer() -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(file_transfer::cancel).await.map_err(|_| "The cancellation did not complete.")?
}
#[tauri::command]
pub async fn get_file_transfer_history() -> Result<Vec<file_transfer::TransferHistoryEntry>, String> {
    run_blocking(|| { let configuration = config::load().map_err(configuration_error)?; file_transfer::list_history(&configuration) }).await?
}
#[tauri::command]
pub async fn open_file_transfer_log(path: String) -> Result<(), String> {
    run_blocking(move || { let configuration = config::load().map_err(configuration_error)?; file_transfer::open_log(&configuration, &path) }).await?
}
#[tauri::command]
pub async fn open_file_transfer_logs_folder() -> Result<(), String> {
    run_blocking(|| { let configuration = config::load().map_err(configuration_error)?; file_transfer::open_logs_folder(&configuration) }).await?
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
pub async fn preview_backup(
    selected_ids: Vec<String>,
) -> Result<backup::BackupPreview, backup::BackupError> {
    run_blocking(move || {
        let configuration = config::load().map_err(configuration_error)?;
        backup::preview(&configuration, &selected_ids)
    })
    .await
    .map_err(backup::BackupError::from_message)?
    .map_err(backup::BackupError::from_message)
}
#[tauri::command]
pub async fn create_backup(
    selected_ids: Vec<String>,
) -> Result<backup::BackupResult, backup::BackupError> {
    run_blocking(move || {
        let configuration = config::load().map_err(configuration_error)?;
        backup::create(&configuration, &selected_ids)
    })
    .await
    .map_err(backup::BackupError::from_message)?
    .map_err(backup::BackupError::from_message)
}
#[tauri::command]
pub async fn inspect_backup_archive(
    app: tauri::AppHandle,
) -> Result<Option<backup::ArchiveInspection>, String> {
    run_blocking(move || {
        let Some(selected) = app
            .dialog()
            .file()
            .set_title("Choose a Codex backup archive")
            .add_filter("ZIP archives", &["zip"])
            .blocking_pick_file()
        else {
            return Ok(None);
        };
        let path = selected
            .into_path()
            .map_err(|_| "The selected archive does not have a readable local path.")?;
        let mut inspection = backup::inspect(&path)?;
        if inspection.validation.valid {
            inspection.restore_token = Some(backup::register_restore_archive(path));
        }
        Ok(Some(inspection))
    })
    .await?
}
#[tauri::command]
pub async fn preview_restore(
    restore_token: String,
    selected_ids: Vec<String>,
) -> Result<backup::RestorePreview, backup::BackupError> {
    run_blocking(move || {
        let configuration = config::load().map_err(configuration_error)?;
        backup::preview_restore(&configuration, &restore_token, &selected_ids)
    })
    .await
    .map_err(backup::BackupError::from_message)?
    .map_err(backup::BackupError::from_message)
}
#[tauri::command]
pub async fn restore_archive(
    restore_token: String,
    selected_ids: Vec<String>,
) -> Result<backup::RestoreResult, backup::BackupError> {
    run_blocking(move || {
        let configuration = config::load().map_err(configuration_error)?;
        backup::restore(&configuration, &restore_token, &selected_ids)
    })
    .await
    .map_err(backup::BackupError::from_message)?
    .map_err(backup::BackupError::from_message)
}
#[tauri::command]
pub async fn get_restore_history(
) -> Result<Vec<crate::restore_history::RestoreHistoryEntry>, backup::BackupError> {
    run_blocking(|| {
        let configuration = config::load().map_err(configuration_error)?;
        crate::restore_history::list(&configuration)
    })
    .await
    .map_err(backup::BackupError::from_message)?
    .map_err(backup::BackupError::from_message)
}
#[tauri::command]
pub async fn preview_local_delete(
    selected_ids: Vec<String>,
) -> Result<local_delete::DeletePreview, backup::BackupError> {
    run_blocking(move || {
        let configuration = config::load().map_err(configuration_error)?;
        local_delete::preview(&configuration, &selected_ids)
    })
    .await
    .map_err(backup::BackupError::from_message)?
    .map_err(backup::BackupError::from_message)
}
#[tauri::command]
pub async fn execute_local_delete(
    selected_ids: Vec<String>,
    confirmation: String,
) -> Result<local_delete::DeleteResult, backup::BackupError> {
    run_blocking(move || {
        let configuration = config::load().map_err(configuration_error)?;
        local_delete::execute(&configuration, &selected_ids, &confirmation)
    })
    .await
    .map_err(backup::BackupError::from_message)?
    .map_err(backup::BackupError::from_message)
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
