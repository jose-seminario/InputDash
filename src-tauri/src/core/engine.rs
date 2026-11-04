use serde::Serialize;

use crate::core::backup::{self, BackupSnapshot};
use crate::core::catalog::{Catalog, TweakDefinition};
use crate::core::guard;
use crate::core::log;
use crate::core::restore_point::{RestorePointManager, RestorePointOutcome};
use crate::db::Database;
use crate::error::{AppError, AppResult};
use crate::sys::registry::{self, RegistryHive, RegistryState};

#[derive(Debug, Clone, Serialize)]
pub struct TweakStatusView {
    pub definition: TweakDefinition,
    pub current_os_value: String,
    pub is_applied: bool,
    pub has_backup: bool,
    pub backup_original_value: Option<String>,
    pub status_error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MutationOutcome {
    pub tweak: TweakStatusView,
    pub restore_point_created: bool,
    pub restore_point_warning: Option<String>,
    pub message: String,
}

pub struct Engine {
    catalog: Catalog,
    db: Database,
    restore_points: RestorePointManager,
}

impl Engine {
    pub fn new(catalog: Catalog, db: Database) -> Self {
        Self {
            catalog,
            db,
            restore_points: RestorePointManager::new(),
        }
    }

    pub fn db(&self) -> &Database {
        &self.db
    }

    pub fn list_tweaks_with_status(&self) -> Vec<TweakStatusView> {
        self.catalog
            .all()
            .iter()
            .map(|tweak| self.inspect_tweak(tweak))
            .collect()
    }

    pub fn inspect_tweak(&self, tweak: &TweakDefinition) -> TweakStatusView {
        let backups_dir = self.db.backups_dir();
        let backup_info = backup::load_snapshot(&backups_dir, &tweak.id).ok().flatten();

        match tweak.target_registry_data() {
            Ok((hive, subkey, val_name, expected_target)) => {
                match guard::validate_registry_location(hive, subkey, val_name)
                    .and_then(|()| registry::read_value(hive, subkey, val_name))
                {
                    Ok(os_state) => {
                        let is_applied = os_state == RegistryState::Present(expected_target);
                        TweakStatusView {
                            definition: tweak.clone(),
                            current_os_value: os_state.display_human(),
                            is_applied,
                            has_backup: backup_info.is_some(),
                            backup_original_value: backup_info
                                .map(|b| b.previous_state.display_human()),
                            status_error: None,
                        }
                    }
                    Err(err) => TweakStatusView {
                        definition: tweak.clone(),
                        current_os_value: "Error de lectura".to_string(),
                        is_applied: false,
                        has_backup: backup_info.is_some(),
                        backup_original_value: backup_info
                            .map(|b| b.previous_state.display_human()),
                        status_error: Some(err.to_string()),
                    },
                }
            }
            Err(err) => TweakStatusView {
                definition: tweak.clone(),
                current_os_value: "Definición inválida".to_string(),
                is_applied: false,
                has_backup: backup_info.is_some(),
                backup_original_value: backup_info.map(|b| b.previous_state.display_human()),
                status_error: Some(err.to_string()),
            },
        }
    }

    /// Flujo transaccional estricto:
    /// `guard.check` -> `backup.snapshot` -> `restore_point` (1ª vez, no bloqueante si el SO lo tiene desactivado)
    /// -> `apply` -> `verify` -> `log`.
    /// Cualquier fallo en apply/verify revierte la transacción completa desde el snapshot JSON atómico.
    pub fn apply_tweak(&self, tweak_id: &str) -> AppResult<MutationOutcome> {
        let tweak = self.catalog.get_by_id(tweak_id)?;

        // 1. guard.check
        guard::check_can_apply(tweak)?;

        let (hive, subkey, value_name, target_data) = tweak.target_registry_data()?;
        let current_state = registry::read_value(hive, subkey, value_name)?;

        // 2. backup.snapshot (red de seguridad obligatoria y atómica)
        let backups_dir = self.db.backups_dir();
        let backup_outcome = backup::ensure_snapshot(
            &backups_dir,
            &tweak.id,
            &tweak.nombre,
            hive,
            subkey,
            value_name,
            current_state.clone(),
        )?;

        // 3. restore_point (1ª vez en la sesión; si Restaurar Sistema está desactivado en el SO,
        // registra advertencia explícita en SQLite y continúa con el backup JSON atómico)
        let (restore_point_created, restore_point_warning) = match self
            .restore_points
            .ensure_first_time("InputDash - Punto de restauracion previo a optimizacion")
        {
            RestorePointOutcome::Created => (true, None),
            RestorePointOutcome::AlreadyCreatedInSession => (false, None),
            RestorePointOutcome::SkippedWarning { code, reason } => {
                let warn_text = format!(
                    "No se pudo crear Punto de Restauración de Windows (código {code}: {reason}). Se continúa usando el backup JSON atómico como red de seguridad."
                );
                let _ = log::record_change(
                    &self.db,
                    &tweak.id,
                    &tweak.nombre,
                    "RESTORE_POINT_WARN",
                    &current_state.display_human(),
                    &current_state.display_human(),
                    true,
                    &warn_text,
                );
                (false, Some(warn_text))
            }
        };

        // 4. apply + 5. verify (con rollback transaccional ante cualquier error)
        let expected_state = RegistryState::Present(target_data.clone());
        let apply_and_verify = registry::write_value(hive, subkey, value_name, &target_data)
            .and_then(|()| {
                let reread = registry::read_value(hive, subkey, value_name)?;
                if reread != expected_state {
                    Err(AppError::VerificationMismatch {
                        tweak_id: tweak.id.clone(),
                        expected: expected_state.display_human(),
                        actual: reread.display_human(),
                    })
                } else {
                    Ok(reread)
                }
            });

        let verified_state = match apply_and_verify {
            Ok(state) => state,
            Err(step_err) => {
                let rollback_res = restore_registry_from_snapshot(&backup_outcome.snapshot);
                if backup_outcome.newly_created && rollback_res.is_ok() {
                    let _ = backup::remove_snapshot(&backups_dir, &tweak.id);
                }

                let _ = log::record_change(
                    &self.db,
                    &tweak.id,
                    &tweak.nombre,
                    "ROLLBACK_ERROR",
                    &current_state.display_human(),
                    &expected_state.display_human(),
                    false,
                    &format!("Fallo durante apply/verify: {step_err}"),
                );

                return match rollback_res {
                    Ok(()) => Err(AppError::TransactionRolledBack {
                        tweak_id: tweak.id.clone(),
                        cause: step_err.to_string(),
                    }),
                    Err(rb_err) => Err(AppError::RollbackFailure {
                        tweak_id: tweak.id.clone(),
                        cause: step_err.to_string(),
                        rollback_error: rb_err.to_string(),
                    }),
                };
            }
        };

        // 6. log
        let apply_details = if restore_point_created {
            "Punto de restauración creado + valor aplicado y verificado en Registro."
        } else if restore_point_warning.is_some() {
            "Valor aplicado y verificado (respaldado por snapshot JSON atómico; Restaurar Sistema no disponible en SO)."
        } else {
            "Valor aplicado y verificado en Registro con backup activo."
        };

        log::record_change(
            &self.db,
            &tweak.id,
            &tweak.nombre,
            "APPLY",
            &current_state.display_human(),
            &verified_state.display_human(),
            true,
            apply_details,
        )?;

        let updated_view = self.inspect_tweak(tweak);
        Ok(MutationOutcome {
            tweak: updated_view,
            restore_point_created,
            restore_point_warning,
            message: format!(
                "Tweak '{}' aplicado y verificado correctamente.",
                tweak.nombre
            ),
        })
    }

    /// Revierte un tweak restaurando exactamente el snapshot previo y verificando la lectura.
    pub fn revert_tweak(&self, tweak_id: &str) -> AppResult<MutationOutcome> {
        let tweak = self.catalog.get_by_id(tweak_id)?;

        // 1. guard.check
        guard::check_can_revert(tweak)?;

        let (hive, subkey, value_name, _) = tweak.target_registry_data()?;
        let before_revert = registry::read_value(hive, subkey, value_name)?;

        // 2. cargar snapshot obligatorio
        let backups_dir = self.db.backups_dir();
        let snapshot = backup::load_snapshot(&backups_dir, &tweak.id)?.ok_or_else(|| {
            AppError::Backup(format!(
                "No existe snapshot de backup para '{}'. Por seguridad no se revierte a ciegas.",
                tweak.nombre
            ))
        })?;

        // 3. restaurar estado exacto previo
        restore_registry_from_snapshot(&snapshot)?;

        // 4. verify post-reversión
        let after_revert = verify_restored_state(hive, subkey, value_name, &snapshot)?;

        // 5. limpiar snapshot activo y registrar en log
        backup::remove_snapshot(&backups_dir, &tweak.id)?;

        log::record_change(
            &self.db,
            &tweak.id,
            &tweak.nombre,
            "REVERT",
            &before_revert.display_human(),
            &after_revert.display_human(),
            true,
            "Restaurado exactamente desde snapshot JSON propio y verificado.",
        )?;

        let updated_view = self.inspect_tweak(tweak);
        Ok(MutationOutcome {
            tweak: updated_view,
            restore_point_created: false,
            restore_point_warning: None,
            message: format!("Tweak '{}' revertido a su estado original.", tweak.nombre),
        })
    }
}

fn restore_registry_from_snapshot(snapshot: &BackupSnapshot) -> AppResult<()> {
    match &snapshot.previous_state {
        RegistryState::Present(prev_data) => registry::write_value(
            snapshot.hive,
            &snapshot.subkey,
            &snapshot.value_name,
            prev_data,
        ),
        RegistryState::KeyMissing | RegistryState::ValueMissing => registry::delete_value_if_exists(
            snapshot.hive,
            &snapshot.subkey,
            &snapshot.value_name,
        ),
    }
}

fn verify_restored_state(
    hive: RegistryHive,
    subkey: &str,
    value_name: &str,
    snapshot: &BackupSnapshot,
) -> AppResult<RegistryState> {
    let reread = registry::read_value(hive, subkey, value_name)?;
    let matches = match (&snapshot.previous_state, &reread) {
        (RegistryState::Present(exp), RegistryState::Present(act)) => exp == act,
        (
            RegistryState::KeyMissing | RegistryState::ValueMissing,
            RegistryState::KeyMissing | RegistryState::ValueMissing,
        ) => true,
        _ => false,
    };

    if !matches {
        return Err(AppError::VerificationMismatch {
            tweak_id: snapshot.tweak_id.clone(),
            expected: snapshot.previous_state.display_human(),
            actual: reread.display_human(),
        });
    }

    Ok(reread)
}
