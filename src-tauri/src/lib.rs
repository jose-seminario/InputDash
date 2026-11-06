pub mod ab;
pub mod commands;
pub mod core;
pub mod db;
pub mod diag;
pub mod error;
pub mod profiles;
pub mod sys;

use std::sync::Arc;
use tauri::Manager;
use crate::ab::blind::BlindSessionManager;
use crate::ab::session::AbSessionManager;
use crate::core::catalog::Catalog;
use crate::core::engine::Engine;
use crate::db::Database;
use crate::profiles::manager::ProfileManager;

pub struct AppState {
    pub engine: Result<Arc<Engine>, String>,
    pub ab_manager: Arc<AbSessionManager>,
    pub blind_manager: Arc<BlindSessionManager>,
    pub profile_manager: Arc<ProfileManager>,
    pub realtime_monitor: Arc<crate::diag::realtime::RealtimeMonitorManager>,
}

impl AppState {
    fn initialize() -> Self {
        let (init_res, db_opt) = match Catalog::load_embedded()
            .and_then(|catalog| Database::initialize().map(|db| (catalog, Arc::new(db))))
        {
            Ok((catalog, db_arc)) => (
                Ok(Arc::new(Engine::new(catalog, Arc::clone(&db_arc)))),
                Some(db_arc),
            ),
            Err(e) => (Err(e.to_string()), None),
        };

        Self {
            engine: init_res,
            ab_manager: Arc::new(AbSessionManager::new()),
            blind_manager: Arc::new(BlindSessionManager::new()),
            profile_manager: Arc::new(ProfileManager::new(db_opt)),
            realtime_monitor: Arc::new(crate::diag::realtime::RealtimeMonitorManager::new()),
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app_state = AppState::initialize();

    tauri::Builder::default()
        .manage(app_state)
        .setup(|app| {
            let handle = app.handle().clone();
            let state = app.state::<AppState>();
            state.profile_manager.set_app_handle(handle);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_tweaks,
            commands::list_validated_tweaks,
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
            commands::apply_cpu_sets_isolate,
            commands::get_timer_resolution,
            commands::start_bufferbloat_analysis,
            commands::export_full_report,
            commands::start_realtime_monitor,
            commands::stop_realtime_monitor,
        ])
        .run(tauri::generate_context!())
        .expect("error fatal al iniciar la ventana de Tauri");
}
