mod beyond_compare;
mod cleanup;
mod codex;
mod codex_content;
mod codex_environment;
mod codex_migration;
mod commands;
mod compression;
mod config;
mod environment;
mod file_transfer;
mod fs_safety;
mod local_delete;
mod logging;
mod personal_bundle;
mod platform;
mod session_storage;
mod sourcetree;
mod sourcetree_config;
mod xampp;
mod xampp_domains;

use commands::{
    discover_conversations, execute_local_delete, get_backup_storage, get_codex_paths,
    get_configuration, get_diagnostics, preview_local_delete, save_configuration,
};

pub fn run() {
    logging::init();
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            get_diagnostics,
            get_backup_storage,
            get_codex_paths,
            discover_conversations,
            preview_local_delete,
            execute_local_delete,
            get_configuration,
            save_configuration,
            commands::preview_environment,
            commands::create_environment,
            commands::inspect_environment,
            commands::preview_environment_restore,
            commands::restore_environment,
            commands::get_codex_migration_overview,
            commands::preview_codex_migration,
            commands::create_codex_migration,
            commands::list_codex_migration_archives,
            commands::open_codex_migration_archive,
            commands::delete_codex_migration_archive,
            commands::inspect_codex_migration_archive,
            commands::inspect_codex_migration,
            commands::preview_codex_migration_restore,
            commands::restore_codex_migration,
            commands::stop_codex_processes,
            commands::scan_cleanup,
            commands::execute_cleanup,
            commands::get_skills,
            commands::import_skill,
            commands::export_skill,
            commands::get_pets,
            commands::install_pet,
            commands::remove_pet,
            commands::get_file_transfer_readiness,
            commands::preview_file_transfer,
            commands::list_file_transfer_directory,
            commands::pick_file_transfer_folder,
            commands::start_file_transfer,
            commands::cancel_file_transfer,
            commands::get_file_transfer_history,
            commands::open_file_transfer_log,
            commands::delete_file_transfer_log,
            commands::open_file_transfer_logs_folder,
            commands::get_compression_readiness,
            commands::scan_compression_source,
            commands::pick_compression_folder,
            commands::preview_compression,
            commands::start_compression,
            commands::cancel_compression,
            commands::get_beyond_compare_readiness,
            commands::preview_beyond_compare,
            commands::create_beyond_compare_bundle,
            commands::inspect_beyond_compare_bundle,
            commands::preview_beyond_compare_recovery,
            commands::recover_beyond_compare,
            commands::get_sourcetree_readiness,
            commands::preview_sourcetree,
            commands::create_sourcetree_bundle,
            commands::inspect_sourcetree_bundle,
            commands::preview_sourcetree_recovery,
            commands::recover_sourcetree,
            commands::preview_sourcetree_config,
            commands::create_sourcetree_config_bundle,
            commands::open_sourcetree_config_bundle_folder,
            commands::list_sourcetree_config_archives,
            commands::open_sourcetree_config_archive,
            commands::delete_sourcetree_config_archive,
            commands::inspect_sourcetree_config_archive,
            commands::inspect_sourcetree_config_bundle,
            commands::preview_sourcetree_config_recovery,
            commands::recover_sourcetree_config,
            commands::delete_sourcetree_config_bundle,
            commands::get_xampp_domains,
            commands::set_xampp_backup_retention,
            commands::prune_xampp_backups,
            commands::open_xampp_domain,
            commands::open_xampp_domain_folder,
            commands::set_xampp_installation,
            commands::initialize_xampp_domains,
            commands::save_xampp_domain,
            commands::delete_xampp_domain,
            commands::export_xampp_ca,
            commands::elevate_xampp_manager,
            commands::get_xampp_readiness,
            commands::preview_xampp,
            commands::create_xampp_bundle,
            commands::inspect_xampp_bundle,
            commands::preview_xampp_recovery,
            commands::recover_xampp,
            commands::get_codex_environment_overview,
            commands::save_codex_environment,
            commands::regenerate_codex_launcher,
            commands::remove_codex_launcher,
            commands::add_codex_launcher_dir_to_user_path,
            commands::get_codex_environment_instructions,
            commands::update_codex_environment_instructions,
            commands::delete_codex_environment,
            commands::open_codex_environment_home,
            commands::open_codex_environment_agents,
            commands::run_codex_environment_action
        ])
        .run(tauri::generate_context!())
        .expect("error while running Dev Companion");
}
