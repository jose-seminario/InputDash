use std::io::{Read, Write};
use std::net::{Shutdown, TcpStream, ToSocketAddrs};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BufferbloatResult {
    pub baseline_ping_ms: f64,
    pub loaded_ping_ms: f64,
    pub delta_ms: f64,
    pub delta_pct: f64,
    pub grade: String, // "A+" | "A" | "B" | "C" | "D" | "F"
    pub status: String, // "optimo" | "aceptable" | "critico"
    pub status_label: String,
    pub explanation: String,
    pub sqm_recommendation: String,
}

/// Mide la latencia RTT de conexión TCP hacia un host (probando puertos 443 HTTPS, 53 DNS o 80 HTTP)
fn probe_network_rtt(host: &str, preferred_port: Option<u16>, timeout: Duration) -> Option<f64> {
    let mut ports = Vec::new();
    if let Some(p) = preferred_port {
        ports.push(p);
    }
    for &p in &[443, 53, 80] {
        if !ports.contains(&p) {
            ports.push(p);
        }
    }

    for port in ports {
        let addr_str = format!("{host}:{port}");
        if let Ok(addrs) = addr_str.to_socket_addrs() {
            for addr in addrs {
                let start = Instant::now();
                if let Ok(stream) = TcpStream::connect_timeout(&addr, timeout) {
                    let elapsed = start.elapsed();
                    let _ = stream.shutdown(Shutdown::Both);
                    return Some((elapsed.as_secs_f64() * 1000.0 * 10.0).round() / 10.0);
                }
            }
        }
    }
    None
}

/// Tarea de saturación de ancho de banda downstream descargando datos reales desde Cloudflare Speed CDN
fn spawn_saturation_worker(stop_signal: Arc<AtomicBool>) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let mut buffer = [0u8; 65536];
        let request = b"GET /__down?bytes=25000000 HTTP/1.1\r\nHost: speed.cloudflare.com\r\nUser-Agent: InputDash\r\nConnection: keep-alive\r\n\r\n";

        while !stop_signal.load(Ordering::Relaxed) {
            let addr_str = "speed.cloudflare.com:80";
            if let Ok(addrs) = addr_str.to_socket_addrs() {
                if let Some(addr) = addrs.into_iter().next() {
                    if let Ok(mut stream) = TcpStream::connect_timeout(&addr, Duration::from_millis(2000)) {
                        let _ = stream.set_read_timeout(Some(Duration::from_millis(600)));
                        if stream.write_all(request).is_ok() {
                            while !stop_signal.load(Ordering::Relaxed) {
                                match stream.read(&mut buffer) {
                                    Ok(0) => break,
                                    Ok(_) => {}
                                    Err(_) => break,
                                }
                            }
                        }
                    }
                }
            }
            thread::sleep(Duration::from_millis(25));
        }
    })
}

/// Ejecuta el test de Bufferbloat científico en dos fases:
/// Fase A: 12 pings en reposo absoluto.
/// Fase B: 4 hilos de saturación concurrente durante 6 segundos con pings periódicos.
pub fn run_bufferbloat_test(probe_host_opt: Option<&str>) -> AppResult<BufferbloatResult> {
    let probe_host = probe_host_opt.unwrap_or("1.1.1.1").trim();
    let host_to_use = if probe_host.is_empty() { "1.1.1.1" } else { probe_host };

    // -------------------------------------------------------------------------
    // FASE A: Medición en Reposo (Baseline)
    // -------------------------------------------------------------------------
    let mut baseline_samples = Vec::with_capacity(12);
    for _ in 0..12 {
        if let Some(rtt) = probe_network_rtt(host_to_use, None, Duration::from_millis(1500)) {
            baseline_samples.push(rtt);
        }
        thread::sleep(Duration::from_millis(120));
    }

    if baseline_samples.is_empty() {
        return Err(AppError::NetworkAnalysis(
            format!("No se pudo establecer conexión de prueba con {host_to_use}. Verifica tu conexión a Internet o que la dirección sea accesible.")
        ));
    }

    baseline_samples.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let baseline_mean = baseline_samples.iter().sum::<f64>() / baseline_samples.len() as f64;

    // -------------------------------------------------------------------------
    // FASE B: Medición Bajo Carga (Saturación de Ancho de Banda)
    // -------------------------------------------------------------------------
    struct SaturationStopGuard(Arc<AtomicBool>);
    impl Drop for SaturationStopGuard {
        fn drop(&mut self) {
            self.0.store(true, Ordering::SeqCst);
        }
    }
    let stop_signal = Arc::new(AtomicBool::new(false));
    let _stop_guard = SaturationStopGuard(Arc::clone(&stop_signal));
    let mut workers = Vec::new();

    // Iniciar 4 streams paralelos de descarga continua
    for _ in 0..4 {
        workers.push(spawn_saturation_worker(Arc::clone(&stop_signal)));
    }

    // Dejar 500ms para que la cola del módem/router comience a saturarse
    thread::sleep(Duration::from_millis(500));

    let mut loaded_samples = Vec::new();
    let saturation_duration = Duration::from_millis(5000);
    let start_load = Instant::now();

    while start_load.elapsed() < saturation_duration {
        if let Some(rtt) = probe_network_rtt(host_to_use, None, Duration::from_millis(2000)) {
            loaded_samples.push(rtt);
        }
        thread::sleep(Duration::from_millis(180));
    }

    // Detener hilos de saturación inmediatamente
    stop_signal.store(true, Ordering::Relaxed);
    for w in workers {
        let _ = w.join();
    }

    let loaded_mean = if !loaded_samples.is_empty() {
        loaded_samples.iter().sum::<f64>() / loaded_samples.len() as f64
    } else {
        // Si todas las sondas fallaron bajo carga, el bufferbloat es extremo
        baseline_mean + 150.0
    };

    let delta_ms = (loaded_mean - baseline_mean).max(0.0);
    let delta_pct = if baseline_mean > 0.0 {
        ((loaded_mean - baseline_mean) / baseline_mean) * 100.0
    } else {
        0.0
    };

    // -------------------------------------------------------------------------
    // Clasificación y Grado de Bufferbloat (Estándar CAKE / FQ-CoDel)
    // -------------------------------------------------------------------------
    let (grade, status, status_label, explanation) = if delta_ms <= 5.0 {
        (
            "A+".to_string(),
            "optimo".to_string(),
            "Excepcional (Grado A+ · Cero Bufferbloat)".to_string(),
            format!(
                "El ping se mantuvo impecable: {:.1} ms en reposo vs {:.1} ms bajo descarga continua (+{:.1} ms / +{:.1}%). Tu router gestiona colas de paquetes de forma óptima sin inflar la latencia.",
                baseline_mean, loaded_mean, delta_ms, delta_pct
            ),
        )
    } else if delta_ms <= 15.0 {
        (
            "A".to_string(),
            "optimo".to_string(),
            "Excelente (Grado A · Bufferbloat Mínimo)".to_string(),
            format!(
                "La latencia apenas varió: {:.1} ms en reposo vs {:.1} ms bajo carga (+{:.1} ms). Fluctuación imperceptible en partidas competitivas.",
                baseline_mean, loaded_mean, delta_ms
            ),
        )
    } else if delta_ms <= 30.0 {
        (
            "B".to_string(),
            "aceptable".to_string(),
            "Aceptable (Grado B · Leve Retardo en Cola)".to_string(),
            format!(
                "El ping aumentó de {:.1} ms a {:.1} ms (+{:.1} ms). Puede experimentarse micro-retraso en el registro de disparos cuando otra persona en tu red sube o descarga archivos.",
                baseline_mean, loaded_mean, delta_ms
            ),
        )
    } else if delta_ms <= 60.0 {
        (
            "C".to_string(),
            "critico".to_string(),
            "Deficiente (Grado C · Bufferbloat Notorio)".to_string(),
            format!(
                "El ping se incrementó en +{:.1} ms (+{:.1}%) durante la carga (de {:.1} ms a {:.1} ms). La cola de buffers de tu módem/router infla artificialmente el tiempo de viaje del paquete.",
                delta_ms, delta_pct, baseline_mean, loaded_mean
            ),
        )
    } else {
        (
            "F".to_string(),
            "critico".to_string(),
            "Crítico (Grado F · Bufferbloat Severo)".to_string(),
            format!(
                "Pico masivo de latencia: el ping subió de {:.1} ms a {:.1} ms (+{:.1} ms / +{:.1}%). Esta es la causa principal de disparos no registrados ('disparé primero y no hizo daño') cuando hay tráfico simultáneo en el hogar.",
                baseline_mean, loaded_mean, delta_ms, delta_pct
            ),
        )
    };

    let sqm_recommendation = if delta_ms > 15.0 {
        "Tu router no tiene control de congestión activo y almacena demasiados paquetes en cola. Solución definitiva: Activa Smart Queue Management (SQM) en tu router con el algoritmo CAKE o FQ-CoDel, configurando el ancho de banda al 90-95% de tu velocidad contratada.".to_string()
    } else {
        "Tu red doméstica no sufre de acumulación de cola. No se requiere intervención de SQM en tu enrutador.".to_string()
    };

    Ok(BufferbloatResult {
        baseline_ping_ms: (baseline_mean * 10.0).round() / 10.0,
        loaded_ping_ms: (loaded_mean * 10.0).round() / 10.0,
        delta_ms: (delta_ms * 10.0).round() / 10.0,
        delta_pct: (delta_pct * 10.0).round() / 10.0,
        grade,
        status,
        status_label,
        explanation,
        sqm_recommendation,
    })
}
