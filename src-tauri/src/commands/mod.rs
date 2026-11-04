use tauri::State;

use crate::core::engine::{Engine, MutationOutcome, TweakStatusView};
use crate::core::log::{self, ChangeLogEntry};
use crate::error::{AppError, AppResult};
use crate::AppState;

fn require_engine(state: &State<'_, AppState>) -> AppResult<&Engine> {
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
pub fn revert_tweak(tweak_id: String, state: State<'_, AppState>) -> AppResult<MutationOutcome> {
    let engine = require_engine(&state)?;
    engine.revert_tweak(&tweak_id)
}

#[tauri::command]
pub fn get_change_logs(
    limit: Option<u32>,
    state: State<'_, AppState>,
) -> AppResult<Vec<ChangeLogEntry>> {
    let engine = require_engine(&state)?;
    let max_rows = limit.unwrap_or(50).min(500);
    log::list_recent_changes(engine.db(), max_rows)
}
