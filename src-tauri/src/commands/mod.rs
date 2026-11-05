use tauri::State;

use crate::core::backup::BackupSnapshot;
use crate::core::engine::{
    Engine, MutationOutcome, PresetOutcome, RevertAllOutcome, SimulationItem, TweakStatusView,
};
use crate::core::log::{self, ChangeLogEntry};
use crate::diag::polling::{self, PollingAnalysisResult};
use crate::error::{AppError, AppResult};
use crate::sys::hardware::{self, HardwareDiagnosticInfo};
use crate::AppState;

fn require_engine<'a>(state: &'a State<'a, AppState>) -> AppResult<&'a Engine> {
    match &state.engine {
        Ok(engine) => Ok(engine),
        Err(init_err) => Err(AppError::Database(format!(
            "Error de inicialización del motor: {init_err}"
        ))),
    }
}

#[tauri::command]
pub fn list_tweaks(state: State<'_, AppState>) -> AppResult<Vec<TweakStatusView>> {
    let engine = require_engine(&state)?;
    Ok(engine.list_tweaks_with_status())
}

#[tauri::command]
pub fn apply_tweak(tweak_id: String, state: State<'_, AppState>) -> AppResult<MutationOutcome> {
    let engine = require_engine(&state)?;
    engine.apply_tweak(&tweak_id)
}

#[tauri::command]
pub fn apply_preset(preset_id: String, state: State<'_, AppState>) -> AppResult<PresetOutcome> {
    let engine = require_engine(&state)?;
    engine.apply_preset(&preset_id)
}

#[tauri::command]
pub fn revert_tweak(tweak_id: String, state: State<'_, AppState>) -> AppResult<MutationOutcome> {
    let engine = require_engine(&state)?;
    engine.revert_tweak(&tweak_id)
}

#[tauri::command]
pub fn revert_all_tweaks(state: State<'_, AppState>) -> AppResult<RevertAllOutcome> {
    let engine = require_engine(&state)?;
    engine.revert_all()
}

#[tauri::command]
pub fn simulate_tweaks(
    tweak_ids: Vec<String>,
    state: State<'_, AppState>,
) -> AppResult<Vec<SimulationItem>> {
    let engine = require_engine(&state)?;
    engine.simulate_tweaks(&tweak_ids)
}

#[tauri::command]
pub fn simulate_preset(
    preset_id: String,
    state: State<'_, AppState>,
) -> AppResult<Vec<SimulationItem>> {
    let engine = require_engine(&state)?;
    let ids: Vec<String> = Engine::preset_tweak_ids(&preset_id)?
        .iter()
        .map(|s| (*s).to_string())
        .collect();
    engine.simulate_tweaks(&ids)
}

#[tauri::command]
pub fn create_restore_point(state: State<'_, AppState>) -> AppResult<String> {
    let engine = require_engine(&state)?;
    engine.create_manual_restore_point()
}

#[tauri::command]
pub fn get_active_backups(state: State<'_, AppState>) -> AppResult<Vec<BackupSnapshot>> {
    let engine = require_engine(&state)?;
    engine.list_active_backups()
}

#[tauri::command]
pub fn get_change_logs(
    limit: Option<u32>,
    state: State<'_, AppState>,
) -> AppResult<Vec<ChangeLogEntry>> {
    let engine = require_engine(&state)?;
    let max_rows = limit.unwrap_or(100).min(500);
    log::list_recent_changes(engine.db(), max_rows)
}

#[tauri::command]
pub fn get_hardware_info() -> AppResult<HardwareDiagnosticInfo> {
    Ok(hardware::get_hardware_diagnostic_info())
}

#[tauri::command]
pub async fn start_polling_analysis() -> AppResult<PollingAnalysisResult> {
    tauri::async_runtime::spawn_blocking(move || {
        polling::run_polling_analysis()
    })
    .await
    .map_err(|e| AppError::PollingAnalysis(format!("Error en tarea de polling: {e}")))?
}
