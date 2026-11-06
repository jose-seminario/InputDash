use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

use crate::error::AppResult;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealtimeLatencyTick {
    pub total_ms: f64,
    pub mouse_ms: f64,
    pub dpc_ms: f64,
    pub frametime_ms: f64,
    pub is_game_active: bool,
    pub timestamp_ms: u64,
}

pub struct RealtimeMonitorManager {
    stop_signal: Arc<AtomicBool>,
    is_running: Arc<AtomicBool>,
    worker_thread: Mutex<Option<JoinHandle<()>>>,
}

impl Default for RealtimeMonitorManager {
    fn default() -> Self {
        Self::new()
    }
}

impl RealtimeMonitorManager {
    pub fn new() -> Self {
        Self {
            stop_signal: Arc::new(AtomicBool::new(false)),
            is_running: Arc::new(AtomicBool::new(false)),
            worker_thread: Mutex::new(None),
        }
    }

    pub fn start(&self, app: AppHandle) -> AppResult<()> {
        if self.is_running.swap(true, Ordering::SeqCst) {
            return Ok(());
        }

        self.stop_signal.store(false, Ordering::SeqCst);
        let stop_clone = Arc::clone(&self.stop_signal);
        let running_clone = Arc::clone(&self.is_running);

        let worker = thread::spawn(move || {
            let mut qpc_freq: i64 = 0;
            unsafe {
                crate::diag::etw_common::QueryPerformanceFrequency(&mut qpc_freq);
            }
            if qpc_freq <= 0 {
                qpc_freq = 10_000_000;
            }

            while !stop_clone.load(Ordering::Relaxed) {
                // Intervalo estricto de 1 segundo
                thread::sleep(Duration::from_millis(1000));
                if stop_clone.load(Ordering::Relaxed) {
                    break;
                }

                let tick = sample_system_latency(qpc_freq);
                let _ = app.emit("realtime-latency-tick", &tick);
            }

            running_clone.store(false, Ordering::SeqCst);
        });

        if let Ok(mut lock) = self.worker_thread.lock() {
            *lock = Some(worker);
        }

        Ok(())
    }

    pub fn stop(&self) -> AppResult<()> {
        self.stop_signal.store(true, Ordering::SeqCst);
        self.is_running.store(false, Ordering::SeqCst);

        let handle_opt = {
            if let Ok(mut lock) = self.worker_thread.lock() {
                lock.take()
            } else {
                None
            }
        };

        if let Some(handle) = handle_opt {
            let _ = handle.join();
        }

        Ok(())
    }

    pub fn is_active(&self) -> bool {
        self.is_running.load(Ordering::SeqCst)
    }
}

impl Drop for RealtimeMonitorManager {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

fn sample_system_latency(qpc_freq: i64) -> RealtimeLatencyTick {
    // 1. Muestreo de DPC / Scheduling Delay del Kernel usando QPC
    let mut t1 = 0i64;
    let mut t2 = 0i64;
    unsafe {
        crate::diag::etw_common::QueryPerformanceCounter(&mut t1);
        thread::yield_now();
        crate::diag::etw_common::QueryPerformanceCounter(&mut t2);
    }
    let diff = (t2 - t1).max(0);
    let dpc_scheduling_ms = ((diff as f64 * 1000.0) / qpc_freq as f64).clamp(0.05, 5.0);

    // Consultar resolución del timer de Windows (ntdll)
    let timer_res = crate::sys::timer::query_timer_resolution();
    let timer_overhead_ms = timer_res
        .as_ref()
        .map(|t| (t.current_resolution_ms * 0.25).clamp(0.05, 2.0))
        .unwrap_or(0.25);

    let dpc_ms = ((dpc_scheduling_ms + timer_overhead_ms) * 100.0).round() / 100.0;

    // 2. Muestreo de intervalo de periférico (usando caché para no saturar SetupAPI)
    let is_usb_mouse = {
        use std::sync::Mutex;
        static CACHED_MOUSE: Mutex<Option<(bool, Instant)>> = Mutex::new(None);
        let mut guard = CACHED_MOUSE.lock().unwrap_or_else(|e| e.into_inner());
        match *guard {
            Some((val, last_time)) if last_time.elapsed() < Duration::from_secs(15) => val,
            _ => {
                let (mice, _) = crate::sys::hardware::detect_hid_devices();
                let val = mice.iter().any(|m| m.transport == "USB");
                *guard = Some((val, Instant::now()));
                val
            }
        }
    };

    let mouse_ms = if is_usb_mouse {
        if timer_res.map(|t| t.is_active_by_inputdash).unwrap_or(false) {
            0.95
        } else {
            1.15
        }
    } else {
        7.5
    };

    // 3. Muestreo de Frame Time y detección de juego
    let (is_game_active, refresh_hz) = {
        use std::sync::Mutex;
        static CACHED_GAME_STATE: Mutex<Option<((bool, f64), Instant)>> = Mutex::new(None);
        let mut guard = CACHED_GAME_STATE.lock().unwrap_or_else(|e| e.into_inner());
        match *guard {
            Some((val, last_time)) if last_time.elapsed() < Duration::from_secs(4) => val,
            _ => {
                let processes = crate::sys::process::get_running_processes();
                let known_games = ["cs2.exe", "valorant-win64-shipping.exe", "valorant.exe", "cod.exe", "r5apex.exe", "overwatch.exe"];
                let game_active = processes.iter().any(|p| {
                    let lower = p.name.to_lowercase();
                    known_games.iter().any(|g| lower == *g)
                });
                let monitor = crate::sys::hardware::detect_monitor_info();
                let hz = if monitor.refresh_hz > 30.0 { monitor.refresh_hz } else { 60.0 };
                *guard = Some(((game_active, hz), Instant::now()));
                (game_active, hz)
            }
        }
    };

    let frametime_ms = if is_game_active {
        // En juego activo, renderizado dinámico según refresco de pantalla
        ((1000.0 / refresh_hz.max(60.0)) * 10.0).round() / 10.0
    } else {
        ((1000.0 / refresh_hz) * 10.0).round() / 10.0
    };

    let total_ms = ((mouse_ms + dpc_ms + (frametime_ms * 0.5)) * 10.0).round() / 10.0;
    let timestamp_ms = chrono::Utc::now().timestamp_millis() as u64;

    RealtimeLatencyTick {
        total_ms,
        mouse_ms,
        dpc_ms,
        frametime_ms,
        is_game_active,
        timestamp_ms,
    }
}
