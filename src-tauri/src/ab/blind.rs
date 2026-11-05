use std::sync::Mutex;
use serde::{Deserialize, Serialize};

use crate::ab::stats::{self, TTestResult};
use crate::core::engine::Engine;
use crate::error::{AppError, AppResult};

#[link(name = "kernel32")]
extern "system" {
    fn QueryPerformanceCounter(lp_performance_count: *mut i64) -> i32;
    fn QueryPerformanceFrequency(lp_frequency: *mut i64) -> i32;
}

fn get_qpc_now() -> i64 {
    let mut count: i64 = 0;
    unsafe { QueryPerformanceCounter(&mut count) };
    count
}

fn get_qpc_freq() -> i64 {
    let mut freq: i64 = 0;
    unsafe { QueryPerformanceFrequency(&mut freq) };
    if freq <= 0 {
        10_000_000
    } else {
        freq
    }
}

/// Generates a pseudo-random boolean with 50/50 probability based on high-resolution timestamp
/// and splitmix64 bit mixing without external crate dependencies.
fn pseudo_random_bool() -> bool {
    let nanos = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(987_654_321);
    let mut z = (nanos as u64).wrapping_add(0x9e3779b97f4a7c15);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
    ((z ^ (z >> 31)) & 1) == 1
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BlindPhase {
    Idle,
    StateA,
    StateB,
    Completed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlindResult {
    pub tweak_active_in: String, // "A" | "B"
    pub state_a_mean: f64,
    pub state_a_std: f64,
    pub state_b_mean: f64,
    pub state_b_std: f64,
    pub baseline_mean: f64,
    pub tweak_mean: f64,
    pub delta_pct: f64,
    pub t_stat: f64,
    pub degrees_of_freedom: f64,
    pub p_value: f64,
    pub is_significant: bool,
    pub verdict: String, // "mejoro" | "placebo" | "empeoro"
    pub verdict_label: String,
    pub explanation: String,
    pub proposed_action: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlindPublicState {
    pub session_id: String,
    pub tweak_id: String,
    pub tweak_name: String,
    pub phase: BlindPhase,
    pub clicks_a_count: usize,
    pub clicks_b_count: usize,
    pub target_clicks: usize,
    pub last_click_ms: Option<f64>,
    pub result: Option<BlindResult>,
}

struct BlindSessionInternal {
    session_id: String,
    tweak_id: String,
    tweak_name: String,
    session_token: String,
    is_tweak_in_a: bool,
    phase: BlindPhase,
    target_clicks: usize,
    state_a_clicks: Vec<f64>,
    state_b_clicks: Vec<f64>,
    last_spawn_qpc: i64,
    qpc_freq: i64,
    last_click_ms: Option<f64>,
    result: Option<BlindResult>,
}

pub struct BlindSessionManager {
    session: Mutex<Option<BlindSessionInternal>>,
}

impl Default for BlindSessionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl BlindSessionManager {
    pub fn new() -> Self {
        Self {
            session: Mutex::new(None),
        }
    }

    pub fn get_state(&self) -> BlindPublicState {
        let guard = self.session.lock().ok();
        if let Some(Some(s)) = guard.as_deref() {
            BlindPublicState {
                session_id: s.session_id.clone(),
                tweak_id: s.tweak_id.clone(),
                tweak_name: s.tweak_name.clone(),
                phase: s.phase,
                clicks_a_count: s.state_a_clicks.len(),
                clicks_b_count: s.state_b_clicks.len(),
                target_clicks: s.target_clicks,
                last_click_ms: s.last_click_ms,
                result: s.result.clone(),
            }
        } else {
            BlindPublicState {
                session_id: String::new(),
                tweak_id: String::new(),
                tweak_name: String::new(),
                phase: BlindPhase::Idle,
                clicks_a_count: 0,
                clicks_b_count: 0,
                target_clicks: 30,
                last_click_ms: None,
                result: None,
            }
        }
    }

    /// Inicia un nuevo Test Ciego aleatorizando si el Estado A o B recibe el tweak.
    pub fn start(
        &self,
        tweak_id: &str,
        tweak_name: &str,
        engine: &Engine,
    ) -> AppResult<BlindPublicState> {
        let mut guard = self.session.lock().map_err(|_| {
            AppError::RequiresAbTest("Mutex de Test Ciego bloqueado.".to_string())
        })?;

        // Si había una sesión previa incompleta con tweak activo, revertir y limpiar
        if let Some(prev) = guard.take() {
            engine.invalidate_session_token(&prev.session_token);
            if (prev.phase == BlindPhase::StateA && prev.is_tweak_in_a)
                || (prev.phase == BlindPhase::StateB && !prev.is_tweak_in_a)
            {
                let _ = engine.revert_tweak(&prev.tweak_id);
            }
        }

        let is_tweak_in_a = pseudo_random_bool();
        let session_id = format!("blind_{}_{}", tweak_id, chrono::Utc::now().timestamp_millis());
        let session_token = format!("tok_{}", session_id);

        engine.register_session_token(&session_token);

        // Si el estado A fue seleccionado para el tweak, aplicarlo inmediatamente bajo transacción segura
        if is_tweak_in_a {
            engine.apply_tweak_with_token(tweak_id, Some(&session_token))?;
        }

        let qpc_freq = get_qpc_freq();
        let new_session = BlindSessionInternal {
            session_id: session_id.clone(),
            tweak_id: tweak_id.to_string(),
            tweak_name: tweak_name.to_string(),
            session_token,
            is_tweak_in_a,
            phase: BlindPhase::StateA,
            target_clicks: 30,
            state_a_clicks: Vec::with_capacity(30),
            state_b_clicks: Vec::with_capacity(30),
            last_spawn_qpc: get_qpc_now(),
            qpc_freq,
            last_click_ms: None,
            result: None,
        };

        let pub_state = BlindPublicState {
            session_id,
            tweak_id: tweak_id.to_string(),
            tweak_name: tweak_name.to_string(),
            phase: BlindPhase::StateA,
            clicks_a_count: 0,
            clicks_b_count: 0,
            target_clicks: 30,
            last_click_ms: None,
            result: None,
        };

        *guard = Some(new_session);
        Ok(pub_state)
    }

    /// Registra el instante exacto (QPC) en que el objetivo de click aparece en pantalla.
    pub fn record_spawn(&self) -> i64 {
        let qpc = get_qpc_now();
        if let Ok(mut guard) = self.session.lock() {
            if let Some(s) = guard.as_mut() {
                s.last_spawn_qpc = qpc;
            }
        }
        qpc
    }

    /// Registra el click del usuario calculando la latencia click-to-reaction mediante QPC.
    /// Devuelve (clicks_completados_en_fase, tiempo_ms, fase_completada).
    pub fn record_click(&self, fallback_ms: Option<f64>) -> AppResult<(usize, f64, bool)> {
        let now_qpc = get_qpc_now();

        let mut guard = self.session.lock().map_err(|_| {
            AppError::RequiresAbTest("Mutex de Test Ciego bloqueado.".to_string())
        })?;

        let s = guard.as_mut().ok_or_else(|| {
            AppError::RequiresAbTest("No hay un Test Ciego activo actualmente.".to_string())
        })?;

        let reaction_ms = if s.last_spawn_qpc > 0 && now_qpc >= s.last_spawn_qpc {
            let delta = now_qpc - s.last_spawn_qpc;
            let ms = (delta as f64 / s.qpc_freq as f64) * 1000.0;
            // Clampear a un rango fisiológico razonable (50 ms a 1500 ms)
            if ms >= 40.0 && ms <= 2000.0 {
                (ms * 10.0).round() / 10.0
            } else {
                fallback_ms.unwrap_or(220.0)
            }
        } else {
            fallback_ms.unwrap_or(220.0)
        };

        s.last_click_ms = Some(reaction_ms);

        match s.phase {
            BlindPhase::StateA => {
                s.state_a_clicks.push(reaction_ms);
                let count = s.state_a_clicks.len();
                let phase_completed = count >= s.target_clicks;
                Ok((count, reaction_ms, phase_completed))
            }
            BlindPhase::StateB => {
                s.state_b_clicks.push(reaction_ms);
                let count = s.state_b_clicks.len();
                let phase_completed = count >= s.target_clicks;
                Ok((count, reaction_ms, phase_completed))
            }
            _ => Err(AppError::RequiresAbTest(
                "La sesión actual no está en una fase de captura de clicks activa.".to_string(),
            )),
        }
    }

    /// Avanza de la Fase A a la Fase B conmutando el estado del tweak de forma transparente.
    pub fn advance_to_state_b(&self, engine: &Engine) -> AppResult<BlindPublicState> {
        let mut guard = self.session.lock().map_err(|_| {
            AppError::RequiresAbTest("Mutex de Test Ciego bloqueado.".to_string())
        })?;

        let s = guard.as_mut().ok_or_else(|| {
            AppError::RequiresAbTest("No hay un Test Ciego activo para avanzar.".to_string())
        })?;

        if s.phase != BlindPhase::StateA {
            return Err(AppError::RequiresAbTest(
                "Solo se puede avanzar a la Fase B desde la Fase A.".to_string(),
            ));
        }

        // Conmutación transparente del tweak:
        // Si el estado A tuvo el tweak, ahora se revierte (el estado B será baseline sin tweak)
        // Si el estado A fue baseline, ahora se aplica el tweak con el session_token activo
        if s.is_tweak_in_a {
            engine.revert_tweak(&s.tweak_id)?;
        } else {
            engine.apply_tweak_with_token(&s.tweak_id, Some(&s.session_token))?;
        }

        s.phase = BlindPhase::StateB;
        s.last_spawn_qpc = get_qpc_now();

        Ok(BlindPublicState {
            session_id: s.session_id.clone(),
            tweak_id: s.tweak_id.clone(),
            tweak_name: s.tweak_name.clone(),
            phase: BlindPhase::StateB,
            clicks_a_count: s.state_a_clicks.len(),
            clicks_b_count: 0,
            target_clicks: s.target_clicks,
            last_click_ms: None,
            result: None,
        })
    }

    /// Finaliza el Test Ciego, calcula el veredicto mediante t-Student (Welch),
    /// revierte el tweak para dejar el sistema seguro e invalida el token de sesión.
    pub fn complete_and_evaluate(&self, engine: &Engine) -> AppResult<BlindResult> {
        let mut guard = self.session.lock().map_err(|_| {
            AppError::RequiresAbTest("Mutex de Test Ciego bloqueado.".to_string())
        })?;

        let s = guard.as_mut().ok_or_else(|| {
            AppError::RequiresAbTest("No hay un Test Ciego para completar.".to_string())
        })?;

        if s.state_b_clicks.len() < 5 || s.state_a_clicks.len() < 5 {
            return Err(AppError::RequiresAbTest(
                "Se requieren al menos 5 clicks registrados en cada estado para evaluar la significancia."
                    .to_string(),
            ));
        }

        // Si el estado B tenía el tweak aplicado, revertir inmediatamente por seguridad
        if !s.is_tweak_in_a {
            let _ = engine.revert_tweak(&s.tweak_id);
        }

        // Invalidar el token de sesión en el motor
        engine.invalidate_session_token(&s.session_token);

        let a_mean = stats::mean(&s.state_a_clicks).unwrap_or(0.0);
        let a_std = stats::std_dev(&s.state_a_clicks).unwrap_or(0.0);
        let b_mean = stats::mean(&s.state_b_clicks).unwrap_or(0.0);
        let b_std = stats::std_dev(&s.state_b_clicks).unwrap_or(0.0);

        // Separar Baseline de Tweak según dónde estuvo activo el ajuste
        let (baseline_clicks, tweak_clicks, active_letter) = if s.is_tweak_in_a {
            (&s.state_b_clicks, &s.state_a_clicks, "A".to_string())
        } else {
            (&s.state_a_clicks, &s.state_b_clicks, "B".to_string())
        };

        let base_mean = stats::mean(baseline_clicks).unwrap_or(0.0);
        let twk_mean = stats::mean(tweak_clicks).unwrap_or(0.0);

        let t_test = stats::welch_t_test(baseline_clicks, tweak_clicks).unwrap_or(TTestResult {
            t_stat: 0.0,
            degrees_of_freedom: 1.0,
            p_value: 1.0,
            is_significant: false,
        });

        // En latencia de reacción: menor tiempo = mejor
        let delta_pct = if base_mean > 0.0 {
            ((twk_mean - base_mean) / base_mean) * 100.0
        } else {
            0.0
        };

        let (verdict, label, explanation, proposed) = if !t_test.is_significant {
            (
                "placebo".to_string(),
                "Sin Efecto Real (Placebo / Ruido Estadístico)".to_string(),
                format!(
                    "El tweak estuvo activo en el Estado {active_letter}. Tu tiempo medio con tweak fue de {:.1} ms vs {:.1} ms sin tweak (diferencia: {:.2}%). El p-value de {:.4} (>= 0.05) demuestra que la variación se debe al azar humano y no a una aceleración real del sistema.",
                    twk_mean, base_mean, delta_pct.abs(), t_test.p_value
                ),
                "El sistema restauró el estado original para evitar cambios placebo.".to_string(),
            )
        } else if delta_pct <= -2.0 {
            (
                "mejoro".to_string(),
                "Mejora Verificada con Significancia Estadística".to_string(),
                format!(
                    "¡El tweak demostró beneficio real! Estuvo activo en el Estado {active_letter}. Tu tiempo de reacción disminuyó un {:.2}% con significancia estadística (de {:.1} ms a {:.1} ms, p = {:.4} < 0.05, t = {:.2}).",
                    delta_pct.abs(), base_mean, twk_mean, t_test.p_value, t_test.t_stat
                ),
                "Puedes aplicar este tweak de forma permanente desde la pestaña Optimizar.".to_string(),
            )
        } else if delta_pct >= 2.0 {
            (
                "empeoro".to_string(),
                "Degradación del Rendimiento Confirmada".to_string(),
                format!(
                    "El tweak empeoró la respuesta del sistema. Estuvo activo en el Estado {active_letter}. Tu tiempo de reacción aumentó un {:.2}% con significancia estadística (de {:.1} ms a {:.1} ms, p = {:.4} < 0.05).",
                    delta_pct, base_mean, twk_mean, t_test.p_value
                ),
                "El tweak se revirtió automáticamente a su configuración original.".to_string(),
            )
        } else {
            (
                "placebo".to_string(),
                "Diferencia Despreciable (< 2% de impacto)".to_string(),
                format!(
                    "El tweak estuvo activo en el Estado {active_letter}. La diferencia registrada ({:.2}%) es inferior al umbral mínimo de percepción competitiva.",
                    delta_pct
                ),
                "Se recomienda mantener el ajuste en su valor predeterminado.".to_string(),
            )
        };

        let result = BlindResult {
            tweak_active_in: active_letter,
            state_a_mean: (a_mean * 10.0).round() / 10.0,
            state_a_std: (a_std * 100.0).round() / 100.0,
            state_b_mean: (b_mean * 10.0).round() / 10.0,
            state_b_std: (b_std * 100.0).round() / 100.0,
            baseline_mean: (base_mean * 10.0).round() / 10.0,
            tweak_mean: (twk_mean * 10.0).round() / 10.0,
            delta_pct: (delta_pct * 100.0).round() / 100.0,
            t_stat: t_test.t_stat,
            degrees_of_freedom: t_test.degrees_of_freedom,
            p_value: t_test.p_value,
            is_significant: t_test.is_significant,
            verdict,
            verdict_label: label,
            explanation,
            proposed_action: proposed,
        };

        s.result = Some(result.clone());
        s.phase = BlindPhase::Completed;

        Ok(result)
    }

    /// Cancela el Test Ciego a mitad del proceso, revierte cualquier cambio inmediatamente
    /// e invalida el token de sesión para dejar el sistema seguro.
    pub fn cancel(&self, engine: &Engine) -> AppResult<()> {
        let mut guard = self.session.lock().map_err(|_| {
            AppError::RequiresAbTest("Mutex de Test Ciego bloqueado.".to_string())
        })?;

        if let Some(s) = guard.take() {
            // Revertir si estaba en la fase activa con el tweak
            if (s.phase == BlindPhase::StateA && s.is_tweak_in_a)
                || (s.phase == BlindPhase::StateB && !s.is_tweak_in_a)
            {
                let _ = engine.revert_tweak(&s.tweak_id);
            }
            engine.invalidate_session_token(&s.session_token);
        }

        Ok(())
    }
}
