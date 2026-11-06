use serde::{Deserialize, Serialize};

#[repr(C)]
#[allow(non_snake_case)]
struct PROCESSENTRY32W {
    dwSize: u32,
    cntUsage: u32,
    th32ProcessID: u32,
    th32DefaultHeapID: usize,
    th32ModuleID: u32,
    cntThreads: u32,
    th32ParentProcessID: u32,
    pcPriClassBase: i32,
    dwFlags: u32,
    szExeFile: [u16; 260],
}

#[link(name = "kernel32")]
extern "system" {
    fn CreateToolhelp32Snapshot(dwFlags: u32, th32ProcessID: u32) -> isize;
    fn Process32FirstW(hSnapshot: isize, lppe: *mut PROCESSENTRY32W) -> i32;
    fn Process32NextW(hSnapshot: isize, lppe: *mut PROCESSENTRY32W) -> i32;
    fn CloseHandle(hObject: isize) -> i32;
    fn OpenProcess(dwDesiredAccess: u32, bInheritHandle: i32, dwProcessId: u32) -> isize;
    fn SetPriorityClass(hProcess: isize, dwPriorityClass: u32) -> i32;
    fn SetProcessAffinityMask(hProcess: isize, dwProcessAffinityMask: usize) -> i32;
    fn SetProcessDefaultCpuSets(
        hProcess: isize,
        CpuSetIds: *const u32,
        CpuSetIdCount: u32,
    ) -> i32;
    fn GetSystemCpuSetInformation(
        Information: *mut u8,
        BufferLength: u32,
        ReturnedLength: *mut u32,
        Process: isize,
        Flags: u32,
    ) -> i32;
    fn GetLogicalProcessorInformationEx(
        RelationshipType: i32,
        Buffer: *mut u8,
        ReturnedLength: *mut u32,
    ) -> i32;
}

const TH32CS_SNAPPROCESS: u32 = 0x00000002;
const INVALID_HANDLE_VALUE: isize = -1;

const PROCESS_SET_INFORMATION: u32 = 0x0200;
const HIGH_PRIORITY_CLASS: u32 = 0x00000080;
const NORMAL_PRIORITY_CLASS: u32 = 0x00000020;

// ---------------------------------------------------------------------------
// Modelos de Topología y Procesos
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessItemView {
    pub pid: u32,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuTopologyInfo {
    pub brand: String,
    pub is_hybrid: bool,
    pub is_x3d: bool,
    pub physical_cores: usize,
    pub logical_threads: usize,
    pub p_cores: usize,
    pub e_cores: usize,
    pub architecture_label: String,
    pub recommendation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreContentionInfo {
    pub has_contention: bool,
    pub contenders: Vec<String>,
    pub recommendation: String,
}

// ---------------------------------------------------------------------------
// Detección de CPUID y Topología (P/E-Cores y AMD X3D)
// ---------------------------------------------------------------------------

#[cfg(target_arch = "x86_64")]
fn query_cpu_brand() -> String {
    use std::arch::x86_64::__cpuid;
    let mut brand_bytes = [0u8; 48];
    for (i, func) in [0x80000002u32, 0x80000003, 0x80000004].iter().enumerate() {
        let res = __cpuid(*func);
        let regs = [res.eax, res.ebx, res.ecx, res.edx];
        for (r_idx, reg) in regs.iter().enumerate() {
            let b = reg.to_ne_bytes();
            let start = i * 16 + r_idx * 4;
            if start + 4 <= brand_bytes.len() {
                brand_bytes[start..start + 4].copy_from_slice(&b);
            }
        }
    }
    String::from_utf8_lossy(&brand_bytes)
        .trim_matches('\0')
        .trim()
        .to_string()
}

#[cfg(not(target_arch = "x86_64"))]
fn query_cpu_brand() -> String {
    "Procesador Genérico".to_string()
}

/// Lee la topología de procesadores de Windows usando GetLogicalProcessorInformationEx
pub fn get_cpu_topology() -> CpuTopologyInfo {
    let brand = query_cpu_brand();
    let upper_brand = brand.to_uppercase();

    // Detección de AMD 3D V-Cache (X3D)
    let is_x3d = upper_brand.contains("X3D") || upper_brand.contains("3D V-CACHE");

    let logical_threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(8);

    let mut returned_length: u32 = 0;
    // RelationProcessorCore = 0
    let _ = unsafe { GetLogicalProcessorInformationEx(0, std::ptr::null_mut(), &mut returned_length) };

    let mut efficiency_classes: Vec<u8> = Vec::new();
    let mut physical_cores = 0;

    if returned_length > 0 {
        let mut buffer = vec![0u8; returned_length as usize];
        let ok = unsafe {
            GetLogicalProcessorInformationEx(0, buffer.as_mut_ptr(), &mut returned_length)
        };

        if ok != 0 {
            let mut offset = 0;
            let total_len = returned_length as usize;

            while offset + 8 <= total_len {
                let relationship = u32::from_ne_bytes(buffer[offset..offset + 4].try_into().unwrap_or([0; 4]));
                let record_size = u32::from_ne_bytes(buffer[offset + 4..offset + 8].try_into().unwrap_or([0; 4])) as usize;

                if record_size == 0 {
                    break;
                }

                // RelationProcessorCore == 0
                if relationship == 0 && offset + 10 <= total_len {
                    physical_cores += 1;
                    // flags está en offset + 8, efficiency_class está en offset + 9
                    let eff = buffer[offset + 9];
                    efficiency_classes.push(eff);
                }

                offset += record_size;
            }
        }
    }

    if physical_cores == 0 {
        physical_cores = (logical_threads / 2).max(1);
    }

    // Análisis de clases de eficiencia para arquitecturas híbridas (Intel Alder/Raptor Lake)
    let min_eff = efficiency_classes.iter().min().copied().unwrap_or(0);
    let max_eff = efficiency_classes.iter().max().copied().unwrap_or(0);

    let is_hybrid = (min_eff != max_eff)
        || (upper_brand.contains("INTEL") && (upper_brand.contains("12TH") || upper_brand.contains("13TH") || upper_brand.contains("14TH") || upper_brand.contains("ULTRA")));

    let mut p_cores = 0;
    let mut e_cores = 0;

    if is_hybrid && !efficiency_classes.is_empty() {
        for &eff in &efficiency_classes {
            if eff > min_eff {
                p_cores += 1;
            } else {
                e_cores += 1;
            }
        }
    } else {
        p_cores = physical_cores;
        e_cores = 0;
    }

    let (architecture_label, recommendation) = if is_hybrid {
        (
            format!("Intel Híbrido P/E-cores ({}P + {}E)", p_cores, e_cores),
            "Windows Thread Director administra la afinidad dinámicamente. La afinidad manual estática está BLOQUEADA para evitar cuellos de botella.".to_string(),
        )
    } else if is_x3d {
        (
            "AMD Ryzen 3D V-Cache (X3D)".to_string(),
            "El controlador de AMD 3D V-Cache Optimizer prioriza los hilos del juego en el CCD con caché 3D. Afinidad manual BLOQUEADA.".to_string(),
        )
    } else {
        (
            "Arquitectura Homogénea Estándar".to_string(),
            "Arquitectura estándar sin núcleos dispares. Soporta ajuste de prioridad High y Timer Resolution 0.5ms.".to_string(),
        )
    };

    CpuTopologyInfo {
        brand,
        is_hybrid,
        is_x3d,
        physical_cores,
        logical_threads,
        p_cores,
        e_cores,
        architecture_label,
        recommendation,
    }
}

// ---------------------------------------------------------------------------
// Lista Segura de Procesos en Ejecución (Solo lectura de szExeFile)
// ---------------------------------------------------------------------------

pub fn get_running_processes() -> Vec<ProcessItemView> {
    let mut result = Vec::new();

    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
    if snapshot == INVALID_HANDLE_VALUE {
        return result;
    }

    let mut entry = PROCESSENTRY32W {
        dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
        cntUsage: 0,
        th32ProcessID: 0,
        th32DefaultHeapID: 0,
        th32ModuleID: 0,
        cntThreads: 0,
        th32ParentProcessID: 0,
        pcPriClassBase: 0,
        dwFlags: 0,
        szExeFile: [0; 260],
    };

    let mut ok = unsafe { Process32FirstW(snapshot, &mut entry) };
    while ok != 0 {
        let exe_name = String::from_utf16_lossy(&entry.szExeFile)
            .trim_matches('\0')
            .to_string();

        if !exe_name.is_empty() && entry.th32ProcessID != 0 {
            result.push(ProcessItemView {
                pid: entry.th32ProcessID,
                name: exe_name,
            });
        }

        entry.szExeFile = [0; 260];
        ok = unsafe { Process32NextW(snapshot, &mut entry) };
    }

    unsafe {
        CloseHandle(snapshot);
    }

    result.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    result
}

// ---------------------------------------------------------------------------
// Detección de Core Contention (Discord, Spotify, iCUE, etc.)
// ---------------------------------------------------------------------------

pub fn get_core_contention() -> CoreContentionInfo {
    let processes = get_running_processes();

    let noisy_apps = [
        ("discord.exe", "Discord (Aceleración por hardware / Audio Hook)"),
        ("spotify.exe", "Spotify (Timers parásitos de Chromium)"),
        ("icue.exe", "Corsair iCUE (Alta frecuencia de sondeo USB)"),
        ("armourycrate.exe", "ASUS Armoury Crate"),
        ("lightingservice.exe", "ASUS Aura / Lighting Service (DPC spikes)"),
        ("razersynapse.exe", "Razer Synapse"),
        ("rzsdkserver.exe", "Razer Chroma SDK"),
        ("nahimicservice.exe", "Nahimic Audio Service (Retardo DPC crítico)"),
    ];

    let mut detected = Vec::new();
    for p in &processes {
        let lower = p.name.to_lowercase();
        for (target, label) in &noisy_apps {
            if lower == *target && !detected.contains(&label.to_string()) {
                detected.push(label.to_string());
            }
        }
    }

    let has_contention = !detected.is_empty();
    let recommendation = if has_contention {
        format!(
            "Se detectaron {} procesos en segundo plano con impacto comprobado en micro-stutters e interrupciones DPC. Ciérralos antes de partidas competitivas.",
            detected.len()
        )
    } else {
        "No se detectaron aplicaciones ruidosas en segundo plano compitiendo por ciclos de CPU.".to_string()
    };

    CoreContentionInfo {
        has_contention,
        contenders: detected,
        recommendation,
    }
}

// ---------------------------------------------------------------------------
// Ajuste Seguro de Prioridad y Afinidad (Sin OpenProcess VM_READ ni Inyección)
// ---------------------------------------------------------------------------

pub fn set_process_priority(pid: u32, high: bool) -> Result<(), String> {
    if pid == 0 {
        return Err("PID inválido".to_string());
    }

    // ÚNICAMENTE PROCESS_SET_INFORMATION (0x0200). PROHIBIDO PROCESS_VM_READ o PROCESS_ALL_ACCESS.
    let handle = unsafe { OpenProcess(PROCESS_SET_INFORMATION, 0, pid) };
    if handle == 0 || handle == INVALID_HANDLE_VALUE {
        return Err(format!("No se pudo abrir el proceso {pid} con PROCESS_SET_INFORMATION (permisos insuficientes)"));
    }

    let priority = if high {
        HIGH_PRIORITY_CLASS
    } else {
        NORMAL_PRIORITY_CLASS
    };

    let ok = unsafe { SetPriorityClass(handle, priority) };
    unsafe { CloseHandle(handle) };

    if ok != 0 {
        Ok(())
    } else {
        Err(format!("Error estableciendo prioridad en proceso {pid}"))
    }
}

pub fn set_process_affinity(pid: u32, mask: usize) -> Result<(), String> {
    if pid == 0 || mask == 0 {
        return Err("Parámetros de afinidad inválidos".to_string());
    }

    let handle = unsafe { OpenProcess(PROCESS_SET_INFORMATION, 0, pid) };
    if handle == 0 || handle == INVALID_HANDLE_VALUE {
        return Err(format!("No se pudo abrir proceso {pid} con PROCESS_SET_INFORMATION"));
    }

    let ok = unsafe { SetProcessAffinityMask(handle, mask) };
    unsafe { CloseHandle(handle) };

    if ok != 0 {
        Ok(())
    } else {
        Err(format!("Error estableciendo máscara de afinidad en proceso {pid}"))
    }
}

/// Aplica CPU Sets para aislar núcleos en el proceso, con candado de seguridad para CPUs monolíticas.
/// Excluye estrictamente el Core 0 (hilo lógico 0).
pub fn set_process_cpu_sets_isolate(pid: u32) -> Result<(), String> {
    if pid == 0 {
        return Err("PID inválido".to_string());
    }

    let topo = get_cpu_topology();
    let is_asymmetric = topo.is_hybrid || topo.is_x3d;
    if !is_asymmetric {
        return Err("Tu CPU es monolítica. El aislamiento de núcleos no es necesario y podría causar inestabilidad.".to_string());
    }

    let mut returned_len = 0u32;
    let _ = unsafe { GetSystemCpuSetInformation(std::ptr::null_mut(), 0, &mut returned_len, 0, 0) };

    let mut selected_cpu_set_ids = Vec::new();

    if returned_len > 0 {
        let mut buffer = vec![0u8; returned_len as usize];
        let ok = unsafe {
            GetSystemCpuSetInformation(
                buffer.as_mut_ptr(),
                returned_len,
                &mut returned_len,
                0,
                0,
            )
        };

        if ok != 0 {
            let mut offset = 0;
            let total_len = returned_len as usize;
            let mut entries: Vec<(u32, u8, u8, u8)> = Vec::new();

            while offset + 20 <= total_len {
                let size = u32::from_ne_bytes(buffer[offset..offset + 4].try_into().unwrap_or([0; 4])) as usize;
                let entry_type = u32::from_ne_bytes(buffer[offset + 4..offset + 8].try_into().unwrap_or([0; 4]));

                if size == 0 {
                    break;
                }

                if entry_type == 0 && offset + 20 <= total_len {
                    let id = u32::from_ne_bytes(buffer[offset + 8..offset + 12].try_into().unwrap_or([0; 4]));
                    let logical_idx = buffer[offset + 14];
                    let eff_class = buffer[offset + 18];
                    let llc_idx = buffer[offset + 16];
                    entries.push((id, logical_idx, eff_class, llc_idx));
                }

                offset += size;
            }

            if topo.is_hybrid {
                let max_eff = entries.iter().map(|e| e.2).max().unwrap_or(0);
                for (id, logical_idx, eff_class, _) in &entries {
                    if *logical_idx != 0 && *eff_class == max_eff {
                        selected_cpu_set_ids.push(*id);
                    }
                }
            } else if topo.is_x3d {
                let half_threads = (topo.logical_threads / 2).max(1) as u8;
                for (id, logical_idx, _, _) in &entries {
                    if *logical_idx != 0 && *logical_idx < half_threads {
                        selected_cpu_set_ids.push(*id);
                    }
                }
            }
        }
    }

    if selected_cpu_set_ids.is_empty() {
        let threads_to_use = if topo.is_hybrid {
            (topo.p_cores * (topo.logical_threads / topo.physical_cores)).max(2)
        } else {
            (topo.logical_threads / 2).max(2)
        };

        for idx in 1..threads_to_use {
            selected_cpu_set_ids.push(idx as u32);
        }
    }

    let handle = unsafe { OpenProcess(PROCESS_SET_INFORMATION, 0, pid) };
    if handle == 0 || handle == INVALID_HANDLE_VALUE {
        return Err(format!("No se pudo abrir el proceso {pid} con PROCESS_SET_INFORMATION"));
    }

    let ok = unsafe {
        SetProcessDefaultCpuSets(
            handle,
            selected_cpu_set_ids.as_ptr(),
            selected_cpu_set_ids.len() as u32,
        )
    };
    unsafe { CloseHandle(handle) };

    if ok != 0 {
        Ok(())
    } else {
        Err(format!("Error aplicando CPU Sets en el proceso {pid}"))
    }
}
