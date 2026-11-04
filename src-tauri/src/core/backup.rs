use std::fs;
use std::path::{Path, PathBuf};

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};
use crate::sys::registry::{RegistryHive, RegistryState};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupSnapshot {
    pub schema_version: u32,
    pub tweak_id: String,
    pub tweak_name: String,
    pub created_at_utc: String,
    pub hive: RegistryHive,
    pub subkey: String,
    pub value_name: String,
    pub previous_state: RegistryState,
}

pub struct BackupOutcome {
    pub snapshot: BackupSnapshot,
    /// Indica si este snapshot fue creado nuevo en esta transacción (true)
    /// o si ya existía un backup original previo sin revertir (false).
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

pub fn has_snapshot(backups_dir: &Path, tweak_id: &str) -> AppResult<bool> {
    let path = snapshot_file_path(backups_dir, tweak_id)?;
    Ok(path.exists())
}

pub fn load_snapshot(backups_dir: &Path, tweak_id: &str) -> AppResult<Option<BackupSnapshot>> {
    let path = snapshot_file_path(backups_dir, tweak_id)?;
    if !path.exists() {
        return Ok(None);
    }

    let raw = fs::read_to_string(&path).map_err(|e| {
        AppError::Backup(format!(
            "No se pudo leer el archivo de backup '{}': {e}",
            path.display()
        ))
    })?;

    let snapshot: BackupSnapshot = serde_json::from_str(&raw).map_err(|e| {
        AppError::Backup(format!(
            "Archivo de backup corrupto en '{}': {e}",
            path.display()
        ))
    })?;

    Ok(Some(snapshot))
}

/// Captura el valor original antes de aplicar un tweak.
/// Si ya existe un snapshot activo para ese tweak, conserva el snapshot original
/// para nunca sobrescribir el valor verdadero previo del usuario.
pub fn ensure_snapshot(
    backups_dir: &Path,
    tweak_id: &str,
    tweak_name: &str,
    hive: RegistryHive,
    subkey: &str,
    value_name: &str,
    current_os_state: RegistryState,
) -> AppResult<BackupOutcome> {
    if let Some(existing) = load_snapshot(backups_dir, tweak_id)? {
        return Ok(BackupOutcome {
            snapshot: existing,
            newly_created: false,
        });
    }

    let snapshot = BackupSnapshot {
        schema_version: 1,
        tweak_id: tweak_id.to_string(),
        tweak_name: tweak_name.to_string(),
        created_at_utc: Utc::now().to_rfc3339(),
        hive,
        subkey: subkey.to_string(),
        value_name: value_name.to_string(),
        previous_state: current_os_state,
    };

    let final_path = snapshot_file_path(backups_dir, tweak_id)?;
    let tmp_path = backups_dir.join(format!("{tweak_id}.json.tmp"));

    let serialized = serde_json::to_string_pretty(&snapshot)
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

    Ok(BackupOutcome {
        snapshot,
        newly_created: true,
    })
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
