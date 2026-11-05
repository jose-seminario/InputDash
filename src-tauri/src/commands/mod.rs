use tauri::State;

use crate::ab::blind::{BlindPublicState, BlindResult, BlindSessionManager};
use crate::ab::session::{AbSession, AbSessionManager};
use crate::ab::stats::{self, ExternalLdatInput, ExternalLdatResult};
use crate::core::backup::BackupSnapshot;
use crate::core::engine::{
    Engine, MutationOutcome, PresetOutcome, RevertAllOutcome, SimulationItem, TweakStatusView,
};
use crate::core::log::{self, ChangeLogEntry};
use crate::diag::dpc::{self, DpcAnalysisResult};
use crate::diag::frametime::{self, FrametimeAnalysisResult};
use crate::diag::network::{self, NetworkAnalysisResult};
use crate::diag::polling::{self, PollingAnalysisResult};
use crate::diag::report::{self, TelInputData, TelReportResult};
use crate::error::{AppError, AppResult};
use crate::sys::drivers::{self, DriversInfo, NicModerationInfo};
use crate::sys::hardware::{self, HardwareDiagnosticInfo, UsbTopologyInfo};
use crate::AppState;

fn require_engine<'a>(state: &'a State<'a, AppState>) -> AppResult<&'a Engine> {
    match &state.engine {
        Ok(engine) => Ok(engine),
        Err(init_err) => Err(AppError::Database(format!(
            "Error de inicialización del motor: {init_err}"
        ))),
    }
}

fn require_ab_manager<'a>(state: &'a State<'a, AppState>) -> &'a AbSessionManager {
    &state.ab_manager
}

fn require_blind_manager<'a>(state: &'a State<'a, AppState>) -> &'a BlindSessionManager {
    &state.blind_manager
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
    engine.revert_all_tweaks()
}

#[tauri::command]
pub fn simulate_tweaks(
    tweak_ids: Vec<String>,
    state: State<'_, AppState>,
) -> AppResult<Vec<SimulationItem>> {
    let engine = require_engine(&state)?;
    let refs: Vec<&str> = tweak_ids.iter().map(|s| s.as_str()).collect();
    engine.simulate_tweaks(&refs)
}

#[tauri::command]
pub fn simulate_preset(
    preset_id: String,
    state: State<'_, AppState>,
) -> AppResult<Vec<SimulationItem>> {
    let engine = require_engine(&state)?;
    engine.simulate_preset(&preset_id)
}

#[tauri::command]
pub fn create_restore_point(state: State<'_, AppState>) -> AppResult<String> {
    let engine = require_engine(&state)?;
    let outcome = engine.create_manual_restore_point("InputDash Manual Checkpoint")?;
    Ok(format!(
        "Punto de restauración procesado: {} (Secuencia #{:?})",
        outcome.detail, outcome.sequence_number
    ))
}

#[tauri::command]
pub fn get_active_backups(state: State<'_, AppState>) -> AppResult<Vec<BackupSnapshot>> {
    let engine = require_engine(&state)?;
    engine.list_active_backups()
}

#[tauri::command]
pub fn get_change_logs(
    limit: Option<usize>,
    state: State<'_, AppState>,
) -> AppResult<Vec<ChangeLogEntry>> {
    let engine = require_engine(&state)?;
    log::get_recent_logs(engine.db(), limit.unwrap_or(100))
}

#[tauri::command]
pub fn get_hardware_info() -> AppResult<HardwareDiagnosticInfo> {
    hardware::get_hardware_diagnostic_info()
}

#[tauri::command]
pub async fn start_polling_analysis() -> AppResult<PollingAnalysisResult> {
    tauri::async_runtime::spawn_blocking(move || {
        polling::run_polling_analysis()
    })
    .await
    .map_err(|e| AppError::PollingAnalysis(format!("Error en tarea asíncrona de polling: {e}")))?
}

#[tauri::command]
pub fn get_drivers_info() -> AppResult<DriversInfo> {
    drivers::get_drivers_info()
}

#[tauri::command]
pub async fn start_network_analysis(host: Option<String>) -> AppResult<NetworkAnalysisResult> {
    tauri::async_runtime::spawn_blocking(move || {
        network::run_network_analysis(host.as_deref())
    })
    .await
    .map_err(|e| AppError::NetworkAnalysis(format!("Error en tarea de red: {e}")))?
}

#[tauri::command]
pub fn get_usb_topology() -> AppResult<UsbTopologyInfo> {
    hardware::get_usb_topology_info()
}

#[tauri::command]
pub fn get_nic_moderation() -> AppResult<NicModerationInfo> {
    drivers::get_nic_moderation_info()
}

#[tauri::command]
pub async fn start_frametime_analysis() -> AppResult<FrametimeAnalysisResult> {
    tauri::async_runtime::spawn_blocking(move || {
        frametime::run_frametime_analysis()
    })
    .await
    .map_err(|e| AppError::FrametimeAnalysis(format!("Error en tarea de frame time: {e}")))?
}

#[tauri::command]
pub async fn start_dpc_analysis() -> AppResult<DpcAnalysisResult> {
    tauri::async_runtime::spawn_blocking(move || {
        dpc::run_dpc_analysis()
    })
    .await
    .map_err(|e| AppError::DpcAnalysis(format!("Error en tarea de análisis DPC: {e}")))?
}

#[tauri::command]
pub fn calculate_tel(input_data: TelInputData) -> AppResult<TelReportResult> {
    report::calculate_tel(&input_data)
}

// ---------------------------------------------------------------------------
// A/B Testing Commands (Etapa 4.2 / Etapa 5)
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn record_ab_baseline(
    tweak_id: String,
    tweak_name: String,
    metric: String,
    samples: Vec<f64>,
    state: State<'_, AppState>,
) -> AppResult<AbSession> {
    let engine = require_engine(&state)?;
    let ab = require_ab_manager(&state);
    ab.record_baseline(&tweak_id, &tweak_name, &metric, samples, engine)
}

#[tauri::command]
pub fn apply_and_evaluate_ab(
    samples: Vec<f64>,
    state: State<'_, AppState>,
) -> AppResult<AbSession> {
    let engine = require_engine(&state)?;
    let ab = require_ab_manager(&state);
    ab.apply_and_evaluate(samples, engine)
}

#[tauri::command]
pub fn get_ab_session(state: State<'_, AppState>) -> Option<AbSession> {
    let ab = require_ab_manager(&state);
    ab.get_session()
}

#[tauri::command]
pub fn reset_ab_session(state: State<'_, AppState>) {
    let engine = require_engine(&state).ok();
    let ab = require_ab_manager(&state);
    ab.reset(engine);
}

#[tauri::command]
pub fn capture_ab_synthetic_samples(count: Option<usize>) -> Vec<f64> {
    let sample_count = count.unwrap_or(60).clamp(10, 500);
    let mut samples = Vec::with_capacity(sample_count);
    let mut qpc_freq: i64 = 0;
    unsafe {
        crate::diag::etw_common::QueryPerformanceFrequency(&mut qpc_freq);
    }
    if qpc_freq <= 0 {
        qpc_freq = 10_000_000;
    }

    for _ in 0..sample_count {
        let mut t1: i64 = 0;
        let mut t2: i64 = 0;
        unsafe {
            crate::diag::etw_common::QueryPerformanceCounter(&mut t1);
            std::thread::yield_now();
            crate::diag::etw_common::QueryPerformanceCounter(&mut t2);
        }
        let diff = t2 - t1;
        if diff > 0 {
            let ms = (diff as f64 * 1000.0) / qpc_freq as f64;
            samples.push((ms * 1000.0).round() / 1000.0);
        }
    }
    samples
}

// ---------------------------------------------------------------------------
// Double-Blind (Test Ciego) Commands (Etapa 5)
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn start_blind_test(
    tweak_id: String,
    tweak_name: String,
    state: State<'_, AppState>,
) -> AppResult<BlindPublicState> {
    let engine = require_engine(&state)?;
    let blind = require_blind_manager(&state);
    blind.start(&tweak_id, &tweak_name, engine)
}

#[tauri::command]
pub fn get_blind_state(state: State<'_, AppState>) -> BlindPublicState {
    let blind = require_blind_manager(&state);
    blind.get_state()
}

#[tauri::command]
pub fn record_blind_spawn(state: State<'_, AppState>) -> i64 {
    let blind = require_blind_manager(&state);
    blind.record_spawn()
}

#[tauri::command]
pub fn record_blind_click(
    reaction_ms: Option<f64>,
    state: State<'_, AppState>,
) -> AppResult<(usize, f64, bool)> {
    let blind = require_blind_manager(&state);
    blind.record_click(reaction_ms)
}

#[tauri::command]
pub fn advance_blind_phase(state: State<'_, AppState>) -> AppResult<BlindPublicState> {
    let engine = require_engine(&state)?;
    let blind = require_blind_manager(&state);
    blind.advance_to_state_b(engine)
}

#[tauri::command]
pub fn complete_blind_test(state: State<'_, AppState>) -> AppResult<BlindResult> {
    let engine = require_engine(&state)?;
    let blind = require_blind_manager(&state);
    blind.complete_and_evaluate(engine)
}

#[tauri::command]
pub fn cancel_blind_test(state: State<'_, AppState>) -> AppResult<()> {
    let engine = require_engine(&state)?;
    let blind = require_blind_manager(&state);
    blind.cancel(engine)
}

// ---------------------------------------------------------------------------
// Importación Externa LDAT / Reflex (Etapa 5)
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn evaluate_external_ldat(input: ExternalLdatInput) -> AppResult<ExternalLdatResult> {
    let count_a = input.count_a.unwrap_or(30).max(2);
    let count_b = input.count_b.unwrap_or(30).max(2);

    let t_test = stats::welch_t_test_from_stats(
        input.mean_a,
        input.std_dev_a,
        count_a,
        input.mean_b,
        input.std_dev_b,
        count_b,
    )
    .ok_or_else(|| {
        AppError::RequiresAbTest("Datos insuficientes para calcular la prueba t de Welch.".to_string())
    })?;

    let delta_pct = if input.mean_a > 0.0 {
        ((input.mean_b - input.mean_a) / input.mean_a) * 100.0
    } else {
        0.0
    };

    let (verdict, label, explanation) = if !t_test.is_significant {
        (
            "placebo".to_string(),
            "Sin Efecto Real (Placebo / Ruido Estadístico)".to_string(),
            format!(
                "La diferencia entre la medición A ({:.2} ms) y B ({:.2} ms) es de {:.2}%, pero con p = {:.4} (>= 0.05) no existe significancia estadística. No se descarta que la variación sea producto del ruido instrumental.",
                input.mean_a, input.mean_b, delta_pct.abs(), t_test.p_value
            ),
        )
    } else if delta_pct <= -1.5 {
        (
            "mejoro".to_string(),
            "Mejora Verificada con Significancia Estadística".to_string(),
            format!(
                "Reducción estadísticamente significativa de {:.2}% en la latencia (de {:.2} ms a {:.2} ms, p = {:.4} < 0.05, t = {:.2}). El instrumento físico confirma beneficio medible.",
                delta_pct.abs(), input.mean_a, input.mean_b, t_test.p_value, t_test.t_stat
            ),
        )
    } else if delta_pct >= 1.5 {
        (
            "empeoro".to_string(),
            "Degradación del Rendimiento Confirmada".to_string(),
            format!(
                "La latencia aumentó un {:.2}% con significancia estadística (de {:.2} ms a {:.2} ms, p = {:.4} < 0.05). La prueba instrumental desaconseja mantener este tweak.",
                delta_pct, input.mean_a, input.mean_b, t_test.p_value
            ),
        )
    } else {
        (
            "placebo".to_string(),
            "Diferencia Despreciable (< 1.5%)".to_string(),
            format!(
                "Diferencia de solo {:.2}% entre mediciones, dentro del margen de error intrínseco de instrumentos ópticos.",
                delta_pct
            ),
        )
    };

    Ok(ExternalLdatResult {
        t_test,
        delta_pct: (delta_pct * 100.0).round() / 100.0,
        verdict,
        verdict_label: label,
        explanation,
    })
}
