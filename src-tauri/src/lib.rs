pub mod ab;
pub mod commands;
pub mod core;
pub mod db;
pub mod diag;
pub mod error;
pub mod profiles;
pub mod sys;

use crate::ab::blind::BlindSessionManager;
use crate::ab::session::AbSessionManager;
use crate::core::catalog::Catalog;
use crate::core::engine::Engine;
use crate::db::Database;
use crate::profiles::manager::ProfileManager;

pub struct AppState {
    pub engine: Result<Engine, String>,
    pub ab_manager: AbSessionManager,
    pub blind_manager: BlindSessionManager,
    pub profile_manager: ProfileManager,
}

impl AppState {
    fn initialize() -> Self {
        let init_res = Catalog::load_embedded()
            .and_then(|catalog| Database::initialize().map(|db| Engine::new(catalog, db)))
            .map_err(|e| e.to_string());

        Self {
            engine: init_res,
            ab_manager: AbSessionManager::new(),
            blind_manager: BlindSessionManager::new(),
            profile_manager: ProfileManager::new(),
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app_state = AppState::initialize();

    tauri::Builder::default()
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            commands::list_tweaks,
            commands::apply_tweak,
            commands::apply_preset,
            commands::revert_tweak,
            commands::revert_all_tweaks,
            commands::simulate_tweaks,
            commands::simulate_preset,
            commands::create_restore_point,
            commands::get_active_backups,
            commands::get_change_logs,
            commands::get_hardware_info,
            commands::start_polling_analysis,
            commands::get_drivers_info,
            commands::start_network_analysis,
            commands::get_usb_topology,
            commands::get_nic_moderation,
            commands::start_frametime_analysis,
            commands::start_dpc_analysis,
            commands::calculate_tel,
            commands::record_ab_baseline,
            commands::apply_and_evaluate_ab,
            commands::get_ab_session,
            commands::reset_ab_session,
            commands::capture_ab_synthetic_samples,
            commands::start_blind_test,
            commands::get_blind_state,
            commands::record_blind_spawn,
            commands::record_blind_click,
            commands::advance_blind_phase,
            commands::complete_blind_test,
            commands::cancel_blind_test,
            commands::evaluate_external_ldat,
            commands::get_running_processes,
            commands::get_cpu_topology,
            commands::get_core_contention,
            commands::get_game_profiles,
            commands::toggle_game_profile,
            commands::update_game_profile,
            commands::get_timer_resolution,
            commands::start_bufferbloat_analysis,
            commands::export_full_report,
        ])
        .run(tauri::generate_context!())
        .expect("error fatal al iniciar la ventana de Tauri");
}
