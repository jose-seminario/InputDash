use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;
use tauri::{AppHandle, Emitter};

use crate::db::Database;
use crate::profiles::games::{GameProfile, ProfileRuntimeState};
use crate::sys::process::{
    get_core_contention, get_cpu_topology, get_running_processes, set_process_affinity,
    set_process_cpu_sets_isolate, set_process_priority, CoreContentionInfo, CpuTopologyInfo,
};
use crate::sys::timer::TimerResolutionGuard;

struct ProfileManagerState {
    profiles: Vec<GameProfile>,
    active_profile_id: Option<String>,
    active_pid: Option<u32>,
    timer_guard: Option<TimerResolutionGuard>,
    cpu_topology: CpuTopologyInfo,
    last_hid_hash: u64,
    app_handle: Option<AppHandle>,
    db: Option<Arc<Database>>,
}

pub struct ProfileManager {
    state: Arc<Mutex<ProfileManagerState>>,
    stop_signal: Arc<AtomicBool>,
    _worker_thread: Option<JoinHandle<()>>,
}

impl Default for ProfileManager {
    fn default() -> Self {
        Self::new(None)
    }
}

impl ProfileManager {
    pub fn new(db: Option<Arc<Database>>) -> Self {
        let topology = get_cpu_topology();
        let mut profiles = GameProfile::default_profiles();

        // Cargar perfiles guardados en SQLite si existen
        if let Some(d) = &db {
            if let Ok(saved) = d.load_game_profiles() {
                for (id, json_str) in saved {
                    if let Ok(saved_prof) = serde_json::from_str::<GameProfile>(&json_str) {
                        if let Some(pos) = profiles.iter().position(|p| p.id == id) {
                            profiles[pos] = saved_prof;
                        } else {
                            profiles.push(saved_prof);
                        }
                    }
                }
            }
        }

        let initial_state = ProfileManagerState {
            profiles,
            active_profile_id: None,
            active_pid: None,
            timer_guard: None,
            cpu_topology: topology,
            last_hid_hash: 0,
            app_handle: None,
            db,
        };

        let state = Arc::new(Mutex::new(initial_state));
        let stop_signal = Arc::new(AtomicBool::new(false));

        let state_clone = Arc::clone(&state);
        let stop_clone = Arc::clone(&stop_signal);

        let worker = thread::spawn(move || {
            while !stop_clone.load(Ordering::Relaxed) {
                // Polling seguro cada 2.5 segundos
                thread::sleep(Duration::from_millis(2500));
                if stop_clone.load(Ordering::Relaxed) {
                    break;
                }

                Self::poll_tick(&state_clone);
            }
        });

        Self {
            state,
            stop_signal,
            _worker_thread: Some(worker),
        }
    }

    pub fn set_app_handle(&self, handle: AppHandle) {
        if let Ok(mut guard) = self.state.lock() {
            guard.app_handle = Some(handle);
        }
    }

    fn poll_tick(state_arc: &Arc<Mutex<ProfileManagerState>>) {
        let running_processes = get_running_processes();

        // 0. Detección de Hot-Plug USB / periféricos HID fuera del mutex lock
        let current_hid_hash = crate::sys::hardware::get_hid_topology_signature();

        let mut guard = match state_arc.lock() {
            Ok(g) => g,
            Err(_) => return,
        };

        let should_emit = if guard.last_hid_hash != 0 && guard.last_hid_hash != current_hid_hash {
            guard.last_hid_hash = current_hid_hash;
            true
        } else {
            if guard.last_hid_hash == 0 {
                guard.last_hid_hash = current_hid_hash;
            }
            false
        };

        if should_emit {
            if let Some(app) = &guard.app_handle {
                let _ = app.emit(
                    "usb-device-changed",
                    "Cambio de periférico detectado. Por favor, vuelve a medir el polling",
                );
            }
        }

        let mut newly_detected: Option<(String, u32, String)> = None;

        // 1. Verificar si el juego actualmente activo sigue corriendo (validando PID y nombre de exe)
        let active_id_opt = guard.active_profile_id.clone();
        if let (Some(active_id), Some(active_pid)) = (active_id_opt, guard.active_pid) {
            let still_running = if let Some(prof) = guard.profiles.iter().find(|p| p.id == active_id) {
                running_processes.iter().any(|p| {
                    p.pid == active_pid && prof.executables.iter().any(|exe| exe.eq_ignore_ascii_case(&p.name))
                })
            } else {
                false
            };

            if !still_running {
                // El juego se cerró: Revertir timer de inmediato con RAII
                guard.timer_guard = None; // Drop de TimerResolutionGuard restaura el SO
                guard.active_profile_id = None;
                guard.active_pid = None;

                if let Some(prof) = guard.profiles.iter_mut().find(|p| p.id == active_id) {
                    prof.runtime_state = ProfileRuntimeState::Waiting;
                    prof.detected_pid = None;
                    prof.detected_exe = None;
                    prof.last_status_message = Some(format!(
                        "Juego cerrado. Timer y afinidad restaurados a valores por defecto."
                    ));
                }
            }
        }

        // 2. Si no hay juego activo, buscar si alguno de los perfiles habilitados se ha iniciado
        if guard.active_profile_id.is_none() {
            for prof in &guard.profiles {
                if !prof.enabled {
                    continue;
                }

                for proc in &running_processes {
                    let proc_name_lower = proc.name.to_lowercase();
                    if prof.executables.iter().any(|exe| exe.to_lowercase() == proc_name_lower) {
                        newly_detected = Some((prof.id.clone(), proc.pid, proc.name.clone()));
                        break;
                    }
                }

                if newly_detected.is_some() {
                    break;
                }
            }
        }

        // 3. Aplicar optimizaciones si se detectó un juego
        if let Some((prof_id, pid, exe_name)) = newly_detected {
            let (high_priority, timer_req, isolate_req, manual_affinity) = {
                let prof = match guard.profiles.iter().find(|p| p.id == prof_id) {
                    Some(p) => p,
                    None => return,
                };
                (
                    prof.high_priority,
                    prof.timer_resolution_05ms,
                    prof.cpu_sets_isolate,
                    prof.manual_affinity_mask,
                )
            };

            let mut status_msg_parts = Vec::new();

            // A. Prioridad de Proceso (HIGH_PRIORITY_CLASS)
            if high_priority {
                match set_process_priority(pid, true) {
                    Ok(()) => status_msg_parts.push("Prioridad Alta activa".to_string()),
                    Err(e) => status_msg_parts.push(format!("Prioridad: {e}")),
                }
            }

            // B. Temporizador Global de 0.5 ms
            let mut timer_res_acquired = None;
            if timer_req {
                match TimerResolutionGuard::acquire_05ms() {
                    Ok(g) => {
                        status_msg_parts.push("Timer 0.5 ms activo".to_string());
                        timer_res_acquired = Some(g);
                    }
                    Err(e) => status_msg_parts.push(format!("Timer: {e}")),
                }
            }

            // C. Afinidad / CPU Sets
            if let Some(mask) = manual_affinity {
                match set_process_affinity(pid, mask) {
                    Ok(()) => status_msg_parts.push(format!("Afinidad fija (Máscara 0x{mask:X})")),
                    Err(e) => status_msg_parts.push(format!("Afinidad: {e}")),
                }
            } else if isolate_req {
                match set_process_cpu_sets_isolate(pid) {
                    Ok(()) => status_msg_parts.push("Núcleos aislados (CPU Sets)".to_string()),
                    Err(e) => status_msg_parts.push(format!("CPU Sets: {e}")),
                }
            }

            if let Some(prof) = guard.profiles.iter_mut().find(|p| p.id == prof_id) {
                prof.runtime_state = ProfileRuntimeState::Active;
                prof.detected_pid = Some(pid);
                prof.detected_exe = Some(exe_name.clone());
                prof.last_status_message = Some(format!(
                    "🎮 Juego detectado ({exe_name} - PID {pid}): {}",
                    status_msg_parts.join(" · ")
                ));
            }

            guard.active_profile_id = Some(prof_id);
            guard.active_pid = Some(pid);
            guard.timer_guard = timer_res_acquired;
        }
    }

    pub fn get_profiles(&self) -> Vec<GameProfile> {
        let guard = match self.state.lock() {
            Ok(g) => g,
            Err(_) => return Vec::new(),
        };
        guard.profiles.clone()
    }

    pub fn toggle_profile(&self, id: &str, enabled: bool) -> Result<GameProfile, String> {
        let mut guard = self.state.lock().map_err(|e| e.to_string())?;

        let is_active = guard.active_profile_id.as_deref() == Some(id);
        if !enabled && is_active {
            if let Some(pid) = guard.active_pid {
                let _ = set_process_priority(pid, false);
            }
            guard.timer_guard = None;
            guard.active_profile_id = None;
            guard.active_pid = None;
        }

        let db_opt = guard.db.clone();

        if let Some(prof) = guard.profiles.iter_mut().find(|p| p.id == id) {
            prof.enabled = enabled;
            if !enabled && is_active {
                prof.runtime_state = ProfileRuntimeState::Waiting;
                prof.detected_pid = None;
                prof.detected_exe = None;
                prof.last_status_message = Some("Perfil desactivado por el usuario. Prioridad y timer restaurados.".to_string());
            }

            let clone = prof.clone();
            if let Some(d) = db_opt {
                if let Ok(json_str) = serde_json::to_string(&clone) {
                    let _ = d.save_game_profile(&clone.id, &json_str);
                }
            }

            Ok(clone)
        } else {
            Err(format!("Perfil con id '{id}' no encontrado"))
        }
    }

    pub fn update_profile(&self, updated: GameProfile) -> Result<GameProfile, String> {
        let mut guard = self.state.lock().map_err(|e| e.to_string())?;

        let is_restricted = guard.cpu_topology.is_hybrid || guard.cpu_topology.is_x3d;
        let mut prof_to_save = updated;

        if !is_restricted && prof_to_save.cpu_sets_isolate {
            prof_to_save.cpu_sets_isolate = false;
            return Err("Tu CPU es monolítica. El aislamiento de núcleos no es necesario y podría causar inestabilidad.".to_string());
        }
        if is_restricted && prof_to_save.manual_affinity_mask.is_some() {
            prof_to_save.manual_affinity_mask = None;
            prof_to_save.last_status_message = Some(
                "Afinidad manual deshabilitada: la arquitectura de tu CPU requiere gestión dinámica.".to_string(),
            );
        }

        let db_opt = guard.db.clone();
        if let Some(d) = &db_opt {
            if let Ok(json_str) = serde_json::to_string(&prof_to_save) {
                let _ = d.save_game_profile(&prof_to_save.id, &json_str);
            }
        }

        if let Some(index) = guard.profiles.iter().position(|p| p.id == prof_to_save.id) {
            guard.profiles[index] = prof_to_save.clone();
            Ok(prof_to_save)
        } else {
            guard.profiles.push(prof_to_save.clone());
            Ok(prof_to_save)
        }
    }

    pub fn get_cpu_topology(&self) -> CpuTopologyInfo {
        let guard = match self.state.lock() {
            Ok(g) => g,
            Err(_) => return get_cpu_topology(),
        };
        guard.cpu_topology.clone()
    }

    pub fn get_core_contention(&self) -> CoreContentionInfo {
        get_core_contention()
    }
}

impl Drop for ProfileManager {
    fn drop(&mut self) {
        self.stop_signal.store(true, Ordering::SeqCst);
        if let Ok(mut guard) = self.state.lock() {
            guard.timer_guard = None;
            guard.active_profile_id = None;
            guard.active_pid = None;
        }
    }
}
