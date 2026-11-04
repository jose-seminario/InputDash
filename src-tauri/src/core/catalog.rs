use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};
use crate::sys::registry::{RegistryData, RegistryHive, RegistryValueKind};

const EMBEDDED_CATALOG_JSON: &str = include_str!("../../resources/catalog.json");

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TweakCategory {
    Cpu,
    Red,
    Energia,
    Gpu,
    Periferico,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TweakLevel {
    Seguro,
    Avanzado,
    Bloqueado,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EvidenceLevel {
    Verificada,
    Posible,
    Descartada,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RiskLevel {
    Bajo,
    Medio,
    Alto,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GameProtocol {
    Udp,
    Tcp,
    Ambos,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "tipo", rename_all = "lowercase")]
pub enum TweakTarget {
    Registry {
        hive: RegistryHive,
        ruta: String,
        valor: String,
        dato: serde_json::Value,
        tipo_dato: RegistryValueKind,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TweakDefinition {
    pub id: String,
    pub nombre: String,
    pub categoria: TweakCategory,
    pub nivel: TweakLevel,
    pub evidencia: EvidenceLevel,
    pub target: TweakTarget,
    pub defecto: serde_json::Value,
    pub requiere_admin: bool,
    pub requiere_reinicio: bool,
    pub riesgo: RiskLevel,
    pub efecto_esperado: String,
    pub aplica_a: Vec<String>,
    pub juego_protocolo: GameProtocol,
}

impl TweakDefinition {
    pub fn target_registry_data(&self) -> AppResult<(RegistryHive, &str, &str, RegistryData)> {
        match &self.target {
            TweakTarget::Registry {
                hive,
                ruta,
                valor,
                dato,
                tipo_dato,
            } => {
                let parsed = json_to_registry_data(&self.id, *tipo_dato, dato)?;
                Ok((*hive, ruta.as_str(), valor.as_str(), parsed))
            }
        }
    }
}

fn json_to_registry_data(
    tweak_id: &str,
    kind: RegistryValueKind,
    raw: &serde_json::Value,
) -> AppResult<RegistryData> {
    match kind {
        RegistryValueKind::DWORD => {
            let num = raw
                .as_u64()
                .and_then(|n| u32::try_from(n).ok())
                .ok_or_else(|| {
                    AppError::Catalog(format!(
                        "El tweak '{tweak_id}' requiere un entero DWORD válido de 32 bits"
                    ))
                })?;
            Ok(RegistryData::Dword(num))
        }
        RegistryValueKind::QWORD => {
            let num = raw.as_u64().ok_or_else(|| {
                AppError::Catalog(format!(
                    "El tweak '{tweak_id}' requiere un entero QWORD válido de 64 bits"
                ))
            })?;
            Ok(RegistryData::Qword(num))
        }
        RegistryValueKind::SZ => {
            let text = raw.as_str().ok_or_else(|| {
                AppError::Catalog(format!(
                    "El tweak '{tweak_id}' requiere una cadena de texto SZ válida"
                ))
            })?;
            Ok(RegistryData::Sz(text.to_string()))
        }
    }
}

pub struct Catalog {
    tweaks: Vec<TweakDefinition>,
}

impl Catalog {
    pub fn load_embedded() -> AppResult<Self> {
        let tweaks: Vec<TweakDefinition> = serde_json::from_str(EMBEDDED_CATALOG_JSON)
            .map_err(|e| AppError::Catalog(format!("JSON de catálogo inválido: {e}")))?;
        Ok(Self { tweaks })
    }

    pub fn all(&self) -> &[TweakDefinition] {
        &self.tweaks
    }

    pub fn get_by_id(&self, id: &str) -> AppResult<&TweakDefinition> {
        self.tweaks
            .iter()
            .find(|t| t.id == id)
            .ok_or_else(|| AppError::TweakNotFound(id.to_string()))
    }
}
