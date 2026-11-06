use std::fs;
use std::path::{Path, PathBuf};

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};
use crate::sys::registry::{RegistryHive, RegistryState};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "target_type", rename_all = "snake_case")]
pub enum BackupPayload {
    Registry {
        hive: RegistryHive,
        subkey: String,
        value_name: String,
        previous_state: RegistryState,
    },
    PowerPlan {
        previous_guid: String,
        previous_name: String,
        applied_guid: String,
        duplicated_guid: Option<String>,
    },
    PowerSetting {
        scheme_guid: String,
        subgroup_guid: String,
        setting_guid: String,
        previous_ac_index: u32,
        previous_dc_index: u32,
    },
}

impl BackupPayload {
    pub fn display_previous(&self) -> String {
        match self {
            Self::Registry { previous_state, .. } => previous_state.display_human(),
            Self::PowerPlan {
                previous_guid,
                previous_name,
                ..
            } => format!("{previous_name} ({previous_guid})"),
            Self::PowerSetting {
                scheme_guid,
                previous_ac_index,
                previous_dc_index,
                ..
            } => format!("AC={previous_ac_index}, DC={previous_dc_index} [Plan {scheme_guid}]"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupSnapshot {
    pub schema_version: u32,
    pub tweak_id: String,
    pub tweak_name: String,
    pub created_at_utc: String,
    pub payload: BackupPayload,
}

#[derive(Deserialize)]
struct LegacyV1Snapshot {
    tweak_id: String,
    tweak_name: String,
    created_at_utc: String,
    hive: RegistryHive,
    subkey: String,
    value_name: String,
    previous_state: RegistryState,
}

pub struct BackupOutcome {
    pub snapshot: BackupSnapshot,
    pub newly_created: bool,
}

fn sanitize_id(tweak_id: &str) -> AppResult<String> {
    if tweak_id.is_empty()
        || !tweak_id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return Err(AppError::Backup(format!(
            "Identificador de tweak inválido para archivo de backup: '{tweak_id}'"
        )));
    }
    Ok(tweak_id.to_string())
}

pub fn snapshot_file_path(backups_dir: &Path, tweak_id: &str) -> AppResult<PathBuf> {
    let safe_id = sanitize_id(tweak_id)?;
    Ok(backups_dir.join(format!("{safe_id}.json")))
}

pub fn load_snapshot(backups_dir: &Path, tweak_id: &str) -> AppResult<Option<BackupSnapshot>> {
    let path = snapshot_file_path(backups_dir, tweak_id)?;
    if !path.exists() {
        return Ok(None);
    }

    parse_snapshot_file(&path).map(Some)
}

fn parse_snapshot_file(path: &Path) -> AppResult<BackupSnapshot> {
    let raw = fs::read_to_string(path).map_err(|e| {
        AppError::Backup(format!(
            "No se pudo leer el archivo de backup '{}': {e}",
            path.display()
        ))
    })?;

    if let Ok(v2) = serde_json::from_str::<BackupSnapshot>(&raw) {
        return Ok(v2);
    }

    if let Ok(v1) = serde_json::from_str::<LegacyV1Snapshot>(&raw) {
        return Ok(BackupSnapshot {
            schema_version: 2,
            tweak_id: v1.tweak_id,
            tweak_name: v1.tweak_name,
            created_at_utc: v1.created_at_utc,
            payload: BackupPayload::Registry {
                hive: v1.hive,
                subkey: v1.subkey,
                value_name: v1.value_name,
                previous_state: v1.previous_state,
            },
        });
    }

    Err(AppError::Backup(format!(
        "Archivo de backup corrupto o formato desconocido en '{}'",
        path.display()
    )))
}

pub fn list_all_snapshots(backups_dir: &Path) -> AppResult<Vec<BackupSnapshot>> {
    if !backups_dir.exists() {
        return Ok(Vec::new());
    }

    let entries = fs::read_dir(backups_dir).map_err(|e| {
        AppError::Backup(format!(
            "No se pudo listar directorio de backups '{}': {e}",
            backups_dir.display()
        ))
    })?;

    let mut snapshots = Vec::new();
    for entry_res in entries {
        let entry = entry_res.map_err(|e| AppError::Backup(format!("Error leyendo entrada: {e}")))?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) == Some("json") {
            if let Ok(snap) = parse_snapshot_file(&path) {
                snapshots.push(snap);
            }
        }
    }

    snapshots.sort_by(|a, b| b.created_at_utc.cmp(&a.created_at_utc));
    Ok(snapshots)
}

pub fn save_snapshot_atomic(backups_dir: &Path, snapshot: &BackupSnapshot) -> AppResult<()> {
    let final_path = snapshot_file_path(backups_dir, &snapshot.tweak_id)?;
    let tmp_path = backups_dir.join(format!("{}.json.tmp", snapshot.tweak_id));

    let serialized = serde_json::to_string_pretty(snapshot)
        .map_err(|e| AppError::Backup(format!("Error serializando snapshot JSON: {e}")))?;

    fs::write(&tmp_path, serialized.as_bytes()).map_err(|e| {
        AppError::Backup(format!(
            "No se pudo escribir snapshot temporal '{}': {e}",
            tmp_path.display()
        ))
    })?;

    fs::rename(&tmp_path, &final_path).map_err(|e| {
        let _ = fs::remove_file(&tmp_path);
        AppError::Backup(format!(
            "No se pudo confirmar atómicamente el backup '{}': {e}",
            final_path.display()
        ))
    })?;

    Ok(())
}

/// Captura el valor original antes de aplicar un tweak.
/// Si ya existe un snapshot válido para el mismo contexto, preserva el valor original verdadero.
pub fn ensure_snapshot(
    backups_dir: &Path,
    tweak_id: &str,
    tweak_name: &str,
    payload: BackupPayload,
) -> AppResult<BackupOutcome> {
    if let Some(existing) = load_snapshot(backups_dir, tweak_id)? {
        let reuse = match (&existing.payload, &payload) {
            (
                BackupPayload::PowerSetting {
                    scheme_guid: old_scheme,
                    ..
                },
                BackupPayload::PowerSetting {
                    scheme_guid: new_scheme,
                    ..
                },
            ) => old_scheme.eq_ignore_ascii_case(new_scheme),
            _ => true,
        };

        if reuse {
            return Ok(BackupOutcome {
                snapshot: existing,
                newly_created: false,
            });
        }
    }

    let snapshot = BackupSnapshot {
        schema_version: 2,
        tweak_id: tweak_id.to_string(),
        tweak_name: tweak_name.to_string(),
        created_at_utc: Utc::now().to_rfc3339(),
        payload,
    };

    save_snapshot_atomic(backups_dir, &snapshot)?;

    Ok(BackupOutcome {
        snapshot,
        newly_created: true,
    })
}

pub fn write_or_replace_snapshot(
    backups_dir: &Path,
    tweak_id: &str,
    tweak_name: &str,
    payload: BackupPayload,
) -> AppResult<BackupSnapshot> {
    let snapshot = BackupSnapshot {
        schema_version: 2,
        tweak_id: tweak_id.to_string(),
        tweak_name: tweak_name.to_string(),
        created_at_utc: Utc::now().to_rfc3339(),
        payload,
    };
    save_snapshot_atomic(backups_dir, &snapshot)?;
    Ok(snapshot)
}

pub fn remove_snapshot(backups_dir: &Path, tweak_id: &str) -> AppResult<()> {
    let path = snapshot_file_path(backups_dir, tweak_id)?;
    if path.exists() {
        fs::remove_file(&path).map_err(|e| {
            AppError::Backup(format!(
                "No se pudo eliminar snapshot ya revertido '{}': {e}",
                path.display()
            ))
        })?;
    }
    Ok(())
}
