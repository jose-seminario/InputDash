use std::sync::Mutex;

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone)]
pub enum RestorePointOutcome {
    /// Punto de restauración creado exitosamente en esta primera modificación.
    Created,
    /// Ya se creó un punto de restauración previamente durante esta sesión.
    AlreadyCreatedInSession,
    /// No se pudo crear por configuración/estado del SO (ej. Protección del Sistema desactivada).
    /// No aborta la transacción; se registra en el log y se continúa con el backup JSON atómico.
    SkippedWarning { code: u32, reason: String },
}

pub struct RestorePointManager {
    created_in_session: Mutex<bool>,
}

impl RestorePointManager {
    pub fn new() -> Self {
        Self {
            created_in_session: Mutex::new(false),
        }
    }

    /// Intenta crear un Punto de Restauración nativo vía `srclient.dll` (`SRSetRestorePointW`)
    /// en la primera modificación de la sesión. Si Restaurar Sistema está desactivado en el SO,
    /// devuelve `SkippedWarning` para registrar el evento sin abortar la transacción.
    pub fn ensure_first_time(&self, description: &str) -> RestorePointOutcome {
        let mut guard = self
            .created_in_session
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        if *guard {
            return RestorePointOutcome::AlreadyCreatedInSession;
        }

        match create_windows_restore_point(description) {
            Ok(()) => {
                *guard = true;
                RestorePointOutcome::Created
            }
            Err(AppError::RestorePoint { code, detail }) => {
                RestorePointOutcome::SkippedWarning {
                    code,
                    reason: detail,
                }
            }
            Err(other) => RestorePointOutcome::SkippedWarning {
                code: 0xFFFF_FFFF,
                reason: other.to_string(),
            },
        }
    }
}

#[cfg(windows)]
fn create_windows_restore_point(description: &str) -> AppResult<()> {
    use windows::core::{s, w};
    use windows::Win32::Foundation::{FreeLibrary, BOOL, HMODULE};
    use windows::Win32::System::LibraryLoader::{
        GetProcAddress, LoadLibraryExW, LOAD_LIBRARY_SEARCH_SYSTEM32,
    };

    const BEGIN_SYSTEM_CHANGE: u32 = 100;
    const END_SYSTEM_CHANGE: u32 = 101;
    const MODIFY_SETTINGS: u32 = 12;
    const MAX_DESC_W: usize = 256;
    const ERROR_SUCCESS_CODE: u32 = 0;
    const ERROR_SERVICE_DISABLED_CODE: u32 = 1058;
    const ERROR_ACCESS_DENIED_CODE: u32 = 5;

    #[repr(C, packed(1))]
    struct RestorePointInfoW {
        dw_event_type: u32,
        dw_restore_pt_type: u32,
        ll_sequence_number: i64,
        sz_description: [u16; MAX_DESC_W],
    }

    #[repr(C, packed(1))]
    struct StateMgrStatus {
        n_status: u32,
        ll_sequence_number: i64,
    }

    type SRSetRestorePointWFn =
        unsafe extern "system" fn(*const RestorePointInfoW, *mut StateMgrStatus) -> BOOL;

    struct ModuleGuard(HMODULE);
    impl Drop for ModuleGuard {
        fn drop(&mut self) {
            unsafe {
                let _ = FreeLibrary(self.0);
            }
        }
    }

    let hmod = unsafe { LoadLibraryExW(w!("srclient.dll"), None, LOAD_LIBRARY_SEARCH_SYSTEM32) }
        .map_err(|e| AppError::RestorePoint {
            code: e.code().0 as u32,
            detail: format!("No se pudo cargar srclient.dll desde System32: {e}"),
        })?;

    let _mod_guard = ModuleGuard(hmod);

    let proc = unsafe { GetProcAddress(hmod, s!("SRSetRestorePointW")) }.ok_or_else(|| {
        AppError::RestorePoint {
            code: 127,
            detail: "No se encontró el símbolo SRSetRestorePointW en srclient.dll".to_string(),
        }
    })?;

    let sr_set_restore_point: SRSetRestorePointWFn = unsafe { core::mem::transmute(proc) };

    let mut desc_buf = [0u16; MAX_DESC_W];
    for (idx, code_unit) in description.encode_utf16().take(MAX_DESC_W - 1).enumerate() {
        desc_buf[idx] = code_unit;
    }

    let begin_info = RestorePointInfoW {
        dw_event_type: BEGIN_SYSTEM_CHANGE,
        dw_restore_pt_type: MODIFY_SETTINGS,
        ll_sequence_number: 0,
        sz_description: desc_buf,
    };

    let mut mgr_status = StateMgrStatus {
        n_status: 0,
        ll_sequence_number: 0,
    };

    let ok_begin = unsafe { sr_set_restore_point(&begin_info, &mut mgr_status) };
    let status_code = mgr_status.n_status;
    let seq_num = mgr_status.ll_sequence_number;

    if !ok_begin.as_bool() || status_code != ERROR_SUCCESS_CODE {
        let explanation = match status_code {
            ERROR_SERVICE_DISABLED_CODE => {
                "Protección del Sistema (Restaurar Sistema) desactivada en Windows para la unidad del SO"
                    .to_string()
            }
            ERROR_ACCESS_DENIED_CODE => {
                "Permisos insuficientes al invocar SRSetRestorePointW".to_string()
            }
            other => format!("SRSetRestorePointW devolvió código de estado Win32 {other}"),
        };
        return Err(AppError::RestorePoint {
            code: status_code,
            detail: explanation,
        });
    }

    if seq_num != 0 {
        let end_info = RestorePointInfoW {
            dw_event_type: END_SYSTEM_CHANGE,
            dw_restore_pt_type: MODIFY_SETTINGS,
            ll_sequence_number: seq_num,
            sz_description: desc_buf,
        };
        let mut end_status = StateMgrStatus {
            n_status: 0,
            ll_sequence_number: 0,
        };
        let ok_end = unsafe { sr_set_restore_point(&end_info, &mut end_status) };
        let end_code = end_status.n_status;
        if !ok_end.as_bool() || end_code != ERROR_SUCCESS_CODE {
            return Err(AppError::RestorePoint {
                code: end_code,
                detail: format!(
                    "Fallo al finalizar Punto de Restauración (END_SYSTEM_CHANGE, código {end_code})"
                ),
            });
        }
    }

    Ok(())
}

#[cfg(not(windows))]
fn create_windows_restore_point(_description: &str) -> AppResult<()> {
    Err(AppError::UnsupportedPlatform(
        "Puntos de restauración solo disponibles en Windows".to_string(),
    ))
}
