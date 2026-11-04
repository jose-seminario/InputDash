use crate::core::catalog::{TweakDefinition, TweakLevel, TweakTarget};
use crate::error::{AppError, AppResult};
use crate::sys::registry::RegistryHive;

struct AllowedRegistryEntry {
    hive: RegistryHive,
    subkey: &'static str,
    value_name: &'static str,
}

/// Allowlist cerrada de claves del Registro permitidas por InputDash.
/// Cualquier intento de lectura/escritura fuera de esta lista es rechazado antes de tocar Win32.
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
    // Avanzados (preparados en allowlist cerrada para etapas posteriores con validación A/B)
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

pub fn check_can_apply(tweak: &TweakDefinition) -> AppResult<()> {
    if tweak.nivel == TweakLevel::Bloqueado {
        return Err(AppError::GuardViolation(format!(
            "El tweak '{}' ({}) está catalogado como BLOQUEADO y no puede aplicarse.",
            tweak.nombre, tweak.id
        )));
    }

    match &tweak.target {
        TweakTarget::Registry {
            hive, ruta, valor, ..
        } => validate_registry_location(*hive, ruta, valor),
    }
}

pub fn check_can_revert(tweak: &TweakDefinition) -> AppResult<()> {
    if tweak.nivel == TweakLevel::Bloqueado {
        return Err(AppError::GuardViolation(format!(
            "El tweak '{}' ({}) está bloqueado y no admite operaciones.",
            tweak.nombre, tweak.id
        )));
    }

    match &tweak.target {
        TweakTarget::Registry {
            hive, ruta, valor, ..
        } => validate_registry_location(*hive, ruta, valor),
    }
}
