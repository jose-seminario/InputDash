pub mod commands;
pub mod core;
pub mod db;
pub mod error;
pub mod sys;

use crate::core::catalog::Catalog;
use crate::core::engine::Engine;
use crate::db::Database;

pub struct AppState {
    pub engine: Result<Engine, String>,
}

impl AppState {
    fn initialize() -> Self {
        let init_res = Catalog::load_embedded()
            .and_then(|catalog| Database::initialize().map(|db| Engine::new(catalog, db)))
            .map_err(|e| e.to_string());

        Self { engine: init_res }
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
            commands::revert_tweak,
            commands::get_change_logs,
        ])
        .run(tauri::generate_context!())
        .expect("error fatal al iniciar la ventana de Tauri");
}
