use core::ffi::c_void;
use core::ptr;
use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PollingAnalysisResult {
    pub measured_hz: f64,
    pub avg_interval_ms: f64,
    pub jitter_ms: f64,
    pub min_interval_ms: f64,
    pub max_interval_ms: f64,
    pub sample_count: usize,
    pub status: String,       // "optimo" | "aceptable" | "critico"
    pub status_label: String, // Texto legible para el usuario
}

// -----------------------------------------------------------------------------
// Win32 Types and Constants
// -----------------------------------------------------------------------------

const WM_INPUT: u32 = 0x00FF;
const RID_INPUT: u32 = 0x10000003;
const RIM_TYPEMOUSE: u32 = 0;
const RIDEV_INPUTSINK: u32 = 0x00000100;
const RIDEV_REMOVE: u32 = 0x00000001;
const PM_REMOVE: u32 = 0x0001;
const QS_ALLINPUT: u32 = 0x04FF;
const MWMO_INPUTAVAILABLE: u32 = 0x0004;
const GWLP_USERDATA: i32 = -21;

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct RAWINPUTDEVICE {
    usUsagePage: u16,
    usUsage: u16,
    dwFlags: u32,
    hwndTarget: *mut c_void,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct RAWINPUTHEADER {
    dwType: u32,
    dwSize: u32,
    hDevice: *mut c_void,
    wParam: usize,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct RAWMOUSE {
    usFlags: u16,
    ulButtons: u32,
    ulRawButtons: u32,
    lLastX: i32,
    lLastY: i32,
    ulExtraInformation: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
union RAWINPUTDATA {
    mouse: RAWMOUSE,
    _padding: [u8; 32],
}

impl Default for RAWINPUTDATA {
    fn default() -> Self {
        RAWINPUTDATA {
            _padding: [0u8; 32],
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct RAWINPUT {
    header: RAWINPUTHEADER,
    data: RAWINPUTDATA,
}

#[repr(C)]
struct WNDCLASSEXW {
    cbSize: u32,
    style: u32,
    lpfnWndProc: Option<unsafe extern "system" fn(*mut c_void, u32, usize, isize) -> isize>,
    cbClsExtra: i32,
    cbWndExtra: i32,
    hInstance: *mut c_void,
    hIcon: *mut c_void,
    hCursor: *mut c_void,
    hbrBackground: *mut c_void,
    lpszMenuName: *const u16,
    lpszClassName: *const u16,
    hIconSm: *mut c_void,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct POINT {
    x: i32,
    y: i32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct MSG {
    hwnd: *mut c_void,
    message: u32,
    wParam: usize,
    lParam: isize,
    time: u32,
    pt: POINT,
}

#[link(name = "user32")]
extern "system" {
    fn RegisterClassExW(lpwcx: *const WNDCLASSEXW) -> u16;
    fn UnregisterClassW(lpClassName: *const u16, hInstance: *mut c_void) -> i32;
    fn CreateWindowExW(
        dwExStyle: u32,
        lpClassName: *const u16,
        lpWindowName: *const u16,
        dwStyle: u32,
        X: i32,
        Y: i32,
        nWidth: i32,
        nHeight: i32,
        hWndParent: *mut c_void,
        hMenu: *mut c_void,
        hInstance: *mut c_void,
        lpParam: *mut c_void,
    ) -> *mut c_void;
    fn DestroyWindow(hWnd: *mut c_void) -> i32;
    fn RegisterRawInputDevices(
        pRawInputDevices: *const RAWINPUTDEVICE,
        uiNumDevices: u32,
        cbSize: u32,
    ) -> i32;
    fn GetRawInputData(
        hRawInput: *mut c_void,
        uiCommand: u32,
        pData: *mut c_void,
        pcbSize: *mut u32,
        cbSizeHeader: u32,
    ) -> u32;
    fn PeekMessageW(
        lpMsg: *mut MSG,
        hWnd: *mut c_void,
        wMsgFilterMin: u32,
        wMsgFilterMax: u32,
        wRemoveMsg: u32,
    ) -> i32;
    fn TranslateMessage(lpMsg: *const MSG) -> i32;
    fn DispatchMessageW(lpMsg: *const MSG) -> isize;
    fn DefWindowProcW(hWnd: *mut c_void, uMsg: u32, wParam: usize, lParam: isize) -> isize;
    fn SetWindowLongPtrW(hWnd: *mut c_void, nIndex: i32, dwNewLong: isize) -> isize;
    fn GetWindowLongPtrW(hWnd: *mut c_void, nIndex: i32) -> isize;
    fn MsgWaitForMultipleObjectsEx(
        nCount: u32,
        pHandles: *const *mut c_void,
        dwMilliseconds: u32,
        dwWakeMask: u32,
        dwFlags: u32,
    ) -> u32;
}

#[link(name = "kernel32")]
extern "system" {
    fn GetModuleHandleW(lpModuleName: *const u16) -> *mut c_void;
    fn QueryPerformanceCounter(lpPerformanceCount: *mut i64) -> i32;
    fn QueryPerformanceFrequency(lpFrequency: *mut i64) -> i32;
    fn GetTickCount64() -> u64;
}

// -----------------------------------------------------------------------------
// Collector State
// -----------------------------------------------------------------------------

struct CollectorState {
    timestamps: Vec<i64>,
}

unsafe extern "system" fn polling_wndproc(
    hwnd: *mut c_void,
    msg: u32,
    wparam: usize,
    lparam: isize,
) -> isize {
    if msg == WM_INPUT {
        let state_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut CollectorState;
        if !state_ptr.is_null() {
            let mut raw = RAWINPUT::default();
            let mut size = std::mem::size_of::<RAWINPUT>() as u32;
            let header_size = std::mem::size_of::<RAWINPUTHEADER>() as u32;

            let read = GetRawInputData(
                lparam as *mut c_void,
                RID_INPUT,
                &mut raw as *mut _ as *mut c_void,
                &mut size,
                header_size,
            );

            if read != u32::MAX && raw.header.dwType == RIM_TYPEMOUSE {
                let mouse_data = raw.data.mouse;
                // Only count true displacement packets (ignore static button-only or wheel packets)
                if mouse_data.lLastX != 0 || mouse_data.lLastY != 0 {
                    let mut qpc = 0i64;
                    QueryPerformanceCounter(&mut qpc);
                    (*state_ptr).timestamps.push(qpc);
                }
            }
        }
    }

    DefWindowProcW(hwnd, msg, wparam, lparam)
}

// -----------------------------------------------------------------------------
// Public Polling Analysis Routine
// -----------------------------------------------------------------------------

pub fn run_polling_analysis() -> AppResult<PollingAnalysisResult> {
    let mut freq = 0i64;
    unsafe {
        QueryPerformanceFrequency(&mut freq);
    }
    if freq <= 0 {
        return Err(AppError::PollingAnalysis(
            "QueryPerformanceCounter no soportado en esta plataforma".to_string(),
        ));
    }

    let h_instance = unsafe { GetModuleHandleW(ptr::null()) };
    let class_name: Vec<u16> = format!("InputDashPollingWindow_{}\0", unsafe { GetTickCount64() })
        .encode_utf16()
        .collect();

    let wc = WNDCLASSEXW {
        cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
        style: 0,
        lpfnWndProc: Some(polling_wndproc),
        cbClsExtra: 0,
        cbWndExtra: 0,
        hInstance: h_instance,
        hIcon: ptr::null_mut(),
        hCursor: ptr::null_mut(),
        hbrBackground: ptr::null_mut(),
        lpszMenuName: ptr::null(),
        lpszClassName: class_name.as_ptr(),
        hIconSm: ptr::null_mut(),
    };

    let atom = unsafe { RegisterClassExW(&wc) };
    if atom == 0 {
        return Err(AppError::PollingAnalysis(
            "Fallo al registrar ventana message-only para captura de mouse".to_string(),
        ));
    }

    // HWND_MESSAGE = ((HWND)-3)
    let hwnd_message = -3isize as *mut c_void;
    let hwnd = unsafe {
        CreateWindowExW(
            0,
            class_name.as_ptr(),
            ptr::null(),
            0,
            0,
            0,
            0,
            0,
            hwnd_message,
            ptr::null_mut(),
            h_instance,
            ptr::null_mut(),
        )
    };

    if hwnd.is_null() {
        unsafe {
            UnregisterClassW(class_name.as_ptr(), h_instance);
        }
        return Err(AppError::PollingAnalysis(
            "Fallo al instanciar ventana message-only".to_string(),
        ));
    }

    let mut collector = CollectorState {
        timestamps: Vec::with_capacity(8192),
    };

    unsafe {
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, &mut collector as *mut _ as isize);
    }

    // Register Mouse Raw Input with RIDEV_INPUTSINK
    let rid = RAWINPUTDEVICE {
        usUsagePage: 0x01, // Generic Desktop Controls
        usUsage: 0x02,     // Mouse
        dwFlags: RIDEV_INPUTSINK,
        hwndTarget: hwnd,
    };

    let reg_ok = unsafe {
        RegisterRawInputDevices(
            &rid,
            1,
            std::mem::size_of::<RAWINPUTDEVICE>() as u32,
        )
    };

    if reg_ok == 0 {
        unsafe {
            DestroyWindow(hwnd);
            UnregisterClassW(class_name.as_ptr(), h_instance);
        }
        return Err(AppError::PollingAnalysis(
            "No se pudo registrar Raw Input para el mouse (RegisterRawInputDevices falló)".to_string(),
        ));
    }

    // 4000 ms capture loop
    let duration_ms = 4000u64;
    let start_tick = unsafe { GetTickCount64() };

    while unsafe { GetTickCount64() }.saturating_sub(start_tick) < duration_ms {
        let mut msg = MSG::default();
        while unsafe { PeekMessageW(&mut msg, hwnd, 0, 0, PM_REMOVE) } != 0 {
            unsafe {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }

        let elapsed = unsafe { GetTickCount64() }.saturating_sub(start_tick);
        if elapsed >= duration_ms {
            break;
        }

        let wait_ms = (duration_ms - elapsed).min(10) as u32;
        unsafe {
            MsgWaitForMultipleObjectsEx(0, ptr::null(), wait_ms, QS_ALLINPUT, MWMO_INPUTAVAILABLE);
        }
    }

    // Clean up resources immediately
    let unreg_rid = RAWINPUTDEVICE {
        usUsagePage: 0x01,
        usUsage: 0x02,
        dwFlags: RIDEV_REMOVE,
        hwndTarget: ptr::null_mut(),
    };
    unsafe {
        RegisterRawInputDevices(
            &unreg_rid,
            1,
            std::mem::size_of::<RAWINPUTDEVICE>() as u32,
        );
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
        DestroyWindow(hwnd);
        UnregisterClassW(class_name.as_ptr(), h_instance);
    }

    let timestamps = collector.timestamps;

    // Robust validation: Check movement count
    if timestamps.len() < 40 {
        return Err(AppError::PollingAnalysis(
            "Movimiento insuficiente detectado. Repite la prueba moviendo el mouse".to_string(),
        ));
    }

    // Calculate intervals in ms
    let freq_f = freq as f64;
    let mut intervals_ms = Vec::with_capacity(timestamps.len());

    for i in 1..timestamps.len() {
        let diff_ticks = timestamps[i] - timestamps[i - 1];
        if diff_ticks > 0 {
            let dt_ms = (diff_ticks as f64 * 1000.0) / freq_f;
            // Discard user pauses between sweeps (intervals > 30 ms or sub-microsecond artifacts < 0.05 ms)
            if dt_ms >= 0.05 && dt_ms <= 30.0 {
                intervals_ms.push(dt_ms);
            }
        }
    }

    if intervals_ms.len() < 30 {
        return Err(AppError::PollingAnalysis(
            "Movimiento insuficiente detectado. Repite la prueba moviendo el mouse".to_string(),
        ));
    }

    let sample_count = intervals_ms.len();
    let sum_intervals: f64 = intervals_ms.iter().sum();
    let avg_interval_ms = sum_intervals / (sample_count as f64);
    let measured_hz = 1000.0 / avg_interval_ms;

    // Variance & Jitter (standard deviation in ms)
    let variance: f64 = intervals_ms
        .iter()
        .map(|dt| {
            let diff = dt - avg_interval_ms;
            diff * diff
        })
        .sum::<f64>()
        / (sample_count as f64);
    let jitter_ms = variance.sqrt();

    let min_interval_ms = intervals_ms.iter().copied().fold(f64::INFINITY, f64::min);
    let max_interval_ms = intervals_ms.iter().copied().fold(f64::NEG_INFINITY, f64::max);

    // Semáforo:
    // Óptimo: >= 990 Hz y jitter < 0.1 ms
    // Aceptable: >= 500 Hz
    // Crítico: <= 125 Hz
    let (status, status_label) = if measured_hz >= 990.0 && jitter_ms < 0.1 {
        ("optimo".to_string(), "Óptimo (1000 Hz Estable)".to_string())
    } else if measured_hz >= 500.0 {
        ("aceptable".to_string(), "Aceptable (500 Hz / Jitter moderado)".to_string())
    } else {
        ("critico".to_string(), "Crítico (<=125 Hz o Inestable)".to_string())
    };

    Ok(PollingAnalysisResult {
        measured_hz: (measured_hz * 10.0).round() / 10.0,
        avg_interval_ms: (avg_interval_ms * 1000.0).round() / 1000.0,
        jitter_ms: (jitter_ms * 1000.0).round() / 1000.0,
        min_interval_ms: (min_interval_ms * 1000.0).round() / 1000.0,
        max_interval_ms: (max_interval_ms * 1000.0).round() / 1000.0,
        sample_count,
        status,
        status_label,
    })
}
