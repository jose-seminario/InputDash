use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crate::profiles::games::{GameProfile, ProfileRuntimeState};
use crate::sys::process::{
    get_core_contention, get_cpu_topology, get_running_processes, set_process_affinity,
    set_process_priority, CoreContentionInfo, CpuTopologyInfo,
};
use crate::sys::timer::TimerResolutionGuard;

struct ProfileManagerState {
    profiles: Vec<GameProfile>,
    active_profile_id: Option<String>,
    active_pid: Option<u32>,
    timer_guard: Option<TimerResolutionGuard>,
    cpu_topology: CpuTopologyInfo,
}

pub struct ProfileManager {
    state: Arc<Mutex<ProfileManagerState>>,
    stop_signal: Arc<AtomicBool>,
    _worker_thread: Option<JoinHandle<()>>,
}

impl ProfileManager {
    pub fn new() -> Self {
        let topology = get_cpu_topology();
        let initial_state = ProfileManagerState {
            profiles: GameProfile::default_profiles(),
            active_profile_id: None,
            active_pid: None,
            timer_guard: None,
            cpu_topology: topology,
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

    fn poll_tick(state_arc: &Arc<Mutex<ProfileManagerState>>) {
        let running_processes = get_running_processes();

        let mut guard = match state_arc.lock() {
            Ok(g) => g,
            Err(_) => return,
        };

        let mut newly_detected: Option<(String, u32, String)> = None;

        // 1. Verificar si el juego actualmente activo sigue corriendo (clonar ID para no tomar prestado guard)
        let active_id_opt = guard.active_profile_id.clone();
        if let (Some(active_id), Some(active_pid)) = (active_id_opt, guard.active_pid) {
            let still_running = running_processes.iter().any(|p| p.pid == active_pid);
            if !still_running {
                // El juego se cerró: Revertir timer de inmediato con RAII
                guard.timer_guard = None; // Drop de TimerResolutionGuard restaura el SO
                guard.active_profile_id = None;
                guard.active_pid = None;

                if let Some(prof) = guard.profiles.iter_mut().find(|p| p.id == active_id) {
                    prof.runtime_state = ProfileRuntimeState::Waiting;
                    prof.detected_pid = None;
                    prof.detected_exe = None;
                    prof.last_status_message = Some("Juego finalizado. Timer y prioridad restaurados.".to_string());
                }
            }
        }

        // 2. Si no hay juego activo, buscar si alguno de los perfiles habilitados se abrió
        if guard.active_profile_id.is_none() {
            for prof in &guard.profiles {
                if !prof.enabled {
                    continue;
                }

                for target_exe in &prof.executables {
                    let target_lower = target_exe.to_lowercase();
                    if let Some(found) = running_processes
                        .iter()
                        .find(|p| p.name.to_lowercase() == target_lower)
                    {
                        newly_detected = Some((prof.id.clone(), found.pid, found.name.clone()));
                        break;
                    }
                }
                if newly_detected.is_some() {
                    break;
                }
            }
        }

        // 3. Activar perfil si se detectó un juego
        if let Some((prof_id, pid, exe_name)) = newly_detected {
            let is_hybrid = guard.cpu_topology.is_hybrid;
            let is_x3d = guard.cpu_topology.is_x3d;

            let mut timer_res_acquired = None;
            let mut status_msg_parts = Vec::new();

            // Buscar configuración del perfil
            if let Some(prof) = guard.profiles.iter_mut().find(|p| p.id == prof_id) {
                // Prioridad High
                if prof.high_priority {
                    match set_process_priority(pid, true) {
                        Ok(()) => status_msg_parts.push("Prioridad Alta activada".to_string()),
                        Err(e) => status_msg_parts.push(format!("Prioridad: {e}")),
                    }
                }

                // Timer Resolution 0.5ms (RAII Guard)
                if prof.timer_resolution_05ms {
                    match TimerResolutionGuard::acquire_05ms() {
                        Ok(t_guard) => {
                            timer_res_acquired = Some(t_guard);
                            status_msg_parts.push("Timer Resolution 0.5ms activo".to_string());
                        }
                        Err(e) => status_msg_parts.push(format!("Timer: {e}")),
                    }
                }

                // Afinidad de CPU con Bloqueo Dinámico
                if let Some(mask) = prof.manual_affinity_mask {
                    if is_hybrid || is_x3d {
                        status_msg_parts.push(
                            "Afinidad manual rechazada (Windows Thread Director / X3D optimiza mejor)".to_string(),
                        );
                    } else {
                        match set_process_affinity(pid, mask) {
                            Ok(()) => status_msg_parts.push(format!("Afinidad fijada a máscara {mask:#X}")),
                            Err(e) => status_msg_parts.push(format!("Afinidad: {e}")),
                        }
                    }
                }

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

        // Evaluar estado activo antes de prestar guard.profiles de forma mutable
        let is_active = guard.active_profile_id.as_deref() == Some(id);
        if !enabled && is_active {
            guard.timer_guard = None;
            guard.active_profile_id = None;
            guard.active_pid = None;
        }

        if let Some(prof) = guard.profiles.iter_mut().find(|p| p.id == id) {
            prof.enabled = enabled;
            if !enabled && is_active {
                prof.runtime_state = ProfileRuntimeState::Waiting;
                prof.detected_pid = None;
                prof.detected_exe = None;
                prof.last_status_message = Some("Perfil desactivado por el usuario.".to_string());
            }
            Ok(prof.clone())
        } else {
            Err(format!("Perfil con id '{id}' no encontrado"))
        }
    }

    pub fn update_profile(&self, updated: GameProfile) -> Result<GameProfile, String> {
        let mut guard = self.state.lock().map_err(|e| e.to_string())?;

        // Bloqueo dinámico de afinidad si la CPU es híbrida o X3D
        let is_restricted = guard.cpu_topology.is_hybrid || guard.cpu_topology.is_x3d;
        let mut prof_to_save = updated;
        if is_restricted && prof_to_save.manual_affinity_mask.is_some() {
            prof_to_save.manual_affinity_mask = None;
            prof_to_save.last_status_message = Some(
                "Afinidad manual deshabilitada: la arquitectura de tu CPU requiere gestión dinámica.".to_string(),
            );
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
        self.stop_signal.store(true, Ordering::Relaxed);
    }
}
