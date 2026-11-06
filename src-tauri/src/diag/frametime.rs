use core::ffi::c_void;
use std::collections::HashMap;
use std::ptr;
use std::time::Duration;
use serde::{Deserialize, Serialize};
use windows::core::GUID;

use crate::diag::etw_common::{
    stop_stale_session, EnableTraceEx2, EtwSessionGuard, OpenTraceW,
    ProcessTrace, QueryPerformanceFrequency, StartTraceW, TracePropertiesBuffer,
    EVENT_CONTROL_CODE_ENABLE_PROVIDER, EVENT_RECORD, EVENT_TRACE_LOGFILEW,
    INVALID_PROCESSTRACE_HANDLE, PROCESS_TRACE_MODE_EVENT_RECORD,
    PROCESS_TRACE_MODE_REAL_TIME, TRACE_LEVEL_INFORMATION,
};
use crate::error::{AppError, AppResult};

const FRAMETIME_SESSION_NAME: &str = "InputDash_Frametime_Trace";

// Provider Microsoft-Windows-DxgKrnl
const DXGKRNL_GUID: GUID = GUID {
    data1: 0x802ec45a,
    data2: 0x1e99,
    data3: 0x4b83,
    data4: [0x99, 0x20, 0x87, 0xc9, 0x82, 0x77, 0xba, 0x9d],
};

// Provider Microsoft-Windows-DXGI
const DXGI_GUID: GUID = GUID {
    data1: 0xca11c036,
    data2: 0x0102,
    data3: 0x4503,
    data4: [0xa1, 0x07, 0xb2, 0xd3, 0x42, 0x37, 0x3d, 0xa4],
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrametimeAnalysisResult {
    pub process_id: u32,
    pub total_frames: usize,
    pub duration_seconds: f64,
    pub avg_frametime_ms: f64,
    pub min_frametime_ms: f64,
    pub max_frametime_ms: f64,
    pub avg_fps: f64,
    pub fps_1pct_low: f64,
    pub fps_01pct_low: f64,
    pub status: String, // "optimo" | "aceptable" | "critico"
    pub status_label: String,
    pub explanation: String,
    pub samples_ms: Vec<f64>,
}

struct FrametimeCollector {
    presents: HashMap<u32, Vec<i64>>,
}

unsafe extern "system" fn frametime_event_callback(record: *mut EVENT_RECORD) {
    if record.is_null() || (*record).UserContext.is_null() {
        return;
    }

    let id = (*record).EventHeader.EventDescriptor.Id;
    let opcode = (*record).EventHeader.EventDescriptor.Opcode;
    // DXGI: Present_Start = 42; DxgKrnl: Present = Opcode 1 or 2
    if id != 42 && opcode != 1 && opcode != 2 {
        return;
    }

    let collector = &mut *((*record).UserContext as *mut FrametimeCollector);
    let pid = (*record).EventHeader.ProcessId;
    let ts = (*record).EventHeader.TimeStamp;

    if pid > 4 && ts > 0 {
        collector.presents.entry(pid).or_default().push(ts);
    }
}

/// Runs a controlled 10-second ETW trace capturing Microsoft-Windows-DxgKrnl and DXGI present events.
pub fn run_frametime_analysis() -> AppResult<FrametimeAnalysisResult> {
    // 1. Clean up any leftover session from previous aborted runs
    stop_stale_session(FRAMETIME_SESSION_NAME);

    let mut qpc_freq: i64 = 0;
    unsafe { QueryPerformanceFrequency(&mut qpc_freq) };
    if qpc_freq <= 0 {
        qpc_freq = 10_000_000;
    }

    // Declare collector BEFORE guard so reverse drop order joins ProcessTrace BEFORE freeing collector
    let mut collector = Box::new(FrametimeCollector {
        presents: HashMap::new(),
    });

    let session_name_wide: Vec<u16> = FRAMETIME_SESSION_NAME
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();

    let mut props_buf = TracePropertiesBuffer::new(FRAMETIME_SESSION_NAME);
    let mut session_handle: u64 = 0;

    let start_status = unsafe {
        StartTraceW(
            &mut session_handle,
            session_name_wide.as_ptr(),
            props_buf.as_mut_ptr(),
        )
    };

    if start_status != 0 {
        return Err(AppError::FrametimeAnalysis(format!(
            "No se pudo iniciar la sesión ETW (código Win32 {start_status}). Se requieren privilegios de Administrador para rastreo ETW."
        )));
    }

    // 2. Register RAII guard immediately after StartTraceW succeeds (no Arc cycle)
    let mut guard = EtwSessionGuard::new(session_handle, FRAMETIME_SESSION_NAME);

    // 3. Enable Providers: DxgKrnl and DXGI
    unsafe {
        let _ = EnableTraceEx2(
            session_handle,
            &DXGKRNL_GUID,
            EVENT_CONTROL_CODE_ENABLE_PROVIDER,
            TRACE_LEVEL_INFORMATION,
            0,
            0,
            0,
            ptr::null(),
        );

        let _ = EnableTraceEx2(
            session_handle,
            &DXGI_GUID,
            EVENT_CONTROL_CODE_ENABLE_PROVIDER,
            TRACE_LEVEL_INFORMATION,
            0,
            0,
            0,
            ptr::null(),
        );
    }

    let mut logfile = unsafe { std::mem::zeroed::<EVENT_TRACE_LOGFILEW>() };
    let mut logfile_name_wide = session_name_wide.clone();
    logfile.LoggerName = logfile_name_wide.as_mut_ptr();
    logfile.ProcessTraceMode = PROCESS_TRACE_MODE_REAL_TIME | PROCESS_TRACE_MODE_EVENT_RECORD;
    logfile.EventRecordCallback = Some(frametime_event_callback);
    logfile.Context = (&mut *collector) as *mut FrametimeCollector as *mut c_void;

    let consumer_handle = unsafe { OpenTraceW(&mut logfile) };
    if consumer_handle == INVALID_PROCESSTRACE_HANDLE {
        return Err(AppError::FrametimeAnalysis(
            "Fallo al abrir el consumidor ETW para análisis de frame time.".to_string(),
        ));
    }

    // 4. Spawn worker thread to pump ProcessTrace using Copy handle (no Arc to guard)
    let worker_thread = std::thread::spawn(move || {
        let handle = consumer_handle;
        unsafe {
            let _ = ProcessTrace(&handle, 1, ptr::null(), ptr::null());
        }
    });
    guard.attach_consumer(consumer_handle, worker_thread);

    // 5. Strict 10-second capture window
    std::thread::sleep(Duration::from_secs(10));

    // 6. Stop session & wait for worker before reading collector
    guard.stop_and_join();

    // 7. Process timestamps, prioritizing actual game processes over desktop compositors
    let procs = crate::sys::process::get_running_processes();
    let proc_map: HashMap<u32, String> = procs.into_iter().map(|p| (p.pid, p.name.to_lowercase())).collect();
    let ignored_names = [
        "dwm.exe",
        "explorer.exe",
        "msedgewebview2.exe",
        "inputdash.exe",
        "taskmgr.exe",
        "searchhost.exe",
        "shellexperiencehost.exe",
        "startmenuexperiencehost.exe",
    ];

    let best_pid_entry = collector
        .presents
        .iter()
        .filter(|(pid, _)| {
            if let Some(name) = proc_map.get(pid) {
                !ignored_names.iter().any(|ign| name == ign)
            } else {
                true
            }
        })
        .max_by_key(|(_, timestamps)| timestamps.len())
        .or_else(|| collector.presents.iter().max_by_key(|(_, timestamps)| timestamps.len()));

    let (active_pid, timestamps) = match best_pid_entry {
        Some((pid, ts)) if ts.len() >= 10 => (*pid, ts),
        _ => {
            return Err(AppError::FrametimeAnalysis(
                "No se detectaron frames 3D. Abre un juego antes de analizar.".to_string(),
            ));
        }
    };

    let mut deltas: Vec<f64> = Vec::with_capacity(timestamps.len());
    for i in 1..timestamps.len() {
        let diff = timestamps[i] - timestamps[i - 1];
        if diff > 0 {
            let ms = (diff as f64 * 1000.0) / qpc_freq as f64;
            // Filter realistic frame interval range: 0.2ms (5000 FPS) to 500ms (2 FPS)
            if ms >= 0.2 && ms <= 500.0 {
                deltas.push(ms);
            }
        }
    }

    if deltas.len() < 10 {
        return Err(AppError::FrametimeAnalysis(
            "No se detectaron frames 3D. Abre un juego antes de analizar.".to_string(),
        ));
    }

    let samples_ms: Vec<f64> = if deltas.len() <= 500 {
        deltas.clone()
    } else {
        let step = deltas.len() as f64 / 500.0;
        (0..500).map(|i| deltas[(i as f64 * step) as usize]).collect()
    };

    let total_frames = deltas.len();
    let sum_ms: f64 = deltas.iter().sum();
    let avg_frametime_ms = sum_ms / total_frames as f64;
    let avg_fps = 1000.0 / avg_frametime_ms;

    deltas.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let min_frametime_ms = deltas[0];
    let max_frametime_ms = deltas[total_frames - 1];

    let p99_idx = ((total_frames as f64 * 0.99) as usize).min(total_frames - 1);
    let p99_ms = deltas[p99_idx];
    let fps_1pct_low = 1000.0 / p99_ms;

    let p999_idx = ((total_frames as f64 * 0.999) as usize).min(total_frames - 1);
    let p999_ms = deltas[p999_idx];
    let fps_01pct_low = 1000.0 / p999_ms;

    let (status, status_label, explanation) = if avg_frametime_ms < 8.0 && fps_1pct_low > 100.0 {
        (
            "optimo".to_string(),
            "Excelente Fluidez y Consistencia de Cuadros".to_string(),
            format!(
                "Frame time promedio de {:.2} ms ({:.0} FPS) con 1% low de {:.0} FPS. La GPU y el motor gráfico entregan cuadros estables sin micro-tartamudeo (stuttering).",
                avg_frametime_ms, avg_fps, fps_1pct_low
            ),
        )
    } else if avg_frametime_ms < 16.67 && fps_1pct_low > 45.0 {
        (
            "aceptable".to_string(),
            "Rendimiento Aceptable para Juego".to_string(),
            format!(
                "Frame time promedio de {:.2} ms ({:.0} FPS). La fluidez es aceptable, aunque se observan caídas del 1% low a {:.0} FPS.",
                avg_frametime_ms, avg_fps, fps_1pct_low
            ),
        )
    } else {
        (
            "critico".to_string(),
            "Inestabilidad de Cuadros o Micro-stuttering".to_string(),
            format!(
                "Frame time elevado de {:.2} ms ({:.0} FPS) o 1% low crítico de {:.0} FPS. Se recomienda aplicar tweaks de MMCSS y prioridad de GPU en la pestaña Optimizar.",
                avg_frametime_ms, avg_fps, fps_1pct_low
            ),
        )
    };

    Ok(FrametimeAnalysisResult {
        process_id: active_pid,
        total_frames,
        duration_seconds: 10.0,
        avg_frametime_ms: (avg_frametime_ms * 100.0).round() / 100.0,
        min_frametime_ms: (min_frametime_ms * 100.0).round() / 100.0,
        max_frametime_ms: (max_frametime_ms * 100.0).round() / 100.0,
        avg_fps: (avg_fps * 10.0).round() / 10.0,
        fps_1pct_low: (fps_1pct_low * 10.0).round() / 10.0,
        fps_01pct_low: (fps_01pct_low * 10.0).round() / 10.0,
        status,
        status_label,
        explanation,
        samples_ms,
    })
}
