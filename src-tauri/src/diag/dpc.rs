use core::ffi::c_void;
use std::collections::HashMap;
use std::ptr;
use std::sync::Arc;
use std::time::Duration;
use serde::{Deserialize, Serialize};

use crate::diag::etw_common::{
    stop_stale_session, CloseTrace, EnumDeviceDrivers, EtwSessionGuard,
    GetDeviceDriverBaseNameW, OpenTraceW, ProcessTrace, QueryPerformanceFrequency,
    StartTraceW, TracePropertiesBuffer, EVENT_RECORD, EVENT_TRACE_FLAG_DPC,
    EVENT_TRACE_FLAG_DRIVER, EVENT_TRACE_FLAG_INTERRUPT, EVENT_TRACE_LOGFILEW,
    INVALID_PROCESSTRACE_HANDLE, KERNEL_LOGGER_NAME, PROCESS_TRACE_MODE_EVENT_RECORD,
    PROCESS_TRACE_MODE_REAL_TIME, SYSTEM_TRACE_CONTROL_GUID,
};
use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriverDpcInfo {
    pub driver_name: String,
    pub max_dpc_us: f64,
    pub avg_dpc_us: f64,
    pub total_calls: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DpcAnalysisResult {
    pub duration_seconds: f64,
    pub total_events: u64,
    pub highest_dpc_us: f64,
    pub top_drivers: Vec<DriverDpcInfo>,
    pub status: String, // "optimo" | "aceptable" | "critico"
    pub status_label: String,
    pub explanation: String,
}

#[derive(Default)]
struct DriverAccum {
    max_us: f64,
    total_us: f64,
    count: u64,
}

struct DpcCollector {
    qpc_freq: i64,
    drivers: Vec<(u64, String)>,
    accum: HashMap<String, DriverAccum>,
    total_events: u64,
}

impl DpcCollector {
    fn resolve_driver(&self, routine: u64) -> String {
        if routine == 0 {
            return "ntoskrnl.exe".to_string();
        }

        match self.drivers.binary_search_by_key(&routine, |d| d.0) {
            Ok(idx) => self.drivers[idx].1.clone(),
            Err(idx) => {
                if idx > 0 {
                    self.drivers[idx - 1].1.clone()
                } else {
                    "kernel_core.sys".to_string()
                }
            }
        }
    }
}

unsafe extern "system" fn dpc_event_callback(record: *mut EVENT_RECORD) {
    if record.is_null() || (*record).UserContext.is_null() {
        return;
    }

    let collector = &mut *((*record).UserContext as *mut DpcCollector);
    let len = (*record).UserDataLength as usize;

    if len >= 16 && !(*record).UserData.is_null() {
        let initial_time = ptr::read_unaligned((*record).UserData as *const u64);
        let routine = ptr::read_unaligned(((*record).UserData as *const u64).add(1));

        let timestamp = (*record).EventHeader.TimeStamp;
        if initial_time > 0 && timestamp >= initial_time as i64 {
            let diff = timestamp - initial_time as i64;
            let duration_us = (diff as f64 * 1_000_000.0) / collector.qpc_freq as f64;

            // Filter reasonable DPC duration range: 0.1 µs to 100,000 µs (100 ms)
            if duration_us >= 0.1 && duration_us <= 100_000.0 {
                let driver_name = collector.resolve_driver(routine);
                let entry = collector.accum.entry(driver_name).or_default();
                if duration_us > entry.max_us {
                    entry.max_us = duration_us;
                }
                entry.total_us += duration_us;
                entry.count += 1;
                collector.total_events += 1;
            }
        }
    }
}

/// Enumerates loaded kernel device drivers and maps their base addresses to .sys file names.
fn load_driver_map() -> Vec<(u64, String)> {
    let mut drivers = Vec::new();
    let mut needed: u32 = 0;
    let mut base_addrs = [ptr::null_mut::<c_void>(); 1024];

    let res = unsafe {
        EnumDeviceDrivers(
            base_addrs.as_mut_ptr(),
            (base_addrs.len() * std::mem::size_of::<*mut c_void>()) as u32,
            &mut needed,
        )
    };

    if res != 0 && needed > 0 {
        let count = (needed as usize / std::mem::size_of::<*mut c_void>()).min(base_addrs.len());
        for i in 0..count {
            let base = base_addrs[i];
            if base.is_null() {
                continue;
            }
            let mut name_buf = [0u16; 256];
            let n = unsafe { GetDeviceDriverBaseNameW(base, name_buf.as_mut_ptr(), 256) };
            if n > 0 {
                let name = String::from_utf16_lossy(&name_buf[..n as usize]);
                drivers.push((base as u64, name));
            }
        }
    }

    drivers.sort_by_key(|d| d.0);
    drivers
}

/// Runs a controlled 10-second ETW kernel trace capturing DPC and ISR events across system drivers.
pub fn run_dpc_analysis() -> AppResult<DpcAnalysisResult> {
    // 1. Clean up any leftover kernel session
    stop_stale_session(KERNEL_LOGGER_NAME);

    let mut qpc_freq: i64 = 0;
    unsafe { QueryPerformanceFrequency(&mut qpc_freq) };
    if qpc_freq <= 0 {
        qpc_freq = 10_000_000;
    }

    let drivers = load_driver_map();

    let kernel_name_wide: Vec<u16> = KERNEL_LOGGER_NAME
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();

    let mut props_buf = TracePropertiesBuffer::new(KERNEL_LOGGER_NAME);
    let p_props = props_buf.as_mut_ptr();

    unsafe {
        (*p_props).Wnode.Guid = SYSTEM_TRACE_CONTROL_GUID;
        (*p_props).EnableFlags =
            EVENT_TRACE_FLAG_DPC | EVENT_TRACE_FLAG_INTERRUPT | EVENT_TRACE_FLAG_DRIVER;
    }

    let mut session_handle: u64 = 0;
    let start_status = unsafe {
        StartTraceW(
            &mut session_handle,
            kernel_name_wide.as_ptr(),
            p_props,
        )
    };

    if start_status != 0 {
        return Err(AppError::DpcAnalysis(format!(
            "No se pudo iniciar el rastreador de kernel ETW (código Win32 {start_status}). Se requieren privilegios de Administrador para capturar eventos DPC/ISR de drivers."
        )));
    }

    let mut collector = DpcCollector {
        qpc_freq,
        drivers,
        accum: HashMap::new(),
        total_events: 0,
    };

    let mut logfile = unsafe { std::mem::zeroed::<EVENT_TRACE_LOGFILEW>() };
    let mut logfile_name_wide = kernel_name_wide.clone();
    logfile.LoggerName = logfile_name_wide.as_mut_ptr();
    logfile.ProcessTraceMode = PROCESS_TRACE_MODE_REAL_TIME | PROCESS_TRACE_MODE_EVENT_RECORD;
    logfile.EventRecordCallback = Some(dpc_event_callback);
    logfile.Context = &mut collector as *mut _ as *mut c_void;

    let consumer_handle = unsafe { OpenTraceW(&mut logfile) };
    if consumer_handle == INVALID_PROCESSTRACE_HANDLE {
        stop_stale_session(KERNEL_LOGGER_NAME);
        return Err(AppError::DpcAnalysis(
            "Fallo al abrir el consumidor ETW de kernel para análisis DPC/ISR.".to_string(),
        ));
    }

    // 2. Wrap in RAII guard
    let guard = Arc::new(EtwSessionGuard::new(
        session_handle,
        KERNEL_LOGGER_NAME,
        consumer_handle,
    ));

    // 3. Worker thread
    let worker_guard = Arc::clone(&guard);
    let worker_thread = std::thread::spawn(move || {
        let handle = worker_guard.consumer_handle;
        unsafe {
            let _ = ProcessTrace(&handle, 1, ptr::null(), ptr::null());
        }
    });

    // 4. Strict 10-second capture window
    std::thread::sleep(Duration::from_secs(10));

    // 5. Stop session & wait for worker
    guard.stop();
    let _ = worker_thread.join();

    if collector.accum.is_empty() {
        return Err(AppError::DpcAnalysis(
            "El sistema no permitió capturar eventos de Kernel DPC/ISR en la ventana de 10s. Asegúrate de ejecutar InputDash como Administrador.".to_string(),
        ));
    }

    // 6. Aggregate top drivers
    let mut driver_list: Vec<DriverDpcInfo> = collector
        .accum
        .into_iter()
        .map(|(name, acc)| {
            let avg = if acc.count > 0 {
                acc.total_us / acc.count as f64
            } else {
                0.0
            };
            DriverDpcInfo {
                driver_name: name,
                max_dpc_us: (acc.max_us * 10.0).round() / 10.0,
                avg_dpc_us: (avg * 10.0).round() / 10.0,
                total_calls: acc.count,
            }
        })
        .collect();

    // Sort by maximum DPC latency descending
    driver_list.sort_by(|a, b| b.max_dpc_us.partial_cmp(&a.max_dpc_us).unwrap_or(std::cmp::Ordering::Equal));

    let top_drivers: Vec<DriverDpcInfo> = driver_list.into_iter().take(5).collect();
    let highest_dpc_us = top_drivers.first().map(|d| d.max_dpc_us).unwrap_or(0.0);

    let (status, status_label, explanation) = if highest_dpc_us < 500.0 {
        (
            "optimo".to_string(),
            "Excelente Latencia DPC/ISR (< 500 µs)".to_string(),
            format!(
                "El pico máximo de DPC registrado fue de {:.1} µs ({}). El sistema procesa interrupciones sin degradar la tasa de cuadros ni causar stuttering de audio/video.",
                highest_dpc_us,
                top_drivers.first().map(|d| d.driver_name.as_str()).unwrap_or("ninguno")
            ),
        )
    } else if highest_dpc_us < 1000.0 {
        (
            "aceptable".to_string(),
            "Latencia DPC Moderada (500 - 1000 µs)".to_string(),
            format!(
                "Pico máximo de {:.1} µs registrado en '{}'. La respuesta es aceptable para juego competitivo, aunque se recomienda verificar configuraciones de ahorro de energía y moderación de interrupciones.",
                highest_dpc_us,
                top_drivers.first().map(|d| d.driver_name.as_str()).unwrap_or("driver")
            ),
        )
    } else {
        (
            "critico".to_string(),
            "Latencia DPC Crítica (> 1000 µs)".to_string(),
            format!(
                "Pico crítico de {:.1} µs registrado en '{}'. Las interrupciones de este controlador bloquean el hilo del procesador por más de 1 ms, provocando caídas de cuadros y micro-congelamientos.",
                highest_dpc_us,
                top_drivers.first().map(|d| d.driver_name.as_str()).unwrap_or("driver")
            ),
        )
    };

    Ok(DpcAnalysisResult {
        duration_seconds: 10.0,
        total_events: collector.total_events,
        highest_dpc_us,
        top_drivers,
        status,
        status_label,
        explanation,
    })
}
