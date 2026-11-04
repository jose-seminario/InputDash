use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RegistryHive {
    HKCU,
    HKLM,
}

impl RegistryHive {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::HKCU => "HKCU",
            Self::HKLM => "HKLM",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RegistryValueKind {
    DWORD,
    QWORD,
    SZ,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value")]
pub enum RegistryData {
    Dword(u32),
    Qword(u64),
    Sz(String),
}

impl RegistryData {
    pub fn display_short(&self) -> String {
        match self {
            Self::Dword(v) => format!("{v} (0x{v:08X})"),
            Self::Qword(v) => format!("{v} (0x{v:016X})"),
            Self::Sz(s) => format!("\"{s}\""),
        }
    }
}

/// Representa el estado exacto de un valor en el Registro de Windows antes o después de un cambio.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", content = "data")]
pub enum RegistryState {
    KeyMissing,
    ValueMissing,
    Present(RegistryData),
}

impl RegistryState {
    pub fn display_human(&self) -> String {
        match self {
            Self::KeyMissing => "<Clave no existente>".to_string(),
            Self::ValueMissing => "<Valor no definido (defecto OS)>".to_string(),
            Self::Present(data) => data.display_short(),
        }
    }
}

#[cfg(windows)]
mod win32_impl {
    use core::ffi::c_void;
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{
        ERROR_ACCESS_DENIED, ERROR_FILE_NOT_FOUND, ERROR_PATH_NOT_FOUND, ERROR_SUCCESS,
        WIN32_ERROR,
    };
    use windows::Win32::System::Registry::{
        RegCloseKey, RegCreateKeyExW, RegDeleteValueW, RegGetValueW, RegOpenKeyExW,
        RegSetValueExW, HKEY, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_QUERY_VALUE, KEY_READ,
        KEY_SET_VALUE, REG_DWORD, REG_EXPAND_SZ, REG_OPTION_NON_VOLATILE, REG_QWORD, REG_SZ,
        REG_VALUE_TYPE, RRF_RT_ANY,
    };

    use super::{RegistryData, RegistryHive, RegistryState};
    use crate::error::{AppError, AppResult};

    struct OwnedHKey(HKEY);

    impl Drop for OwnedHKey {
        fn drop(&mut self) {
            if !self.0.is_invalid() {
                unsafe {
                    let _ = RegCloseKey(self.0);
                }
            }
        }
    }

    fn hive_to_hkey(hive: RegistryHive) -> HKEY {
        match hive {
            RegistryHive::HKCU => HKEY_CURRENT_USER,
            RegistryHive::HKLM => HKEY_LOCAL_MACHINE,
        }
    }

    fn to_wide_null(input: &str) -> Vec<u16> {
        input.encode_utf16().chain(std::iter::once(0)).collect()
    }

    fn map_win32_err(operation: &str, path: &str, err: WIN32_ERROR) -> AppError {
        let hint = if err == ERROR_ACCESS_DENIED {
            format!("Acceso denegado a '{path}'. Ejecuta InputDash como Administrador.")
        } else {
            format!("Fallo sobre '{path}'.")
        };

        AppError::RegistryApi {
            operation: operation.to_string(),
            win32_code: err.0,
            detail: hint,
        }
    }

    pub fn read_value(
        hive: RegistryHive,
        subkey: &str,
        value_name: &str,
    ) -> AppResult<RegistryState> {
        let full_location = format!("{}\\{}\\{}", hive.as_str(), subkey, value_name);
        let subkey_w = to_wide_null(subkey);
        let value_w = to_wide_null(value_name);

        let mut raw_hkey = HKEY::default();
        let open_status = unsafe {
            RegOpenKeyExW(
                hive_to_hkey(hive),
                PCWSTR(subkey_w.as_ptr()),
                None,
                KEY_READ,
                &mut raw_hkey,
            )
        };

        if open_status == ERROR_FILE_NOT_FOUND || open_status == ERROR_PATH_NOT_FOUND {
            return Ok(RegistryState::KeyMissing);
        }
        if open_status != ERROR_SUCCESS {
            return Err(map_win32_err("RegOpenKeyExW", &full_location, open_status));
        }

        let key_guard = OwnedHKey(raw_hkey);

        let mut reg_type = REG_VALUE_TYPE::default();
        let mut cb_data: u32 = 0;

        let probe_status = unsafe {
            RegGetValueW(
                key_guard.0,
                PCWSTR::null(),
                PCWSTR(value_w.as_ptr()),
                RRF_RT_ANY,
                Some(&mut reg_type),
                None,
                Some(&mut cb_data),
            )
        };

        if probe_status == ERROR_FILE_NOT_FOUND || probe_status == ERROR_PATH_NOT_FOUND {
            return Ok(RegistryState::ValueMissing);
        }
        if probe_status != ERROR_SUCCESS {
            return Err(map_win32_err(
                "RegGetValueW(size)",
                &full_location,
                probe_status,
            ));
        }

        let alloc_len = (cb_data as usize).max(8);
        let mut buffer = vec![0u8; alloc_len];
        let mut read_len = alloc_len as u32;

        let read_status = unsafe {
            RegGetValueW(
                key_guard.0,
                PCWSTR::null(),
                PCWSTR(value_w.as_ptr()),
                RRF_RT_ANY,
                Some(&mut reg_type),
                Some(buffer.as_mut_ptr() as *mut c_void),
                Some(&mut read_len),
            )
        };

        if read_status != ERROR_SUCCESS {
            return Err(map_win32_err(
                "RegGetValueW(data)",
                &full_location,
                read_status,
            ));
        }

        buffer.truncate(read_len as usize);

        if reg_type == REG_DWORD {
            if buffer.len() < 4 {
                return Err(AppError::RegistryApi {
                    operation: "Decode(REG_DWORD)".to_string(),
                    win32_code: 0,
                    detail: format!("Tamaño inválido ({}) en '{}'", buffer.len(), full_location),
                });
            }
            let val = u32::from_le_bytes([buffer[0], buffer[1], buffer[2], buffer[3]]);
            Ok(RegistryState::Present(RegistryData::Dword(val)))
        } else if reg_type == REG_QWORD {
            if buffer.len() < 8 {
                return Err(AppError::RegistryApi {
                    operation: "Decode(REG_QWORD)".to_string(),
                    win32_code: 0,
                    detail: format!("Tamaño inválido ({}) en '{}'", buffer.len(), full_location),
                });
            }
            let mut arr = [0u8; 8];
            arr.copy_from_slice(&buffer[..8]);
            Ok(RegistryState::Present(RegistryData::Qword(
                u64::from_le_bytes(arr),
            )))
        } else if reg_type == REG_SZ || reg_type == REG_EXPAND_SZ {
            let u16_len = buffer.len() / 2;
            let mut wide = Vec::with_capacity(u16_len);
            for chunk in buffer.chunks_exact(2) {
                wide.push(u16::from_le_bytes([chunk[0], chunk[1]]));
            }
            let text = String::from_utf16_lossy(&wide)
                .trim_end_matches('\0')
                .to_string();
            Ok(RegistryState::Present(RegistryData::Sz(text)))
        } else {
            Err(AppError::RegistryApi {
                operation: "Decode(REG_VALUE_TYPE)".to_string(),
                win32_code: reg_type.0,
                detail: format!(
                    "Tipo de registro no soportado ({}) en '{}'",
                    reg_type.0, full_location
                ),
            })
        }
    }

    pub fn write_value(
        hive: RegistryHive,
        subkey: &str,
        value_name: &str,
        data: &RegistryData,
    ) -> AppResult<()> {
        let full_location = format!("{}\\{}\\{}", hive.as_str(), subkey, value_name);
        let subkey_w = to_wide_null(subkey);
        let value_w = to_wide_null(value_name);

        let mut raw_hkey = HKEY::default();
        let create_status = unsafe {
            RegCreateKeyExW(
                hive_to_hkey(hive),
                PCWSTR(subkey_w.as_ptr()),
                None,
                PCWSTR::null(),
                REG_OPTION_NON_VOLATILE,
                KEY_SET_VALUE | KEY_QUERY_VALUE,
                None,
                &mut raw_hkey,
                None,
            )
        };

        if create_status != ERROR_SUCCESS {
            return Err(map_win32_err(
                "RegCreateKeyExW",
                &full_location,
                create_status,
            ));
        }

        let key_guard = OwnedHKey(raw_hkey);

        let (reg_type, raw_bytes): (REG_VALUE_TYPE, Vec<u8>) = match data {
            RegistryData::Dword(v) => (REG_DWORD, v.to_le_bytes().to_vec()),
            RegistryData::Qword(v) => (REG_QWORD, v.to_le_bytes().to_vec()),
            RegistryData::Sz(s) => {
                let wide = to_wide_null(s);
                let mut bytes = Vec::with_capacity(wide.len() * 2);
                for unit in wide {
                    bytes.extend_from_slice(&unit.to_le_bytes());
                }
                (REG_SZ, bytes)
            }
        };

        let set_status = unsafe {
            RegSetValueExW(
                key_guard.0,
                PCWSTR(value_w.as_ptr()),
                None,
                reg_type,
                Some(&raw_bytes),
            )
        };

        if set_status != ERROR_SUCCESS {
            return Err(map_win32_err("RegSetValueExW", &full_location, set_status));
        }

        Ok(())
    }

    pub fn delete_value_if_exists(
        hive: RegistryHive,
        subkey: &str,
        value_name: &str,
    ) -> AppResult<()> {
        let full_location = format!("{}\\{}\\{}", hive.as_str(), subkey, value_name);
        let subkey_w = to_wide_null(subkey);
        let value_w = to_wide_null(value_name);

        let mut raw_hkey = HKEY::default();
        let open_status = unsafe {
            RegOpenKeyExW(
                hive_to_hkey(hive),
                PCWSTR(subkey_w.as_ptr()),
                None,
                KEY_SET_VALUE | KEY_QUERY_VALUE,
                &mut raw_hkey,
            )
        };

        if open_status == ERROR_FILE_NOT_FOUND || open_status == ERROR_PATH_NOT_FOUND {
            return Ok(());
        }
        if open_status != ERROR_SUCCESS {
            return Err(map_win32_err("RegOpenKeyExW", &full_location, open_status));
        }

        let key_guard = OwnedHKey(raw_hkey);
        let del_status = unsafe { RegDeleteValueW(key_guard.0, PCWSTR(value_w.as_ptr())) };

        if del_status == ERROR_SUCCESS || del_status == ERROR_FILE_NOT_FOUND {
            Ok(())
        } else {
            Err(map_win32_err("RegDeleteValueW", &full_location, del_status))
        }
    }
}

#[cfg(windows)]
pub use win32_impl::{delete_value_if_exists, read_value, write_value};

#[cfg(not(windows))]
pub fn read_value(
    _hive: RegistryHive,
    _subkey: &str,
    _value_name: &str,
) -> AppResult<RegistryState> {
    Err(AppError::UnsupportedPlatform(
        "InputDash requiere Windows 10/11".to_string(),
    ))
}

#[cfg(not(windows))]
pub fn write_value(
    _hive: RegistryHive,
    _subkey: &str,
    _value_name: &str,
    _data: &RegistryData,
) -> AppResult<()> {
    Err(AppError::UnsupportedPlatform(
        "InputDash requiere Windows 10/11".to_string(),
    ))
}

#[cfg(not(windows))]
pub fn delete_value_if_exists(
    _hive: RegistryHive,
    _subkey: &str,
    _value_name: &str,
) -> AppResult<()> {
    Err(AppError::UnsupportedPlatform(
        "InputDash requiere Windows 10/11".to_string(),
    ))
}
