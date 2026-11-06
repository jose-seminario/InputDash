use std::sync::Mutex;
use serde::{Deserialize, Serialize};

use crate::ab::stats::{self, SampleSummary, TTestResult};
use crate::core::engine::Engine;
use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AbSessionState {
    Idle,
    BaselineReady,
    TweakApplied,
    Completed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AbVerdict {
    Mejoro,
    Placebo,
    Empeoro,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbSession {
    pub id: String,
    pub tweak_id: String,
    pub tweak_name: String,
    pub session_token: String,
    pub metric: String, // "frametime" | "dpc" | "tel" | "synthetic_jitter"
    pub state: AbSessionState,
    pub baseline_samples: Vec<f64>,
    pub test_samples: Vec<f64>,
    pub baseline_summary: Option<SampleSummary>,
    pub test_summary: Option<SampleSummary>,
    pub t_test: Option<TTestResult>,
    pub delta_pct: Option<f64>,
    pub verdict: Option<AbVerdict>,
    pub verdict_label: Option<String>,
    pub explanation: Option<String>,
    pub proposed_action: Option<String>,
    pub can_rollback: bool,
}

impl AbSession {
    pub fn new(tweak_id: String, tweak_name: String, metric: String) -> Self {
        let id = format!(
            "ab_{}_{}",
            tweak_id,
            chrono::Utc::now().timestamp_millis()
        );
        let session_token = format!("tok_{}", id);
        Self {
            id,
            tweak_id,
            tweak_name,
            session_token,
            metric,
            state: AbSessionState::Idle,
            baseline_samples: Vec::new(),
            test_samples: Vec::new(),
            baseline_summary: None,
            test_summary: None,
            t_test: None,
            delta_pct: None,
            verdict: None,
            verdict_label: None,
            explanation: None,
            proposed_action: None,
            can_rollback: false,
        }
    }
}

pub struct AbSessionManager {
    current_session: Mutex<Option<AbSession>>,
}

impl Default for AbSessionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl AbSessionManager {
    pub fn new() -> Self {
        Self {
            current_session: Mutex::new(None),
        }
    }

    pub fn get_session(&self) -> Option<AbSession> {
        let guard = self.current_session.lock().ok()?;
        guard.clone()
    }

    pub fn reset(&self, engine: Option<&Engine>) {
        if let Ok(mut guard) = self.current_session.lock() {
            if let Some(session) = guard.take() {
                if let Some(eng) = engine {
                    if session.state == AbSessionState::TestRunning
                        || session.verdict == Some(AbVerdict::Placebo)
                        || session.verdict == Some(AbVerdict::Empeoro)
                    {
                        let _ = eng.revert_tweak(&session.tweak_id);
                    }
                    eng.invalidate_session_token(&session.session_token);
                }
            }
        }
    }

    /// Paso 1: Inicia o actualiza el estado Baseline con las muestras capturadas.
    /// Registra el session_token en el motor para autorizar tweaks avanzados durante la prueba.
    pub fn record_baseline(
        &self,
        tweak_id: &str,
        tweak_name: &str,
        metric: &str,
        samples: Vec<f64>,
        engine: &Engine,
    ) -> AppResult<AbSession> {
        if samples.len() < 5 {
            return Err(AppError::RequiresAbTest(
                "Se requieren al menos 5 muestras para una captura de Baseline estadísticamente válida."
                    .to_string(),
            ));
        }

        let summary = stats::compute_summary(&samples).ok_or_else(|| {
            AppError::RequiresAbTest("No se pudo calcular el resumen estadístico del Baseline.".to_string())
        })?;

        let mut session = AbSession::new(tweak_id.to_string(), tweak_name.to_string(), metric.to_string());
        session.baseline_samples = samples;
        session.baseline_summary = Some(summary);
        session.state = AbSessionState::BaselineReady;

        // Registrar token para habilitar temporalmente la aplicación del tweak avanzado en el motor
        engine.register_session_token(&session.session_token);

        let mut guard = self.current_session.lock().map_err(|_| {
            AppError::RequiresAbTest("Mutex de sesión A/B envenenado.".to_string())
        })?;
        if let Some(prev) = guard.as_ref() {
            if prev.session_token != session.session_token {
                engine.invalidate_session_token(&prev.session_token);
            }
        }
        *guard = Some(session.clone());

        Ok(session)
    }

    /// Paso 2: Aplica el tweak mediante la transacción segura y registra las muestras de Test.
    /// Calcula t-test, percentiles y emite el veredicto.
    /// IMPORTANTE: Si el veredicto es Placebo o Empeoró, el motor NO revierte automáticamente;
    /// entrega el veredicto a la UI para que el usuario tome la decisión informada.
    pub fn apply_and_evaluate(
        &self,
        samples: Vec<f64>,
        engine: &Engine,
    ) -> AppResult<AbSession> {
        if samples.len() < 5 {
            return Err(AppError::RequiresAbTest(
                "Se requieren al menos 5 muestras en el estado de prueba para evaluar la significancia."
                    .to_string(),
            ));
        }

        let mut guard = self.current_session.lock().map_err(|_| {
            AppError::RequiresAbTest("Mutex de sesión A/B envenenado.".to_string())
        })?;

        let session = guard.as_mut().ok_or_else(|| {
            AppError::RequiresAbTest(
                "No hay una sesión A/B activa. Inicia primero la captura de Baseline.".to_string(),
            )
        })?;

        if session.state != AbSessionState::BaselineReady && session.state != AbSessionState::TweakApplied {
            return Err(AppError::RequiresAbTest(
                "El estado actual no permite evaluar. Se requiere una captura de Baseline previa."
                    .to_string(),
            ));
        }

        // Aplicación del tweak mediante el motor transaccional con el token de sesión activo
        if session.state == AbSessionState::BaselineReady {
            let _outcome = engine.apply_tweak_ab(&session.tweak_id, Some(&session.session_token))?;
            session.state = AbSessionState::TweakApplied;
            session.can_rollback = true;
        }

        let test_summary = stats::compute_summary(&samples).ok_or_else(|| {
            AppError::RequiresAbTest("Error calculando estadísticas del grupo de prueba.".to_string())
        })?;

        let baseline_summary = session.baseline_summary.as_ref().ok_or_else(|| {
            AppError::RequiresAbTest("Resumen de Baseline ausente.".to_string())
        })?;

        let t_test = stats::welch_t_test(&session.baseline_samples, &samples).unwrap_or(TTestResult {
            t_stat: 0.0,
            degrees_of_freedom: 1.0,
            p_value: 1.0,
            is_significant: false,
        });

        // En métricas de latencia / frametime / jitter: menor es mejor
        let b_med = baseline_summary.median;
        let t_med = test_summary.median;
        let delta_pct = if b_med > 0.0 {
            ((t_med - b_med) / b_med) * 100.0
        } else {
            0.0
        };

        let (verdict, label, explanation, proposed) = evaluate_verdict(
            delta_pct,
            &t_test,
            &session.metric,
            baseline_summary,
            &test_summary,
        );

        session.test_samples = samples;
        session.test_summary = Some(test_summary);
        session.t_test = Some(t_test);
        session.delta_pct = Some((delta_pct * 100.0).round() / 100.0);
        session.verdict = Some(verdict);
        session.verdict_label = Some(label);
        session.explanation = Some(explanation);
        session.proposed_action = Some(proposed);
        session.state = AbSessionState::Completed;

        if verdict == AbVerdict::Mejoro {
            let _ = engine.record_ab_validation(&session.tweak_id, &session.metric, "mejoro");
        }

        // Al finalizar la prueba, invalidar el token para que el tweak no pueda modificarse fuera de sesión
        engine.invalidate_session_token(&session.session_token);

        Ok(session.clone())
    }
}

fn evaluate_verdict(
    delta_pct: f64,
    t_test: &TTestResult,
    _metric: &str,
    b_sum: &SampleSummary,
    t_sum: &SampleSummary,
) -> (AbVerdict, String, String, String) {
    if !t_test.is_significant {
        (
            AbVerdict::Placebo,
            "Sin Efecto Real (Placebo / Ruido Estadístico)".to_string(),
            format!(
                "La diferencia observada de {:.2}% (mediana: {:.2} ms vs {:.2} ms) no es estadísticamente distinguible del ruido del sistema (p-value = {:.4} >= 0.05). No hay evidencia empírica de mejora.",
                delta_pct.abs(),
                b_sum.median,
                t_sum.median,
                t_test.p_value
            ),
            "Se propone revertir el tweak para mantener el sistema libre de modificaciones innecesarias.".to_string(),
        )
    } else if delta_pct <= -1.5 {
        (
            AbVerdict::Mejoro,
            "Mejora Verificada con Significancia Estadística".to_string(),
            format!(
                "Reducción estadísticamente significativa de {:.2}% en la latencia/frame time (mediana: {:.2} ms → {:.2} ms, p = {:.4} < 0.05, t = {:.2}). El tweak demuestra beneficio real en este equipo.",
                delta_pct.abs(),
                b_sum.median,
                t_sum.median,
                t_test.p_value,
                t_test.t_stat
            ),
            "Conservar el tweak optimizado en el sistema.".to_string(),
        )
    } else if delta_pct >= 1.5 {
        (
            AbVerdict::Empeoro,
            "Degradación del Rendimiento Confirmada".to_string(),
            format!(
                "La latencia o tiempo de cuadro aumentó un {:.2}% con significancia estadística (mediana: {:.2} ms → {:.2} ms, p = {:.4} < 0.05). El ajuste resulta perjudicial para esta configuración.",
                delta_pct,
                b_sum.median,
                t_sum.median,
                t_test.p_value
            ),
            "Se propone revertir el tweak inmediatamente a su estado original.".to_string(),
        )
    } else {
        (
            AbVerdict::Placebo,
            "Diferencia Despreciable (< 1.5%)".to_string(),
            format!(
                "La diferencia es de solo {:.2}% (mediana: {:.2} ms vs {:.2} ms), inferior al umbral mínimo de percepción competitiva.",
                delta_pct,
                b_sum.median,
                t_sum.median
            ),
            "Se propone revertir el tweak a su estado original.".to_string(),
        )
    }
}
