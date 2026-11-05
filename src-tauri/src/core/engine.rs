use std::collections::HashSet;

use serde::Serialize;

use crate::core::backup::{self, BackupPayload, BackupSnapshot};
use crate::core::catalog::{Catalog, TweakDefinition, TweakLevel, TweakTarget};
use crate::core::guard;
use crate::core::log;
use crate::core::restore_point::{RestorePointManager, RestorePointOutcome};
use crate::db::Database;
use crate::error::{AppError, AppResult};
use crate::sys::power::{self, PowerPlanInfo};
use crate::sys::registry::{self, RegistryHive, RegistryState};

#[derive(Debug, Clone, Serialize)]
pub struct TweakStatusView {
    pub definition: TweakDefinition,
    pub target_summary: String,
    pub expected_value_display: String,
    pub current_os_value: String,
    pub is_applied: bool,
    pub has_backup: bool,
    pub backup_original_value: Option<String>,
    pub status_error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MutationOutcome {
    pub tweak: TweakStatusView,
    pub all_tweaks: Vec<TweakStatusView>,
    pub restore_point_created: bool,
    pub restore_point_warning: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PresetOutcome {
    pub preset_id: String,
    pub applied_ids: Vec<String>,
    pub all_tweaks: Vec<TweakStatusView>,
    pub restore_point_created: bool,
    pub restore_point_warning: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RevertAllOutcome {
    pub reverted_count: usize,
    pub reverted_ids: Vec<String>,
    pub errors: Vec<String>,
    pub all_tweaks: Vec<TweakStatusView>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SimulationItem {
    pub tweak_id: String,
    pub tweak_name: String,
    pub nivel: TweakLevel,
    pub target_summary: String,
    pub current_os_value: String,
    pub proposed_value: String,
    pub would_change: bool,
    pub requires_admin: bool,
    pub reversion_preview: String,
    pub note: Option<String>,
}

pub struct Engine {
    catalog: Catalog,
    db: Database,
    restore_points: RestorePointManager,
    active_tokens: std::sync::Mutex<HashSet<String>>,
}

impl Engine {
    pub fn new(catalog: Catalog, db: Database) -> Self {
        Self {
            catalog,
            db,
            restore_points: RestorePointManager::new(),
            active_tokens: std::sync::Mutex::new(HashSet::new()),
        }
    }

    pub fn register_session_token(&self, token: &str) {
        if let Ok(mut tokens) = self.active_tokens.lock() {
            tokens.insert(token.to_string());
        }
    }

    pub fn invalidate_session_token(&self, token: &str) {
        if let Ok(mut tokens) = self.active_tokens.lock() {
            tokens.remove(token);
        }
    }

    pub fn is_token_active(&self, token: &str) -> bool {
        self.active_tokens
            .lock()
            .map(|tokens| tokens.contains(token))
            .unwrap_or(false)
    }

    pub fn db(&self) -> &Database {
        &self.db
    }

    pub fn preset_tweak_ids(preset_id: &str) -> AppResult<&'static [&'static str]> {
        match preset_id {
            "equilibrado" => Ok(&[
                "power_plan_high_performance",
                "usb_selective_suspend_off",
                "game_mode_on",
                "game_dvr_off",
            ]),
            "competitivo" => Ok(&[
                "power_plan_ultimate",
                "usb_selective_suspend_off",
                "power_throttling_off",
                "game_mode_on",
                "game_dvr_off",
            ]),
            other => Err(AppError::InvalidPreset(other.to_string())),
        }
    }

    pub fn list_tweaks_with_status(&self) -> Vec<TweakStatusView> {
        let active_plan_res = power::get_active_plan();
        self.catalog
            .all()
            .iter()
            .map(|tweak| self.inspect_tweak_with_plan(tweak, active_plan_res.as_ref().ok()))
            .collect()
    }

    pub fn list_active_backups(&self) -> AppResult<Vec<BackupSnapshot>> {
        backup::list_all_snapshots(&self.db.backups_dir())
    }

    pub fn inspect_tweak(&self, tweak: &TweakDefinition) -> TweakStatusView {
        let active_plan = power::get_active_plan().ok();
        self.inspect_tweak_with_plan(tweak, active_plan.as_ref())
    }

    fn inspect_tweak_with_plan(
        &self,
        tweak: &TweakDefinition,
        cached_plan: Option<&PowerPlanInfo>,
    ) -> TweakStatusView {
        let backups_dir = self.db.backups_dir();
        let backup_info = backup::load_snapshot(&backups_dir, &tweak.id).ok().flatten();
        let target_summary = tweak.target_summary();
        let expected_value_display = tweak.target_expected_display();

        match &tweak.target {
            TweakTarget::Registry {
                hive,
                ruta,
                valor,
                dato,
                tipo_dato,
            } => {
                // Etapa 5/7: adaptadores TCP dinámicos por GUID
                if ruta.contains("{ADAPTER_GUID}") {
                    return TweakStatusView {
                        definition: tweak.clone(),
                        target_summary,
                        expected_value_display,
                        current_os_value: "Por interfaz de red TCP (Etapa 5/7)".to_string(),
                        is_applied: false,
                        has_backup: backup_info.is_some(),
                        backup_original_value: backup_info
                            .map(|b| b.payload.display_previous()),
                        status_error: None,
                    };
                }

                match tweak.parse_registry_target(*hive, *tipo_dato, dato) {
                    Ok((_, expected_data)) => match guard::validate_registry_location(
                        *hive, ruta, valor,
                    )
                    .and_then(|()| registry::read_value(*hive, ruta, valor))
                    {
                        Ok(os_state) => {
                            let is_applied = os_state == RegistryState::Present(expected_data);
                            TweakStatusView {
                                definition: tweak.clone(),
                                target_summary,
                                expected_value_display,
                                current_os_value: os_state.display_human(),
                                is_applied,
                                has_backup: backup_info.is_some(),
                                backup_original_value: backup_info
                                    .map(|b| b.payload.display_previous()),
                                status_error: None,
                            }
                        }
                        Err(err) => TweakStatusView {
                            definition: tweak.clone(),
                            target_summary,
                            expected_value_display,
                            current_os_value: "Error de lectura".to_string(),
                            is_applied: false,
                            has_backup: backup_info.is_some(),
                            backup_original_value: backup_info
                                .map(|b| b.payload.display_previous()),
                            status_error: Some(err.to_string()),
                        },
                    },
                    Err(err) => TweakStatusView {
                        definition: tweak.clone(),
                        target_summary,
                        expected_value_display,
                        current_os_value: "Definición inválida".to_string(),
                        is_applied: false,
                        has_backup: backup_info.is_some(),
                        backup_original_value: backup_info.map(|b| b.payload.display_previous()),
                        status_error: Some(err.to_string()),
                    },
                }
            }
            TweakTarget::PowerPlan { guid } => {
                let plan_opt = cached_plan
                    .cloned()
                    .or_else(|| power::get_active_plan().ok());
                match plan_opt {
                    Some(active) => {
                        let matches_template = active.guid.eq_ignore_ascii_case(guid);
                        let matches_dup = backup_info
                            .as_ref()
                            .and_then(|b| match &b.payload {
                                BackupPayload::PowerPlan { applied_guid, .. } => {
                                    Some(active.guid.eq_ignore_ascii_case(applied_guid))
                                }
                                _ => None,
                            })
                            .unwrap_or(false);

                        TweakStatusView {
                            definition: tweak.clone(),
                            target_summary,
                            expected_value_display,
                            current_os_value: format!("{} ({})", active.friendly_name, active.guid),
                            is_applied: matches_template || matches_dup,
                            has_backup: backup_info.is_some(),
                            backup_original_value: backup_info
                                .map(|b| b.payload.display_previous()),
                            status_error: None,
                        }
                    }
                    None => TweakStatusView {
                        definition: tweak.clone(),
                        target_summary,
                        expected_value_display,
                        current_os_value: "No disponible".to_string(),
                        is_applied: false,
                        has_backup: backup_info.is_some(),
                        backup_original_value: backup_info.map(|b| b.payload.display_previous()),
                        status_error: Some(
                            "No se pudo consultar el plan activo vía PowrProf".to_string(),
                        ),
                    },
                }
            }
            TweakTarget::PowerSetting {
                subgroup_guid,
                setting_guid,
                ac_index,
                dc_index,
            } => match power::read_setting_on_active_scheme(subgroup_guid, setting_guid) {
                Ok(indices) => {
                    let is_applied =
                        indices.ac_index == *ac_index && indices.dc_index == *dc_index;
                    TweakStatusView {
                        definition: tweak.clone(),
                        target_summary,
                        expected_value_display,
                        current_os_value: indices.display_human(),
                        is_applied,
                        has_backup: backup_info.is_some(),
                        backup_original_value: backup_info.map(|b| b.payload.display_previous()),
                        status_error: None,
                    }
                }
                Err(err) => TweakStatusView {
                    definition: tweak.clone(),
                    target_summary,
                    expected_value_display,
                    current_os_value: "Error leyendo PowrProf".to_string(),
                    is_applied: false,
                    has_backup: backup_info.is_some(),
                    backup_original_value: backup_info.map(|b| b.payload.display_previous()),
                    status_error: Some(err.to_string()),
                },
            },
            TweakTarget::Process { .. } => {
                // Etapa 6: inspección de prioridad/CPU Sets sobre proceso en ejecución
                TweakStatusView {
                    definition: tweak.clone(),
                    target_summary,
                    expected_value_display,
                    current_os_value: "Se aplica solo al detectar el juego (Etapa 6)".to_string(),
                    is_applied: false,
                    has_backup: false,
                    backup_original_value: None,
                    status_error: None,
                }
            }
            TweakTarget::Blocked { .. } => TweakStatusView {
                definition: tweak.clone(),
                target_summary,
                expected_value_display,
                current_os_value: "Protegido / No se modifica".to_string(),
                is_applied: false,
                has_backup: false,
                backup_original_value: None,
                status_error: None,
            },
        }
    }

    fn trigger_session_restore_point(
        &self,
        tweak: &TweakDefinition,
        current_display: &str,
    ) -> (bool, Option<String>) {
        match self
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
                    current_display,
                    current_display,
                    true,
                    &warn_text,
                );
                (false, Some(warn_text))
            }
        }
    }

    pub fn create_manual_restore_point(&self) -> AppResult<String> {
        self.restore_points
            .create_manual("InputDash - Punto de restauracion manual (Seguridad)")?;

        let msg = "Punto de restauración de Windows creado correctamente.".to_string();
        log::record_change(
            &self.db,
            "system_restore_point",
            "Punto de Restauración de Windows",
            "RESTORE_POINT",
            "Estado actual del SO",
            "Punto creado vía srclient.dll",
            true,
            &msg,
        )?;

        Ok(msg)
    }

    /// Aplica un tweak verificando si es de nivel Avanzado.
    /// Si es Avanzado, exige un session_token activo originado en una sesión A/B o Test Ciego (Etapa 5).
    pub fn apply_tweak_with_token(
        &self,
        tweak_id: &str,
        session_token: Option<&str>,
    ) -> AppResult<MutationOutcome> {
        let tweak = self.catalog.get_by_id(tweak_id)?;

        // 1. guard.check (bloquea inmediatamente los ítems Bloqueados y claves fuera de allowlist)
        guard::check_can_apply(tweak)?;

        // Candado estricto de Etapa 5: los tweaks Avanzados solo se aplican dentro de una sesión A/B o Test Ciego con token activo
        if tweak.nivel == TweakLevel::Avanzado {
            let is_valid = session_token
                .map(|tok| self.is_token_active(tok))
                .unwrap_or(false);

            if !is_valid {
                return Err(AppError::RequiresAbTest(
                    "Este tweak es de nivel Avanzado y requiere una sesión A/B o Test Ciego activa con token válido.".to_string(),
                ));
            }
        }

        match &tweak.target {
            TweakTarget::Registry {
                hive,
                ruta,
                valor,
                dato,
                tipo_dato,
            } => self.apply_registry_tweak(tweak, *hive, ruta, valor, dato, *tipo_dato),
            TweakTarget::PowerPlan { guid } => self.apply_power_plan_tweak(tweak, guid),
            TweakTarget::PowerSetting {
                subgroup_guid,
                setting_guid,
                ac_index,
                dc_index,
            } => self.apply_power_setting_tweak(
                tweak,
                subgroup_guid,
                setting_guid,
                *ac_index,
                *dc_index,
            ),
            TweakTarget::Process { .. } => {
                // Reservado para Etapa 6 (sys/process.rs + profiles/manager.rs)
                Err(AppError::StagePending(
                    "Los ajustes dinámicos por proceso de juego pertenecen a la Etapa 6".to_string(),
                ))
            }
            TweakTarget::Blocked { .. } => Err(AppError::GuardViolation(
                "Bloqueado por seguridad. Ver Anti-mitos".to_string(),
            )),
        }
    }

    pub fn apply_tweak(&self, tweak_id: &str) -> AppResult<MutationOutcome> {
        self.apply_tweak_with_token(tweak_id, None)
    }

    /// Aplica un tweak dentro de una sesión A/B controlada.
    /// Permite evaluar tanto tweaks Seguros como Avanzados, ejecutando el flujo transaccional
    /// completo: guard -> backup JSON atómico -> restore_point -> apply -> verify -> log.
    pub fn apply_tweak_ab(&self, tweak_id: &str, token: Option<&str>) -> AppResult<MutationOutcome> {
        self.apply_tweak_with_token(tweak_id, token)
    }

    fn apply_registry_tweak(
        &self,
        tweak: &TweakDefinition,
        hive: RegistryHive,
        subkey: &str,
        value_name: &str,
        dato: &serde_json::Value,
        tipo_dato: crate::sys::registry::RegistryValueKind,
    ) -> AppResult<MutationOutcome> {
        let (_, target_data) = tweak.parse_registry_target(hive, tipo_dato, dato)?;
        let current_state = registry::read_value(hive, subkey, value_name)?;
        let current_human = current_state.display_human();

        let backups_dir = self.db.backups_dir();
        let backup_outcome = backup::ensure_snapshot(
            &backups_dir,
            &tweak.id,
            &tweak.nombre,
            BackupPayload::Registry {
                hive,
                subkey: subkey.to_string(),
                value_name: value_name.to_string(),
                previous_state: current_state.clone(),
            },
        )?;

        let (restore_point_created, restore_point_warning) =
            self.trigger_session_restore_point(tweak, &current_human);

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
                let rollback_res = restore_from_snapshot(&backup_outcome.snapshot);
                if backup_outcome.newly_created && rollback_res.is_ok() {
                    let _ = backup::remove_snapshot(&backups_dir, &tweak.id);
                }

                let _ = log::record_change(
                    &self.db,
                    &tweak.id,
                    &tweak.nombre,
                    "ROLLBACK_ERROR",
                    &current_human,
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

        log::record_change(
            &self.db,
            &tweak.id,
            &tweak.nombre,
            "APPLY",
            &current_human,
            &verified_state.display_human(),
            true,
            "Valor aplicado y verificado en Registro con backup JSON atómico.",
        )?;

        let all_tweaks = self.list_tweaks_with_status();
        let updated_view = all_tweaks
            .iter()
            .find(|t| t.definition.id == tweak.id)
            .cloned()
            .unwrap_or_else(|| self.inspect_tweak(tweak));

        Ok(MutationOutcome {
            tweak: updated_view,
            all_tweaks,
            restore_point_created,
            restore_point_warning,
            message: format!(
                "Tweak '{}' aplicado y verificado correctamente.",
                tweak.nombre
            ),
        })
    }

    fn apply_power_plan_tweak(
        &self,
        tweak: &TweakDefinition,
        target_guid: &str,
    ) -> AppResult<MutationOutcome> {
        let current_plan = power::get_active_plan()?;
        let current_human = format!("{} ({})", current_plan.friendly_name, current_plan.guid);

        let backups_dir = self.db.backups_dir();

        // Si ya había un snapshot de otro plan de energía de InputDash (ej. pasa de High Performance a Ultimate),
        // heredamos el plan original verdadero del usuario previo a InputDash para no perder su estado inicial.
        let mut true_original_guid = current_plan.guid.clone();
        let mut true_original_name = current_plan.friendly_name.clone();
        let mut superseded_plan_snapshots: Vec<(String, Option<String>)> = Vec::new();

        for existing in backup::list_all_snapshots(&backups_dir)? {
            if let BackupPayload::PowerPlan {
                previous_guid,
                previous_name,
                duplicated_guid,
                ..
            } = existing.payload
            {
                true_original_guid = previous_guid;
                true_original_name = previous_name;
                if existing.tweak_id != tweak.id {
                    superseded_plan_snapshots.push((existing.tweak_id, duplicated_guid));
                }
            }
        }

        let (restore_point_created, restore_point_warning) =
            self.trigger_session_restore_point(tweak, &current_human);

        let apply_res = power::activate_or_duplicate_plan(target_guid)?;

        // Verify post-activación releyendo desde PowrProf
        let reread_plan = power::get_active_plan()?;
        if !reread_plan
            .guid
            .eq_ignore_ascii_case(&apply_res.active_plan.guid)
        {
            let _ = power::restore_plan(&current_plan.guid, apply_res.duplicated_guid.as_deref());
            return Err(AppError::VerificationMismatch {
                tweak_id: tweak.id.clone(),
                expected: apply_res.active_plan.guid,
                actual: reread_plan.guid,
            });
        }

        backup::write_or_replace_snapshot(
            &backups_dir,
            &tweak.id,
            &tweak.nombre,
            BackupPayload::PowerPlan {
                previous_guid: true_original_guid,
                previous_name: true_original_name,
                applied_guid: reread_plan.guid.clone(),
                duplicated_guid: apply_res.duplicated_guid,
            },
        )?;

        // Limpia snapshots de planes anteriores reemplazados por el nuevo plan activo
        for (old_id, old_dup) in superseded_plan_snapshots {
            if let Some(dup_g) = old_dup {
                if !dup_g.eq_ignore_ascii_case(&reread_plan.guid) {
                    let _ = power::restore_plan(&reread_plan.guid, Some(&dup_g));
                }
            }
            let _ = backup::remove_snapshot(&backups_dir, &old_id);
        }

        let verified_human = format!("{} ({})", reread_plan.friendly_name, reread_plan.guid);
        log::record_change(
            &self.db,
            &tweak.id,
            &tweak.nombre,
            "APPLY",
            &current_human,
            &verified_human,
            true,
            "Plan de energía activado y verificado vía PowrProf con GUID original respaldado.",
        )?;

        let all_tweaks = self.list_tweaks_with_status();
        let updated_view = all_tweaks
            .iter()
            .find(|t| t.definition.id == tweak.id)
            .cloned()
            .unwrap_or_else(|| self.inspect_tweak(tweak));

        Ok(MutationOutcome {
            tweak: updated_view,
            all_tweaks,
            restore_point_created,
            restore_point_warning,
            message: format!("Plan '{}' activado y verificado.", tweak.nombre),
        })
    }

    fn apply_power_setting_tweak(
        &self,
        tweak: &TweakDefinition,
        subgroup_guid: &str,
        setting_guid: &str,
        target_ac: u32,
        target_dc: u32,
    ) -> AppResult<MutationOutcome> {
        let current = power::read_setting_on_active_scheme(subgroup_guid, setting_guid)?;
        let current_human = current.display_human();

        let backups_dir = self.db.backups_dir();
        let backup_outcome = backup::ensure_snapshot(
            &backups_dir,
            &tweak.id,
            &tweak.nombre,
            BackupPayload::PowerSetting {
                scheme_guid: current.scheme_guid.clone(),
                subgroup_guid: subgroup_guid.to_string(),
                setting_guid: setting_guid.to_string(),
                previous_ac_index: current.ac_index,
                previous_dc_index: current.dc_index,
            },
        )?;

        let (restore_point_created, restore_point_warning) =
            self.trigger_session_restore_point(tweak, &current_human);

        let expected_display = format!(
            "AC={target_ac}, DC={target_dc} [Plan {}]",
            current.scheme_guid
        );

        let apply_and_verify = power::write_setting_on_scheme(
            &current.scheme_guid,
            subgroup_guid,
            setting_guid,
            target_ac,
            target_dc,
        )
        .and_then(|()| {
            let reread =
                power::read_setting_on_scheme(&current.scheme_guid, subgroup_guid, setting_guid)?;
            if reread.ac_index != target_ac || reread.dc_index != target_dc {
                Err(AppError::VerificationMismatch {
                    tweak_id: tweak.id.clone(),
                    expected: expected_display.clone(),
                    actual: reread.display_human(),
                })
            } else {
                Ok(reread)
            }
        });

        let verified = match apply_and_verify {
            Ok(v) => v,
            Err(step_err) => {
                let rollback_res = restore_from_snapshot(&backup_outcome.snapshot);
                if backup_outcome.newly_created && rollback_res.is_ok() {
                    let _ = backup::remove_snapshot(&backups_dir, &tweak.id);
                }

                let _ = log::record_change(
                    &self.db,
                    &tweak.id,
                    &tweak.nombre,
                    "ROLLBACK_ERROR",
                    &current_human,
                    &expected_display,
                    false,
                    &format!("Fallo durante apply/verify PowrProf: {step_err}"),
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

        log::record_change(
            &self.db,
            &tweak.id,
            &tweak.nombre,
            "APPLY",
            &current_human,
            &verified.display_human(),
            true,
            "Índices AC/DC aplicados y verificados vía PowrProf.",
        )?;

        let all_tweaks = self.list_tweaks_with_status();
        let updated_view = all_tweaks
            .iter()
            .find(|t| t.definition.id == tweak.id)
            .cloned()
            .unwrap_or_else(|| self.inspect_tweak(tweak));

        Ok(MutationOutcome {
            tweak: updated_view,
            all_tweaks,
            restore_point_created,
            restore_point_warning,
            message: format!("Ajuste '{}' aplicado y verificado.", tweak.nombre),
        })
    }

    pub fn apply_preset(&self, preset_id: &str) -> AppResult<PresetOutcome> {
        let ids = Self::preset_tweak_ids(preset_id)?;
        let mut applied_ids = Vec::new();
        let mut any_rp_created = false;
        let mut first_rp_warning: Option<String> = None;

        for tid in ids {
            let outcome = self.apply_tweak(tid)?;
            applied_ids.push((*tid).to_string());
            if outcome.restore_point_created {
                any_rp_created = true;
            }
            if first_rp_warning.is_none() && outcome.restore_point_warning.is_some() {
                first_rp_warning = outcome.restore_point_warning;
            }
        }

        let all_tweaks = self.list_tweaks_with_status();
        let preset_label = if preset_id == "competitivo" {
            "Competitivo"
        } else {
            "Equilibrado"
        };

        Ok(PresetOutcome {
            preset_id: preset_id.to_string(),
            applied_ids,
            all_tweaks,
            restore_point_created: any_rp_created,
            restore_point_warning: first_rp_warning,
            message: format!(
                "Preset '{}' aplicado y verificado con éxito ({} ajustes seguros).",
                preset_label,
                ids.len()
            ),
        })
    }

    pub fn revert_tweak(&self, tweak_id: &str) -> AppResult<MutationOutcome> {
        let tweak = self.catalog.get_by_id(tweak_id)?;
        guard::check_can_revert(tweak)?;

        let before_view = self.inspect_tweak(tweak);
        let backups_dir = self.db.backups_dir();
        let snapshot = backup::load_snapshot(&backups_dir, &tweak.id)?.ok_or_else(|| {
            AppError::Backup(format!(
                "No existe snapshot de backup activo para '{}'. Por seguridad no se revierte a ciegas.",
                tweak.nombre
            ))
        })?;

        let after_human = restore_and_verify_snapshot(&snapshot)?;
        backup::remove_snapshot(&backups_dir, &tweak.id)?;

        log::record_change(
            &self.db,
            &tweak.id,
            &tweak.nombre,
            "REVERT",
            &before_view.current_os_value,
            &after_human,
            true,
            "Restaurado milimétricamente desde snapshot JSON atómico y verificado.",
        )?;

        let all_tweaks = self.list_tweaks_with_status();
        let updated_view = all_tweaks
            .iter()
            .find(|t| t.definition.id == tweak.id)
            .cloned()
            .unwrap_or_else(|| self.inspect_tweak(tweak));

        Ok(MutationOutcome {
            tweak: updated_view,
            all_tweaks,
            restore_point_created: false,
            restore_point_warning: None,
            message: format!("Tweak '{}' revertido a su estado original.", tweak.nombre),
        })
    }

    /// Itera sobre el historial de SQLite (`change_logs` en orden LIFO) y aplica los backups activos,
    /// asegurando revertir configuraciones `PowerSetting` antes de eliminar esquemas `PowerPlan` duplicados.
    pub fn revert_all(&self) -> AppResult<RevertAllOutcome> {
        let backups_dir = self.db.backups_dir();
        let active_snapshots = backup::list_all_snapshots(&backups_dir)?;
        if active_snapshots.is_empty() {
            return Ok(RevertAllOutcome {
                reverted_count: 0,
                reverted_ids: Vec::new(),
                errors: Vec::new(),
                all_tweaks: self.list_tweaks_with_status(),
                message: "No hay cambios activos pendientes por revertir.".to_string(),
            });
        }

        let active_ids: HashSet<String> = active_snapshots
            .iter()
            .map(|s| s.tweak_id.clone())
            .collect();

        // Orden primario desde el historial transaccional de SQLite (LIFO)
        let mut candidate_order = Vec::new();
        let mut seen = HashSet::new();

        for logged_id in log::list_applied_tweak_ids_lifo(&self.db)? {
            if active_ids.contains(&logged_id) && seen.insert(logged_id.clone()) {
                candidate_order.push(logged_id);
            }
        }

        // Respaldo: cualquier snapshot activo en disco que provenga de sesiones previas
        for snap in &active_snapshots {
            if seen.insert(snap.tweak_id.clone()) {
                candidate_order.push(snap.tweak_id.clone());
            }
        }

        // Garantiza que ajustes internos de un plan (PowerSetting) se reviertan antes de cambiar/borrar el PowerPlan
        let mut non_plan_ids = Vec::new();
        let mut plan_ids = Vec::new();
        for tid in candidate_order {
            let is_plan = active_snapshots
                .iter()
                .find(|s| s.tweak_id == tid)
                .map(|s| matches!(s.payload, BackupPayload::PowerPlan { .. }))
                .unwrap_or(false);
            if is_plan {
                plan_ids.push(tid);
            } else {
                non_plan_ids.push(tid);
            }
        }
        non_plan_ids.extend(plan_ids);

        let mut reverted_ids = Vec::new();
        let mut errors = Vec::new();

        for tid in non_plan_ids {
            match self.revert_tweak(&tid) {
                Ok(_) => reverted_ids.push(tid),
                Err(err) => errors.push(format!("{tid}: {err}")),
            }
        }

        let all_tweaks = self.list_tweaks_with_status();
        let message = if errors.is_empty() {
            format!(
                "Se revirtieron y verificaron correctamente {} ajustes desde sus backups originales.",
                reverted_ids.len()
            )
        } else {
            format!(
                "Se revirtieron {} ajustes, pero ocurrieron {} errores.",
                reverted_ids.len(),
                errors.len()
            )
        };

        Ok(RevertAllOutcome {
            reverted_count: reverted_ids.len(),
            reverted_ids,
            errors,
            all_tweaks,
            message,
        })
    }

    /// Modo Simulación (solo lectura): calcula exactamente qué cambiaría sin escribir en Registro ni PowrProf.
    pub fn simulate_tweaks(&self, tweak_ids: &[String]) -> AppResult<Vec<SimulationItem>> {
        let active_plan = power::get_active_plan().ok();
        let mut results = Vec::with_capacity(tweak_ids.len());

        for tid in tweak_ids {
            let tweak = self.catalog.get_by_id(tid)?;
            let view = self.inspect_tweak_with_plan(tweak, active_plan.as_ref());

            let (reversion_preview, note) = match tweak.nivel {
                TweakLevel::Seguro => {
                    let rev = match &tweak.target {
                        TweakTarget::Registry { .. } => {
                            format!(
                                "Snapshot JSON restauraría valor previo ({})",
                                view.current_os_value
                            )
                        }
                        TweakTarget::PowerPlan { .. } => {
                            format!(
                                "Snapshot JSON guardaría y reactivaría {}",
                                view.current_os_value
                            )
                        }
                        TweakTarget::PowerSetting { .. } => {
                            format!(
                                "Snapshot JSON restauraría índices exactos ({})",
                                view.current_os_value
                            )
                        }
                        _ => "N/A".to_string(),
                    };
                    (rev, None)
                }
                TweakLevel::Avanzado => (
                    "Reversión sujeta a prueba A/B obligatoria".to_string(),
                    Some("Bloqueado en Etapa 2: requiere validación A/B (Etapa 5).".to_string()),
                ),
                TweakLevel::Bloqueado => (
                    "No aplicable".to_string(),
                    Some("Tweak prohibido por seguridad y evidencia descartada.".to_string()),
                ),
            };

            results.push(SimulationItem {
                tweak_id: tweak.id.clone(),
                tweak_name: tweak.nombre.clone(),
                nivel: tweak.nivel,
                target_summary: view.target_summary,
                current_os_value: view.current_os_value,
                proposed_value: view.expected_value_display,
                would_change: tweak.nivel == TweakLevel::Seguro && !view.is_applied,
                requires_admin: tweak.requiere_admin,
                reversion_preview,
                note,
            });
        }

        Ok(results)
    }
}

fn restore_from_snapshot(snapshot: &BackupSnapshot) -> AppResult<()> {
    let _ = restore_and_verify_snapshot(snapshot)?;
    Ok(())
}

fn restore_and_verify_snapshot(snapshot: &BackupSnapshot) -> AppResult<String> {
    match &snapshot.payload {
        BackupPayload::Registry {
            hive,
            subkey,
            value_name,
            previous_state,
        } => {
            match previous_state {
                RegistryState::Present(prev_data) => {
                    registry::write_value(*hive, subkey, value_name, prev_data)?;
                }
                RegistryState::KeyMissing | RegistryState::ValueMissing => {
                    registry::delete_value_if_exists(*hive, subkey, value_name)?;
                }
            }

            let reread = registry::read_value(*hive, subkey, value_name)?;
            let matches = match (previous_state, &reread) {
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
                    expected: previous_state.display_human(),
                    actual: reread.display_human(),
                });
            }

            Ok(reread.display_human())
        }
        BackupPayload::PowerPlan {
            previous_guid,
            duplicated_guid,
            ..
        } => {
            let restored = power::restore_plan(previous_guid, duplicated_guid.as_deref())?;
            if !restored.guid.eq_ignore_ascii_case(previous_guid) {
                return Err(AppError::VerificationMismatch {
                    tweak_id: snapshot.tweak_id.clone(),
                    expected: previous_guid.clone(),
                    actual: restored.guid,
                });
            }
            Ok(format!("{} ({})", restored.friendly_name, restored.guid))
        }
        BackupPayload::PowerSetting {
            scheme_guid,
            subgroup_guid,
            setting_guid,
            previous_ac_index,
            previous_dc_index,
        } => {
            match power::write_setting_on_scheme(
                scheme_guid,
                subgroup_guid,
                setting_guid,
                *previous_ac_index,
                *previous_dc_index,
            ) {
                Ok(()) => {
                    let reread =
                        power::read_setting_on_scheme(scheme_guid, subgroup_guid, setting_guid)?;
                    if reread.ac_index != *previous_ac_index
                        || reread.dc_index != *previous_dc_index
                    {
                        return Err(AppError::VerificationMismatch {
                            tweak_id: snapshot.tweak_id.clone(),
                            expected: format!("AC={previous_ac_index}, DC={previous_dc_index}"),
                            actual: reread.display_human(),
                        });
                    }
                    Ok(reread.display_human())
                }
                Err(AppError::PowerApi { win32_code: 2, .. }) => {
                    Ok("Esquema temporal previo ya eliminado".to_string())
                }
                Err(other) => Err(other),
            }
        }
    }
}
