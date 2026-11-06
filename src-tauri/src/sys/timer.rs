use std::sync::atomic::{AtomicBool, Ordering};
use windows::core::w;
use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryExW, LOAD_LIBRARY_SEARCH_SYSTEM32};

type NtSetTimerResolutionFn = unsafe extern "system" fn(
    desired_resolution: u32,
    set_resolution: u8,
    current_resolution: *mut u32,
) -> i32;

type NtQueryTimerResolutionFn = unsafe extern "system" fn(
    minimum_resolution: *mut u32,
    maximum_resolution: *mut u32,
    current_resolution: *mut u32,
) -> i32;

static IS_TIMER_ACTIVE: AtomicBool = AtomicBool::new(false);

/// Representa el estado de resolución del temporizador de Windows en milisegundos
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TimerResolutionStatus {
    pub min_resolution_ms: f64,
    pub max_resolution_ms: f64,
    pub current_resolution_ms: f64,
    pub is_active_by_inputdash: bool,
}

/// Consulta la resolución actual del temporizador global en ntdll
pub fn query_timer_resolution() -> Option<TimerResolutionStatus> {
    unsafe {
        let ntdll = windows::Win32::System::LibraryLoader::GetModuleHandleW(windows::core::w!("ntdll.dll")).ok()?;
        let query_proc = GetProcAddress(ntdll, windows::core::s!("NtQueryTimerResolution"))?;
        let query_fn: NtQueryTimerResolutionFn = std::mem::transmute(query_proc);

        let mut min_res: u32 = 0;
        let mut max_res: u32 = 0;
        let mut cur_res: u32 = 0;

        let status = query_fn(&mut min_res, &mut max_res, &mut cur_res);
        if status >= 0 {
            Some(TimerResolutionStatus {
                min_resolution_ms: (min_res as f64) / 10000.0,
                max_resolution_ms: (max_res as f64) / 10000.0,
                current_resolution_ms: (cur_res as f64) / 10000.0,
                is_active_by_inputdash: IS_TIMER_ACTIVE.load(Ordering::SeqCst),
            })
        } else {
            None
        }
    }
}

/// Guarda RAII que asegura que la resolución de 0.5ms se restaura al salir o cerrar el juego.
pub struct TimerResolutionGuard {
    active: bool,
    requested_units: u32,
}

impl TimerResolutionGuard {
    /// Activa la resolución de 0.5 ms (5,000 unidades de 100ns)
    pub fn acquire_05ms() -> Result<Self, String> {
        Self::acquire_units(5000)
    }

    /// Activa resolución personalizada en unidades de 100ns
    pub fn acquire_units(units: u32) -> Result<Self, String> {
        unsafe {
            let ntdll = LoadLibraryExW(w!("ntdll.dll"), None, LOAD_LIBRARY_SEARCH_SYSTEM32)
                .map_err(|e| format!("No se pudo cargar ntdll.dll: {e}"))?;

            let set_proc = GetProcAddress(ntdll, windows::core::s!("NtSetTimerResolution"))
                .ok_or_else(|| "NtSetTimerResolution no encontrado en ntdll.dll".to_string())?;

            let set_fn: NtSetTimerResolutionFn = std::mem::transmute(set_proc);

            let mut cur_res: u32 = 0;
            // 1 = True (Set/Request timer resolution)
            let status = set_fn(units, 1, &mut cur_res);

            if status >= 0 {
                IS_TIMER_ACTIVE.store(true, Ordering::SeqCst);
                Ok(Self {
                    active: true,
                    requested_units: units,
                })
            } else {
                Err(format!("NtSetTimerResolution falló con NTSTATUS {status:#X}"))
            }
        }
    }

    /// Libera la solicitud de timer y devuelve el control al sistema operativo
    pub fn release(&mut self) {
        if self.active {
            unsafe {
                if let Ok(ntdll) = LoadLibraryExW(w!("ntdll.dll"), None, LOAD_LIBRARY_SEARCH_SYSTEM32) {
                    if let Some(set_proc) = GetProcAddress(ntdll, windows::core::s!("NtSetTimerResolution")) {
                        let set_fn: NtSetTimerResolutionFn = std::mem::transmute(set_proc);
                        let mut cur_res: u32 = 0;
                        // 0 = False (Unset/Release timer resolution)
                        let _ = set_fn(self.requested_units, 0, &mut cur_res);
                    }
                }
            }
            self.active = false;
            IS_TIMER_ACTIVE.store(false, Ordering::SeqCst);
        }
    }
}

impl Drop for TimerResolutionGuard {
    fn drop(&mut self) {
        self.release();
    }
}
