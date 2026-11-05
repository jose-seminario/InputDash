use serde::{Deserialize, Serialize};
use crate::error::AppResult;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TelInputData {
    pub mouse_avg_interval_ms: Option<f64>,
    pub mouse_polling_hz: Option<f64>,
    pub avg_frametime_ms: Option<f64>,
    pub monitor_refresh_hz: Option<f64>,
    pub network_avg_ping_ms: Option<f64>,
    pub tickrate: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelComponent {
    pub name: String,
    pub description: String,
    pub latency_ms: f64,
    pub is_measured: bool,
    pub formula: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelEnemyAdvantage {
    pub enemy_ping_ms: f64,
    pub enemy_tel_ms: f64,
    pub advantage_ms: f64,
    pub status: String, // "ventaja" | "desventaja" | "neutral"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelReportResult {
    pub total_tel_ms: f64,
    pub is_complete: bool,
    pub missing_components: Vec<String>,
    pub components: Vec<TelComponent>,
    pub enemy_comparisons: Vec<TelEnemyAdvantage>,
    pub verdict: String, // "optimo" | "aceptable" | "critico" | "incompleto"
    pub verdict_label: String,
    pub explanation: String,
}

/// Calculates Total Engagement Latency (TEL) combining mouse polling, frame rendering, monitor scanout, and network transit.
pub fn calculate_tel(input: &TelInputData) -> AppResult<TelReportResult> {
    let mut missing_components = Vec::new();
    let mut components = Vec::new();

    // 1. Polling USB (Mouse)
    let (usb_latency, usb_measured) = if let Some(avg_interval) = input.mouse_avg_interval_ms {
        if avg_interval > 0.0 {
            (avg_interval, true)
        } else {
            (1.0, false)
        }
    } else if let Some(hz) = input.mouse_polling_hz {
        if hz > 0.0 {
            (1000.0 / hz, true)
        } else {
            (1.0, false)
        }
    } else {
        missing_components.push("Polling de Mouse (ejecuta el análisis de sondeo)".to_string());
        (1.0, false)
    };

    components.push(TelComponent {
        name: "Retardo de Sondeo USB (Mouse)".to_string(),
        description: "Intervalo medio entre paquetes de entrada HID entregados al kernel.".to_string(),
        latency_ms: (usb_latency * 100.0).round() / 100.0,
        is_measured: usb_measured,
        formula: if usb_measured {
            "Intervalo medido con QueryPerformanceCounter".to_string()
        } else {
            "Estimado base (1.0 ms para 1000 Hz)".to_string()
        },
    });

    // 2. Frame Time GPU
    let (ft_latency, ft_measured) = if let Some(ft) = input.avg_frametime_ms {
        if ft > 0.0 {
            (ft, true)
        } else {
            (6.94, false)
        }
    } else {
        missing_components.push("Frame Time 3D (ejecuta el análisis de frame time con un juego abierto)".to_string());
        (6.94, false)
    };

    components.push(TelComponent {
        name: "Frame Time del Motor Gráfico (GPU)".to_string(),
        description: "Tiempo que toma al pipeline de renderizado componer un cuadro nuevo.".to_string(),
        latency_ms: (ft_latency * 100.0).round() / 100.0,
        is_measured: ft_measured,
        formula: if ft_measured {
            "Frame time promedio capturado vía ETW DXGI/DxgKrnl".to_string()
        } else {
            "Estimado base (6.94 ms ~ 144 FPS)".to_string()
        },
    });

    // 3. Monitor Scanout
    let (scanout_latency, scanout_measured) = if let Some(hz) = input.monitor_refresh_hz {
        if hz > 0.0 {
            (1000.0 / (hz * 2.0), true)
        } else {
            (3.47, false)
        }
    } else {
        missing_components.push("Tasa de Refresco del Monitor (Hz de pantalla)".to_string());
        (3.47, false)
    };

    components.push(TelComponent {
        name: "Retardo de Scanout del Monitor".to_string(),
        description: "Retardo promedio de exploración vertical hasta la visualización física.".to_string(),
        latency_ms: (scanout_latency * 100.0).round() / 100.0,
        is_measured: scanout_measured,
        formula: "(1000 / Hz_refresco) / 2".to_string(),
    });

    // 4. One-Way Network Ping
    let (net_latency, net_measured) = if let Some(ping) = input.network_avg_ping_ms {
        if ping > 0.0 {
            (ping / 2.0, true)
        } else {
            (15.0, false)
        }
    } else {
        missing_components.push("Latencia de Red (ejecuta el diagnóstico de red ICMP/UDP)".to_string());
        (15.0, false)
    };

    components.push(TelComponent {
        name: "Retardo One-Way de Red al Servidor".to_string(),
        description: "Tiempo de tránsito unidireccional del paquete UDP desde tu NIC hasta el servidor.".to_string(),
        latency_ms: (net_latency * 100.0).round() / 100.0,
        is_measured: net_measured,
        formula: "Ping promedio ICMP / 2".to_string(),
    });

    // 5. Mitad de Tick Interval
    let tickrate = input.tickrate.unwrap_or(64.0).max(1.0);
    let tick_latency = (1000.0 / tickrate) / 2.0;

    components.push(TelComponent {
        name: "Espera de Tick en Servidor de Juego".to_string(),
        description: format!("Retardo medio de sincronización antes de procesar el paquete en tickrate {tickrate:.0}."),
        latency_ms: (tick_latency * 100.0).round() / 100.0,
        is_measured: true,
        formula: format!("(1000 / {tickrate:.0}) / 2"),
    });

    let total_tel_ms: f64 = components.iter().map(|c| c.latency_ms).sum();
    let total_tel_ms = (total_tel_ms * 100.0).round() / 100.0;
    let is_complete = missing_components.is_empty();

    // Ventana de ventaja vs enemigo (comparativa con pings típicos de 15ms, 30ms y 50ms)
    // El oponente estimado usa hardware estándar competitivo (1000Hz mouse, 144FPS, 144Hz monitor, tickrate idéntico)
    let opponent_local_pipeline = 1.0 + 6.94 + 3.47 + tick_latency;
    let enemy_pings = [15.0, 30.0, 50.0];
    let mut enemy_comparisons = Vec::new();

    for enemy_ping in enemy_pings {
        let enemy_tel = opponent_local_pipeline + (enemy_ping / 2.0);
        let enemy_tel_ms = (enemy_tel * 100.0).round() / 100.0;
        let advantage_ms = ((enemy_tel - total_tel_ms) * 100.0).round() / 100.0;

        let status = if advantage_ms > 0.5 {
            "ventaja".to_string()
        } else if advantage_ms < -0.5 {
            "desventaja".to_string()
        } else {
            "neutral".to_string()
        };

        enemy_comparisons.push(TelEnemyAdvantage {
            enemy_ping_ms: enemy_ping,
            enemy_tel_ms,
            advantage_ms,
            status,
        });
    }

    let (verdict, verdict_label, explanation) = if !is_complete {
        (
            "incompleto".to_string(),
            "Diagnóstico TEL Parcial (Incompleto)".to_string(),
            format!(
                "Se han integrado las métricas disponibles calculando un TEL parcial de {:.1} ms. Completa los diagnósticos pendientes ({}) para obtener la certificación completa.",
                total_tel_ms,
                missing_components.join(", ")
            ),
        )
    } else if total_tel_ms < 25.0 {
        (
            "optimo".to_string(),
            "Nivel Profesional de Esports (< 25 ms)".to_string(),
            format!(
                "TEL sobresaliente de {:.1} ms. La cadena integral desde el sensor hasta el servidor de juego ofrece latencia instantánea de competición.",
                total_tel_ms
            ),
        )
    } else if total_tel_ms <= 45.0 {
        (
            "aceptable".to_string(),
            "Nivel Competitivo Estándar (25 - 45 ms)".to_string(),
            format!(
                "TEL competitivo de {:.1} ms. Flujo de respuesta sólido y equilibrado para partidas clasificatorias.",
                total_tel_ms
            ),
        )
    } else {
        (
            "critico".to_string(),
            "Latencia Acumulada Elevada (> 45 ms)".to_string(),
            format!(
                "TEL elevado de {:.1} ms. La acumulación de retardos entre periféricos, monitor y red sitúa tu tiempo de reacción en desventaja frente a oponentes optimizados.",
                total_tel_ms
            ),
        )
    };

    Ok(TelReportResult {
        total_tel_ms,
        is_complete,
        missing_components,
        components,
        enemy_comparisons,
        verdict,
        verdict_label,
        explanation,
    })
}
