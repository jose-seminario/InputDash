use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProfileRuntimeState {
    Waiting,
    Active,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameProfile {
    pub id: String,
    pub name: String,
    pub executables: Vec<String>,
    pub enabled: bool,
    pub high_priority: bool,
    pub timer_resolution_05ms: bool,
    pub manual_affinity_mask: Option<usize>,
    pub runtime_state: ProfileRuntimeState,
    pub detected_pid: Option<u32>,
    pub detected_exe: Option<String>,
    pub last_status_message: Option<String>,
}

impl GameProfile {
    pub fn default_profiles() -> Vec<Self> {
        vec![
            Self {
                id: "cs2".to_string(),
                name: "Counter-Strike 2".to_string(),
                executables: vec!["cs2.exe".to_string()],
                enabled: true,
                high_priority: true,
                timer_resolution_05ms: true,
                manual_affinity_mask: None,
                runtime_state: ProfileRuntimeState::Waiting,
                detected_pid: None,
                detected_exe: None,
                last_status_message: Some("Esperando ejecución de cs2.exe...".to_string()),
            },
            Self {
                id: "valorant".to_string(),
                name: "VALORANT".to_string(),
                executables: vec![
                    "VALORANT.exe".to_string(),
                    "VALORANT-Win64-Shipping.exe".to_string(),
                ],
                enabled: true,
                high_priority: true,
                timer_resolution_05ms: true,
                manual_affinity_mask: None,
                runtime_state: ProfileRuntimeState::Waiting,
                detected_pid: None,
                detected_exe: None,
                last_status_message: Some("Esperando ejecución de VALORANT.exe...".to_string()),
            },
            Self {
                id: "warzone".to_string(),
                name: "Call of Duty: Warzone".to_string(),
                executables: vec![
                    "cod.exe".to_string(),
                    "modernwarfare.exe".to_string(),
                    "bootstrapper.exe".to_string(),
                ],
                enabled: true,
                high_priority: true,
                timer_resolution_05ms: true,
                manual_affinity_mask: None,
                runtime_state: ProfileRuntimeState::Waiting,
                detected_pid: None,
                detected_exe: None,
                last_status_message: Some("Esperando ejecución de cod.exe...".to_string()),
            },
            Self {
                id: "custom".to_string(),
                name: "Juego Personalizado".to_string(),
                executables: vec!["game.exe".to_string()],
                enabled: false,
                high_priority: true,
                timer_resolution_05ms: true,
                manual_affinity_mask: None,
                runtime_state: ProfileRuntimeState::Waiting,
                detected_pid: None,
                detected_exe: None,
                last_status_message: Some("Configura el ejecutable de tu juego".to_string()),
            },
        ]
    }
}
