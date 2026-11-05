use core::ffi::c_void;
use core::ptr;
use serde::{Deserialize, Serialize};

use crate::error::AppResult;
use crate::sys::registry::{read_value, RegistryHive, RegistryState};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriversInfo {
    pub active_nic_name: String,
    pub active_nic_driver: String,
    pub primary_audio_name: String,
    pub primary_audio_driver: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NicModerationInfo {
    pub adapter_name: String,
    pub value_str: String,
    pub is_enabled: bool,
    pub verdict: String,
    pub explanation: String,
}

#[link(name = "iphlpapi")]
extern "system" {
    fn GetBestInterface(dwDestAddr: u32, pdwBestIfIndex: *mut u32) -> u32;
    fn GetAdaptersAddresses(
        Family: u32,
        Flags: u32,
        Reserved: *mut c_void,
        AdapterAddresses: *mut IP_ADAPTER_ADDRESSES,
        SizePointer: *mut u32,
    ) -> u32;
}

#[repr(C)]
struct IP_ADAPTER_ADDRESSES {
    Length: u32,
    IfIndex: u32,
    Next: *mut IP_ADAPTER_ADDRESSES,
    AdapterName: *mut u8,
    // remaining fields omitted for sizing
}

const NET_CLASS_KEY: &str =
    "SYSTEM\\CurrentControlSet\\Control\\Class\\{4d36e972-e325-11ce-bfc1-08002be10318}";
const MEDIA_CLASS_KEY: &str =
    "SYSTEM\\CurrentControlSet\\Control\\Class\\{4d36e96c-e325-11ce-bfc1-08002be10318}";

pub fn get_drivers_info() -> AppResult<DriversInfo> {
    let (nic_name, nic_ver, _nic_sub) = find_active_nic_details();
    let (audio_name, audio_ver) = find_primary_audio_details();

    Ok(DriversInfo {
        active_nic_name: nic_name,
        active_nic_driver: nic_ver,
        primary_audio_name: audio_name,
        primary_audio_driver: audio_ver,
    })
}

pub fn get_nic_moderation() -> AppResult<NicModerationInfo> {
    let (nic_name, _, nic_sub) = find_active_nic_details();

    let mut raw_val = String::new();
    let mut found = false;

    // Inspect the active NIC subkey first, or scan Ethernet/WiFi adapters in Net class
    let candidate_subkeys = if !nic_sub.is_empty() {
        vec![nic_sub.clone()]
    } else {
        (0..25).map(|i| format!("{i:04}")).collect()
    };

    let mut target_adapter_name = nic_name.clone();

    for sub in &candidate_subkeys {
        let subkey = format!("{NET_CLASS_KEY}\\{sub}");

        // Check if InterruptModeration exists
        for val_name in &["*InterruptModeration", "InterruptModeration"] {
            match read_value(RegistryHive::HKLM, &subkey, val_name) {
                Ok(RegistryState::Present(crate::sys::registry::RegistryData::Dword(v))) => {
                    raw_val = v.to_string();
                    found = true;
                    if let Ok(RegistryState::Present(crate::sys::registry::RegistryData::Sz(desc))) =
                        read_value(RegistryHive::HKLM, &subkey, "DriverDesc")
                    {
                        target_adapter_name = desc;
                    }
                    break;
                }
                Ok(RegistryState::Present(crate::sys::registry::RegistryData::Sz(s))) => {
                    raw_val = s;
                    found = true;
                    if let Ok(RegistryState::Present(crate::sys::registry::RegistryData::Sz(desc))) =
                        read_value(RegistryHive::HKLM, &subkey, "DriverDesc")
                    {
                        target_adapter_name = desc;
                    }
                    break;
                }
                _ => {}
            }
        }

        if found {
            break;
        }
    }

    if !found {
        return Ok(NicModerationInfo {
            adapter_name: target_adapter_name,
            value_str: "No configurado".to_string(),
            is_enabled: false,
            verdict: "No configurado (Automático por el SO / Controlador)".to_string(),
            explanation: "El parámetro *InterruptModeration no está definido explícitamente en el registro. El adaptador opera en su modo predeterminado de fábrica.".to_string(),
        });
    }

    let is_enabled = raw_val.trim() == "1";
    let (verdict, explanation) = if is_enabled {
        (
            "Interrupt Moderation: ACTIVADO".to_string(),
            "Riesgo de retraso de 2-8 ms: La tarjeta agrupa paquetes entrantes en búfer antes de notificar a la CPU. Ahorra ciclos de procesador pero añade micro-retraso en juegos competitivos. (Revisable con prueba A/B)".to_string(),
        )
    } else {
        (
            "Interrupt Moderation: DESACTIVADO".to_string(),
            "Latencia mínima de interrupción: Cada paquete de red genera interrupción inmediata al CPU para procesamiento en tiempo real sin colas artificiales.".to_string(),
        )
    };

    Ok(NicModerationInfo {
        adapter_name: target_adapter_name,
        value_str: raw_val,
        is_enabled,
        verdict,
        explanation,
    })
}

fn find_active_nic_details() -> (String, String, String) {
    let active_guid = get_active_nic_instance_guid();

    // Scan Net class subkeys (0000..0030)
    let mut fallback_nic: Option<(String, String, String)> = None;

    for i in 0..30 {
        let sub = format!("{i:04}");
        let subkey = format!("{NET_CLASS_KEY}\\{sub}");

        let desc = match read_value(RegistryHive::HKLM, &subkey, "DriverDesc") {
            Ok(RegistryState::Present(crate::sys::registry::RegistryData::Sz(d))) => d,
            _ => continue,
        };

        let ver = match read_value(RegistryHive::HKLM, &subkey, "DriverVersion") {
            Ok(RegistryState::Present(crate::sys::registry::RegistryData::Sz(v))) => v,
            _ => "No disponible".to_string(),
        };

        let inst_id = match read_value(RegistryHive::HKLM, &subkey, "NetCfgInstanceId") {
            Ok(RegistryState::Present(crate::sys::registry::RegistryData::Sz(id))) => id,
            _ => String::new(),
        };

        // Match by active GUID
        if let Some(ref target_guid) = active_guid {
            if inst_id.eq_ignore_ascii_case(target_guid) {
                return (desc, ver, sub);
            }
        }

        // Store candidate fallback if it looks like a physical adapter (Ethernet/WiFi)
        let is_virtual = desc.to_lowercase().contains("virtual")
            || desc.to_lowercase().contains("miniport")
            || desc.to_lowercase().contains("kernel debug");

        if !is_virtual && fallback_nic.is_none() && ver != "No disponible" {
            fallback_nic = Some((desc, ver, sub));
        }
    }

    fallback_nic.unwrap_or_else(|| {
        (
            "Adaptador de Red Primario".to_string(),
            "No disponible".to_string(),
            String::new(),
        )
    })
}

fn get_active_nic_instance_guid() -> Option<String> {
    // 0x08080808 = 8.8.8.8
    let mut if_idx = 0u32;
    let res = unsafe { GetBestInterface(0x08080808, &mut if_idx) };
    if res != 0 {
        return None;
    }

    let mut buf_len = 15000u32;
    let mut buffer = vec![0u8; buf_len as usize];

    let g_res = unsafe {
        GetAdaptersAddresses(
            0, // AF_UNSPEC
            0,
            ptr::null_mut(),
            buffer.as_mut_ptr() as *mut IP_ADAPTER_ADDRESSES,
            &mut buf_len,
        )
    };

    if g_res != 0 && g_res != 234 {
        return None;
    }

    if g_res == 234 {
        buffer = vec![0u8; buf_len as usize];
        let retry = unsafe {
            GetAdaptersAddresses(
                0,
                0,
                ptr::null_mut(),
                buffer.as_mut_ptr() as *mut IP_ADAPTER_ADDRESSES,
                &mut buf_len,
            )
        };
        if retry != 0 {
            return None;
        }
    }

    let mut curr_ptr = buffer.as_ptr() as *const IP_ADAPTER_ADDRESSES;
    while !curr_ptr.is_null() {
        let curr = unsafe { &*curr_ptr };
        if curr.IfIndex == if_idx {
            if !curr.AdapterName.is_null() {
                let mut len = 0;
                while unsafe { *curr.AdapterName.add(len) } != 0 {
                    len += 1;
                }
                let slice = unsafe { std::slice::from_raw_parts(curr.AdapterName, len) };
                if let Ok(guid_str) = std::str::from_utf8(slice) {
                    return Some(guid_str.trim().to_string());
                }
            }
        }
        curr_ptr = curr.Next;
    }

    None
}

fn find_primary_audio_details() -> (String, String) {
    let mut best_audio: Option<(String, String, i32)> = None;

    for i in 0..25 {
        let sub = format!("{i:04}");
        let subkey = format!("{MEDIA_CLASS_KEY}\\{sub}");

        let desc = match read_value(RegistryHive::HKLM, &subkey, "DriverDesc") {
            Ok(RegistryState::Present(crate::sys::registry::RegistryData::Sz(d))) => d,
            _ => continue,
        };

        let ver = match read_value(RegistryHive::HKLM, &subkey, "DriverVersion") {
            Ok(RegistryState::Present(crate::sys::registry::RegistryData::Sz(v))) => v,
            _ => "No disponible".to_string(),
        };

        let lower = desc.to_lowercase();
        // Priority ranking
        let rank = if lower.contains("realtek") {
            10
        } else if lower.contains("high definition audio") {
            8
        } else if lower.contains("smart sound") || lower.contains("intel") {
            7
        } else if lower.contains("nvidia") || lower.contains("amd") {
            6
        } else if lower.contains("creative") || lower.contains("focusrite") {
            9
        } else if lower.contains("proxy") || lower.contains("clock") {
            -1
        } else {
            1
        };

        if rank > 0 {
            if best_audio.as_ref().map(|b| b.2).unwrap_or(-1) < rank {
                best_audio = Some((desc, ver, rank));
            }
        }
    }

    match best_audio {
        Some((name, ver, _)) => (name, ver),
        None => ("Dispositivo de Audio Principal".to_string(), "No disponible".to_string()),
    }
}
