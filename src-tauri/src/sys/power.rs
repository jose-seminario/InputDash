use serde::{Deserialize, Serialize};

#[cfg(not(windows))]
use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PowerPlanInfo {
    pub guid: String,
    pub friendly_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PowerPlanApplyResult {
    pub active_plan: PowerPlanInfo,
    /// Si el plan (ej. Ultimate Performance) no estaba instanciado y fue duplicado desde
    /// la plantilla nativa de Windows, guarda el GUID generado para limpieza al revertir.
    pub duplicated_guid: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PowerSettingIndices {
    pub scheme_guid: String,
    pub ac_index: u32,
    pub dc_index: u32,
}

impl PowerSettingIndices {
    pub fn display_human(&self) -> String {
        let fmt_state = |v: u32| match v {
            0 => "0 (Desactivado)".to_string(),
            1 => "1 (Activado)".to_string(),
            other => format!("{other}"),
        };
        format!(
            "AC: {} | DC: {} [Plan {}]",
            fmt_state(self.ac_index),
            fmt_state(self.dc_index),
            self.scheme_guid
        )
    }
}

#[cfg(windows)]
mod win32_impl {
    use core::ffi::c_void;
    use core::ptr;

    use windows::core::GUID;
    use windows::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_MORE_DATA, ERROR_SUCCESS};
    use windows::Win32::System::Registry::HKEY;

    use super::{PowerPlanApplyResult, PowerPlanInfo, PowerSettingIndices};
    use crate::error::{AppError, AppResult};

    #[link(name = "powrprof")]
    extern "system" {
        fn PowerGetActiveScheme(user_root_power_key: HKEY, active_policy_guid: *mut *mut GUID) -> u32;
        fn PowerSetActiveScheme(user_root_power_key: HKEY, scheme_guid: *const GUID) -> u32;
        fn PowerDuplicateScheme(
            root_power_key: HKEY,
            source_scheme_guid: *const GUID,
            destination_scheme_guid: *mut *mut GUID,
        ) -> u32;
        fn PowerDeleteScheme(root_power_key: HKEY, scheme_guid: *const GUID) -> u32;
        fn PowerReadFriendlyName(
            root_power_key: HKEY,
            scheme_guid: *const GUID,
            subgroup_of_power_settings_guid: *const GUID,
            power_setting_guid: *const GUID,
            buffer: *mut u8,
            buffer_size: *mut u32,
        ) -> u32;
        fn PowerReadACValueIndex(
            root_power_key: HKEY,
            scheme_guid: *const GUID,
            subgroup_of_power_settings_guid: *const GUID,
            power_setting_guid: *const GUID,
            ac_value_index: *mut u32,
        ) -> u32;
        fn PowerReadDCValueIndex(
            root_power_key: HKEY,
            scheme_guid: *const GUID,
            subgroup_of_power_settings_guid: *const GUID,
            power_setting_guid: *const GUID,
            dc_value_index: *mut u32,
        ) -> u32;
        fn PowerWriteACValueIndex(
            root_power_key: HKEY,
            scheme_guid: *const GUID,
            subgroup_of_power_settings_guid: *const GUID,
            power_setting_guid: *const GUID,
            ac_value_index: u32,
        ) -> u32;
        fn PowerWriteDCValueIndex(
            root_power_key: HKEY,
            scheme_guid: *const GUID,
            subgroup_of_power_settings_guid: *const GUID,
            power_setting_guid: *const GUID,
            dc_value_index: u32,
        ) -> u32;
    }

    #[link(name = "kernel32")]
    extern "system" {
        fn LocalFree(hmem: *mut c_void) -> *mut c_void;
    }

    struct LocalAllocGuard(*mut GUID);
    impl Drop for LocalAllocGuard {
        fn drop(&mut self) {
            if !self.0.is_null() {
                unsafe {
                    let _ = LocalFree(self.0 as *mut c_void);
                }
            }
        }
    }

    pub fn parse_guid(text: &str) -> AppResult<GUID> {
        let clean = text.trim().trim_start_matches('{').trim_end_matches('}');
        let parts: Vec<&str> = clean.split('-').collect();
        if parts.len() != 5
            || parts[0].len() != 8
            || parts[1].len() != 4
            || parts[2].len() != 4
            || parts[3].len() != 4
            || parts[4].len() != 12
        {
            return Err(AppError::PowerApi {
                operation: "parse_guid".to_string(),
                win32_code: 0,
                detail: format!("Formato GUID inválido: '{text}'"),
            });
        }

        let d1 = u32::from_str_radix(parts[0], 16).map_err(|_| guid_hex_err(text))?;
        let d2 = u16::from_str_radix(parts[1], 16).map_err(|_| guid_hex_err(text))?;
        let d3 = u16::from_str_radix(parts[2], 16).map_err(|_| guid_hex_err(text))?;

        let mut d4 = [0u8; 8];
        d4[0] = u8::from_str_radix(&parts[3][0..2], 16).map_err(|_| guid_hex_err(text))?;
        d4[1] = u8::from_str_radix(&parts[3][2..4], 16).map_err(|_| guid_hex_err(text))?;
        for i in 0..6 {
            let start = i * 2;
            d4[2 + i] =
                u8::from_str_radix(&parts[4][start..start + 2], 16).map_err(|_| guid_hex_err(text))?;
        }

        Ok(GUID {
            data1: d1,
            data2: d2,
            data3: d3,
            data4: d4,
        })
    }

    fn guid_hex_err(text: &str) -> AppError {
        AppError::PowerApi {
            operation: "parse_guid".to_string(),
            win32_code: 0,
            detail: format!("Caracteres hexadecimales inválidos en GUID: '{text}'"),
        }
    }

    pub fn format_guid(g: &GUID) -> String {
        format!(
            "{:08x}-{:04x}-{:04x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
            g.data1,
            g.data2,
            g.data3,
            g.data4[0],
            g.data4[1],
            g.data4[2],
            g.data4[3],
            g.data4[4],
            g.data4[5],
            g.data4[6],
            g.data4[7]
        )
    }

    fn read_scheme_friendly_name_raw(scheme: &GUID) -> Result<String, u32> {
        let mut buf_size: u32 = 0;
        let probe = unsafe {
            PowerReadFriendlyName(
                HKEY::default(),
                scheme,
                ptr::null(),
                ptr::null(),
                ptr::null_mut(),
                &mut buf_size,
            )
        };

        if probe != ERROR_SUCCESS.0 && probe != ERROR_MORE_DATA.0 {
            return Err(probe);
        }
        if buf_size == 0 {
            return Ok(format_guid(scheme));
        }

        let mut buffer = vec![0u8; buf_size as usize];
        let status = unsafe {
            PowerReadFriendlyName(
                HKEY::default(),
                scheme,
                ptr::null(),
                ptr::null(),
                buffer.as_mut_ptr(),
                &mut buf_size,
            )
        };

        if status != ERROR_SUCCESS.0 {
            return Err(status);
        }

        buffer.truncate(buf_size as usize);
        let mut wide = Vec::with_capacity(buffer.len() / 2);
        for chunk in buffer.chunks_exact(2) {
            wide.push(u16::from_le_bytes([chunk[0], chunk[1]]));
        }
        let name = String::from_utf16_lossy(&wide)
            .trim_end_matches('\0')
            .trim()
            .to_string();

        if name.is_empty() {
            Ok(format_guid(scheme))
        } else {
            Ok(name)
        }
    }

    fn get_active_guid_raw() -> AppResult<GUID> {
        let mut guid_ptr: *mut GUID = ptr::null_mut();
        let status = unsafe { PowerGetActiveScheme(HKEY::default(), &mut guid_ptr) };
        let _guard = LocalAllocGuard(guid_ptr);

        if status != ERROR_SUCCESS.0 || guid_ptr.is_null() {
            return Err(AppError::PowerApi {
                operation: "PowerGetActiveScheme".to_string(),
                win32_code: status,
                detail: "No se pudo consultar el plan de energía activo".to_string(),
            });
        }

        Ok(unsafe { *guid_ptr })
    }

    pub fn get_active_plan() -> AppResult<PowerPlanInfo> {
        let guid = get_active_guid_raw()?;
        let guid_str = format_guid(&guid);
        let friendly_name =
            read_scheme_friendly_name_raw(&guid).unwrap_or_else(|_| guid_str.clone());

        Ok(PowerPlanInfo {
            guid: guid_str,
            friendly_name,
        })
    }

    pub fn activate_or_duplicate_plan(target_guid_str: &str) -> AppResult<PowerPlanApplyResult> {
        let target_guid = parse_guid(target_guid_str)?;

        // Verifica si el esquema ya existe instanciado en el almacén de políticas del usuario/SO
        let (guid_to_activate, duplicated_guid) = match read_scheme_friendly_name_raw(&target_guid) {
            Ok(_) => (target_guid, None),
            Err(code) if code == ERROR_FILE_NOT_FOUND.0 || code == 259 => {
                let mut dup_ptr: *mut GUID = ptr::null_mut();
                let dup_status =
                    unsafe { PowerDuplicateScheme(HKEY::default(), &target_guid, &mut dup_ptr) };
                let _dup_guard = LocalAllocGuard(dup_ptr);

                if dup_status != ERROR_SUCCESS.0 || dup_ptr.is_null() {
                    return Err(AppError::PowerApi {
                        operation: "PowerDuplicateScheme".to_string(),
                        win32_code: dup_status,
                        detail: format!(
                            "No se pudo instanciar la plantilla del plan '{target_guid_str}'. Verifica permisos de Administrador o soporte S0 Modern Standby."
                        ),
                    });
                }

                let created = unsafe { *dup_ptr };
                let created_str = format_guid(&created);
                (created, Some(created_str))
            }
            Err(other_code) => {
                return Err(AppError::PowerApi {
                    operation: "PowerReadFriendlyName".to_string(),
                    win32_code: other_code,
                    detail: format!("Error inspeccionando plan '{target_guid_str}'"),
                });
            }
        };

        let set_status = unsafe { PowerSetActiveScheme(HKEY::default(), &guid_to_activate) };
        if set_status != ERROR_SUCCESS.0 {
            if let Some(ref dup_str) = duplicated_guid {
                if let Ok(dup_g) = parse_guid(dup_str) {
                    unsafe {
                        let _ = PowerDeleteScheme(HKEY::default(), &dup_g);
                    }
                }
            }
            return Err(AppError::PowerApi {
                operation: "PowerSetActiveScheme".to_string(),
                win32_code: set_status,
                detail: format!(
                    "No se pudo activar el plan '{}'. Ejecuta InputDash como Administrador.",
                    format_guid(&guid_to_activate)
                ),
            });
        }

        let active_plan = get_active_plan()?;
        Ok(PowerPlanApplyResult {
            active_plan,
            duplicated_guid,
        })
    }

    pub fn restore_plan(
        previous_guid_str: &str,
        duplicated_guid_to_cleanup: Option<&str>,
    ) -> AppResult<PowerPlanInfo> {
        let prev_guid = parse_guid(previous_guid_str)?;

        let status = unsafe { PowerSetActiveScheme(HKEY::default(), &prev_guid) };
        if status != ERROR_SUCCESS.0 {
            return Err(AppError::PowerApi {
                operation: "PowerSetActiveScheme(restore)".to_string(),
                win32_code: status,
                detail: format!("No se pudo restaurar el plan original '{previous_guid_str}'"),
            });
        }

        if let Some(dup_str) = duplicated_guid_to_cleanup {
            if !dup_str.eq_ignore_ascii_case(previous_guid_str) {
                if let Ok(dup_guid) = parse_guid(dup_str) {
                    unsafe {
                        let _ = PowerDeleteScheme(HKEY::default(), &dup_guid);
                    }
                }
            }
        }

        get_active_plan()
    }

    pub fn read_setting_on_active_scheme(
        subgroup_guid_str: &str,
        setting_guid_str: &str,
    ) -> AppResult<PowerSettingIndices> {
        let active_guid = get_active_guid_raw()?;
        let scheme_str = format_guid(&active_guid);
        read_setting_on_scheme(&scheme_str, subgroup_guid_str, setting_guid_str)
    }

    pub fn read_setting_on_scheme(
        scheme_guid_str: &str,
        subgroup_guid_str: &str,
        setting_guid_str: &str,
    ) -> AppResult<PowerSettingIndices> {
        let scheme_guid = parse_guid(scheme_guid_str)?;
        let subgroup_guid = parse_guid(subgroup_guid_str)?;
        let setting_guid = parse_guid(setting_guid_str)?;

        let mut ac_val: u32 = 0;
        let ac_status = unsafe {
            PowerReadACValueIndex(
                HKEY::default(),
                &scheme_guid,
                &subgroup_guid,
                &setting_guid,
                &mut ac_val,
            )
        };

        if ac_status != ERROR_SUCCESS.0 {
            return Err(AppError::PowerApi {
                operation: "PowerReadACValueIndex".to_string(),
                win32_code: ac_status,
                detail: format!(
                    "No se pudo leer índice AC para '{setting_guid_str}' en plan '{scheme_guid_str}'"
                ),
            });
        }

        let mut dc_val: u32 = 0;
        let dc_status = unsafe {
            PowerReadDCValueIndex(
                HKEY::default(),
                &scheme_guid,
                &subgroup_guid,
                &setting_guid,
                &mut dc_val,
            )
        };

        if dc_status != ERROR_SUCCESS.0 {
            if dc_status == ERROR_FILE_NOT_FOUND.0 {
                dc_val = ac_val;
            } else {
                return Err(AppError::PowerApi {
                    operation: "PowerReadDCValueIndex".to_string(),
                    win32_code: dc_status,
                    detail: format!(
                        "No se pudo leer índice DC para '{setting_guid_str}' en plan '{scheme_guid_str}'"
                    ),
                });
            }
        }

        Ok(PowerSettingIndices {
            scheme_guid: format_guid(&scheme_guid),
            ac_index: ac_val,
            dc_index: dc_val,
        })
    }

    pub fn write_setting_on_scheme(
        scheme_guid_str: &str,
        subgroup_guid_str: &str,
        setting_guid_str: &str,
        ac_index: u32,
        dc_index: u32,
    ) -> AppResult<()> {
        let scheme_guid = parse_guid(scheme_guid_str)?;
        let subgroup_guid = parse_guid(subgroup_guid_str)?;
        let setting_guid = parse_guid(setting_guid_str)?;

        let ac_status = unsafe {
            PowerWriteACValueIndex(
                HKEY::default(),
                &scheme_guid,
                &subgroup_guid,
                &setting_guid,
                ac_index,
            )
        };
        if ac_status != ERROR_SUCCESS.0 {
            return Err(AppError::PowerApi {
                operation: "PowerWriteACValueIndex".to_string(),
                win32_code: ac_status,
                detail: format!(
                    "Fallo escribiendo índice AC={ac_index} en '{setting_guid_str}'"
                ),
            });
        }

        let dc_status = unsafe {
            PowerWriteDCValueIndex(
                HKEY::default(),
                &scheme_guid,
                &subgroup_guid,
                &setting_guid,
                dc_index,
            )
        };
        if dc_status != ERROR_SUCCESS.0 {
            return Err(AppError::PowerApi {
                operation: "PowerWriteDCValueIndex".to_string(),
                win32_code: dc_status,
                detail: format!(
                    "Fallo escribiendo índice DC={dc_index} en '{setting_guid_str}'"
                ),
            });
        }

        // Si el esquema modificado es el esquema actualmente activo, re-aplicar para efecto inmediato
        if let Ok(current_active) = get_active_guid_raw() {
            if current_active == scheme_guid {
                let apply_status = unsafe { PowerSetActiveScheme(HKEY::default(), &scheme_guid) };
                if apply_status != ERROR_SUCCESS.0 {
                    return Err(AppError::PowerApi {
                        operation: "PowerSetActiveScheme(refresh)".to_string(),
                        win32_code: apply_status,
                        detail: "No se pudo refrescar el esquema activo tras modificar USB Selective Suspend"
                            .to_string(),
                    });
                }
            }
        }

        Ok(())
    }
}

#[cfg(windows)]
pub use win32_impl::{
    activate_or_duplicate_plan, get_active_plan, read_setting_on_active_scheme,
    read_setting_on_scheme, restore_plan, write_setting_on_scheme,
};

#[cfg(not(windows))]
pub fn get_active_plan() -> AppResult<PowerPlanInfo> {
    Err(AppError::UnsupportedPlatform(
        "PowrProf requiere Windows 10/11".to_string(),
    ))
}

#[cfg(not(windows))]
pub fn activate_or_duplicate_plan(_target_guid_str: &str) -> AppResult<PowerPlanApplyResult> {
    Err(AppError::UnsupportedPlatform(
        "PowrProf requiere Windows 10/11".to_string(),
    ))
}

#[cfg(not(windows))]
pub fn restore_plan(
    _previous_guid_str: &str,
    _duplicated_guid_to_cleanup: Option<&str>,
) -> AppResult<PowerPlanInfo> {
    Err(AppError::UnsupportedPlatform(
        "PowrProf requiere Windows 10/11".to_string(),
    ))
}

#[cfg(not(windows))]
pub fn read_setting_on_active_scheme(
    _subgroup_guid_str: &str,
    _setting_guid_str: &str,
) -> AppResult<PowerSettingIndices> {
    Err(AppError::UnsupportedPlatform(
        "PowrProf requiere Windows 10/11".to_string(),
    ))
}

#[cfg(not(windows))]
pub fn read_setting_on_scheme(
    _scheme_guid_str: &str,
    _subgroup_guid_str: &str,
    _setting_guid_str: &str,
) -> AppResult<PowerSettingIndices> {
    Err(AppError::UnsupportedPlatform(
        "PowrProf requiere Windows 10/11".to_string(),
    ))
}

#[cfg(not(windows))]
pub fn write_setting_on_scheme(
    _scheme_guid_str: &str,
    _subgroup_guid_str: &str,
    _setting_guid_str: &str,
    _ac_index: u32,
    _dc_index: u32,
) -> AppResult<()> {
    Err(AppError::UnsupportedPlatform(
        "PowrProf requiere Windows 10/11".to_string(),
    ))
}
