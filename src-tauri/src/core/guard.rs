use crate::core::catalog::{TweakDefinition, TweakLevel, TweakTarget};
use crate::error::{AppError, AppResult};
use crate::sys::registry::RegistryHive;

struct AllowedRegistryEntry {
    hive: RegistryHive,
    subkey: &'static str,
    value_name: &'static str,
}

/// Allowlist cerrada de claves del Registro permitidas por InputDash.
const ALLOWLIST_EXACT: &[AllowedRegistryEntry] = &[
    // Seguros
    AllowedRegistryEntry {
        hive: RegistryHive::HKCU,
        subkey: "Software\\Microsoft\\GameBar",
        value_name: "AutoGameModeEnabled",
    },
    AllowedRegistryEntry {
        hive: RegistryHive::HKCU,
        subkey: "System\\GameConfigStore",
        value_name: "GameDVR_Enabled",
    },
    AllowedRegistryEntry {
        hive: RegistryHive::HKLM,
        subkey: "SYSTEM\\CurrentControlSet\\Control\\Power\\PowerThrottling",
        value_name: "PowerThrottlingOff",
    },
    // Avanzados (requieren A/B en Etapa 5)
    AllowedRegistryEntry {
        hive: RegistryHive::HKLM,
        subkey: "SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\Multimedia\\SystemProfile",
        value_name: "SystemResponsiveness",
    },
    AllowedRegistryEntry {
        hive: RegistryHive::HKLM,
        subkey: "SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\Multimedia\\SystemProfile",
        value_name: "NetworkThrottlingIndex",
    },
    AllowedRegistryEntry {
        hive: RegistryHive::HKLM,
        subkey: "SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\Multimedia\\SystemProfile\\Tasks\\Games",
        value_name: "GPU Priority",
    },
    AllowedRegistryEntry {
        hive: RegistryHive::HKLM,
        subkey: "SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\Multimedia\\SystemProfile\\Tasks\\Games",
        value_name: "Priority",
    },
    AllowedRegistryEntry {
        hive: RegistryHive::HKLM,
        subkey: "SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\Multimedia\\SystemProfile\\Tasks\\Games",
        value_name: "Scheduling Category",
    },
    AllowedRegistryEntry {
        hive: RegistryHive::HKLM,
        subkey: "SYSTEM\\CurrentControlSet\\Control\\Session Manager\\kernel",
        value_name: "GlobalTimerResolutionRequests",
    },
];

/// Allowlist cerrada de planes de energía permitidos (High Performance y Ultimate Performance).
const ALLOWLIST_POWER_PLANS: &[&str] = &[
    "8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c",
    "e9a42b02-d5df-448d-aa00-03f14749eb61",
];

/// Allowlist cerrada de subgrupo/ajuste PowrProf (USB Selective Suspend).
const ALLOWLIST_POWER_SETTINGS: &[(&str, &str)] = &[(
    "2a737441-1930-4402-8d77-b2bebba308a3",
    "48e6b7a6-50f5-4782-a5d4-53bb8f07e226",
)];

/// Patrones explícitamente prohibidos por seguridad y estabilidad del sistema.
const FORBIDDEN_KEYWORDS: &[&str] = &[
    "windows defender",
    "windefend",
    "securityhealth",
    "windowsupdate",
    "firewallpolicy",
    "sharedaccess",
    "diagtrack",
    "maxprerenderedframes",
    "bcd00000000",
    "disabledynamictick",
    "useplatformclock",
];

fn is_tcpip_adapter_nagle_target(hive: RegistryHive, subkey: &str, value_name: &str) -> bool {
    const PREFIX: &str = "SYSTEM\\CurrentControlSet\\Services\\Tcpip\\Parameters\\Interfaces\\{";
    if hive != RegistryHive::HKLM {
        return false;
    }
    if !subkey.starts_with(PREFIX) || !subkey.ends_with('}') {
        return false;
    }
    let guid_inner = &subkey[PREFIX.len()..subkey.len() - 1];
    if guid_inner == "ADAPTER_GUID" {
        return value_name == "TcpAckFrequency" || value_name == "TCPNoDelay";
    }
    let valid_guid_chars = !guid_inner.is_empty()
        && !guid_inner.contains('\\')
        && guid_inner
            .chars()
            .all(|c| c.is_ascii_hexdigit() || c == '-');

    valid_guid_chars && (value_name == "TcpAckFrequency" || value_name == "TCPNoDelay")
}

pub fn validate_registry_location(
    hive: RegistryHive,
    subkey: &str,
    value_name: &str,
) -> AppResult<()> {
    if subkey.trim().is_empty() || value_name.trim().is_empty() {
        return Err(AppError::GuardViolation(
            "Ruta de registro o nombre de valor vacío".to_string(),
        ));
    }

    let lower_path = format!("{}\\{}", subkey.to_lowercase(), value_name.to_lowercase());
    for forbidden in FORBIDDEN_KEYWORDS {
        if lower_path.contains(forbidden) {
            return Err(AppError::GuardViolation(format!(
                "Objetivo explícitamente prohibido por política de seguridad ('{forbidden}')"
            )));
        }
    }

    let in_exact = ALLOWLIST_EXACT.iter().any(|entry| {
        entry.hive == hive
            && entry.subkey.eq_ignore_ascii_case(subkey)
            && entry.value_name.eq_ignore_ascii_case(value_name)
    });

    if in_exact || is_tcpip_adapter_nagle_target(hive, subkey, value_name) {
        Ok(())
    } else {
        Err(AppError::GuardViolation(format!(
            "Clave fuera de la allowlist cerrada: {}\\{}\\{}",
            hive.as_str(),
            subkey,
            value_name
        )))
    }
}

pub fn validate_power_plan_guid(guid: &str) -> AppResult<()> {
    let allowed = ALLOWLIST_POWER_PLANS
        .iter()
        .any(|g| g.eq_ignore_ascii_case(guid));
    if allowed {
        Ok(())
    } else {
        Err(AppError::GuardViolation(format!(
            "GUID de plan de energía fuera de la allowlist cerrada: {guid}"
        )))
    }
}

pub fn validate_power_setting_guids(subgroup: &str, setting: &str) -> AppResult<()> {
    let allowed = ALLOWLIST_POWER_SETTINGS.iter().any(|(sg, st)| {
        sg.eq_ignore_ascii_case(subgroup) && st.eq_ignore_ascii_case(setting)
    });
    if allowed {
        Ok(())
    } else {
        Err(AppError::GuardViolation(format!(
            "Ajuste PowrProf fuera de la allowlist cerrada: {subgroup} / {setting}"
        )))
    }
}

pub fn check_can_apply(tweak: &TweakDefinition) -> AppResult<()> {
    if tweak.nivel == TweakLevel::Bloqueado {
        return Err(AppError::GuardViolation(format!(
            "El tweak '{}' ({}) está catalogado como BLOQUEADO y jamás puede aplicarse.",
            tweak.nombre, tweak.id
        )));
    }

    match &tweak.target {
        TweakTarget::Registry {
            hive, ruta, valor, ..
        } => validate_registry_location(*hive, ruta, valor),
        TweakTarget::PowerPlan { guid } => validate_power_plan_guid(guid),
        TweakTarget::PowerSetting {
            subgroup_guid,
            setting_guid,
            ..
        } => validate_power_setting_guids(subgroup_guid, setting_guid),
        TweakTarget::Process { accion, .. } => {
            if accion == "priority_high" || accion == "cpu_affinity" {
                Ok(())
            } else {
                Err(AppError::GuardViolation(format!(
                    "Acción de proceso no permitida: {accion}"
                )))
            }
        }
        TweakTarget::Blocked { motivo_bloqueo } => Err(AppError::GuardViolation(format!(
            "Tweak bloqueado: {motivo_bloqueo}"
        ))),
    }
}

pub fn check_can_revert(tweak: &TweakDefinition) -> AppResult<()> {
    check_can_apply(tweak)
}
