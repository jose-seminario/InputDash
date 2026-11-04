use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Bloqueado por seguridad (Guard): {0}")]
    GuardViolation(String),

    #[error("{0}")]
    RequiresAbTest(String),

    #[error("Funcionalidad programada para etapa posterior: {0}")]
    StagePending(String),

    #[error("Tweak no encontrado en el catálogo: {0}")]
    TweakNotFound(String),

    #[error("Preset no reconocido: {0}")]
    InvalidPreset(String),

    #[error("Error en API nativa de Registro ({operation}, código Win32 {win32_code:#010X}): {detail}")]
    RegistryApi {
        operation: String,
        win32_code: u32,
        detail: String,
    },

    #[error("Error en API nativa de Energía PowrProf ({operation}, código Win32 {win32_code:#010X}): {detail}")]
    PowerApi {
        operation: String,
        win32_code: u32,
        detail: String,
    },

    #[error("Error en sistema de Backup: {0}")]
    Backup(String),

    #[error("Error al crear Punto de Restauración de Windows (código {code}): {detail}")]
    RestorePoint { code: u32, detail: String },

    #[error("Fallo de verificación post-escritura para '{tweak_id}': se esperaba {expected}, pero el sistema reportó {actual}")]
    VerificationMismatch {
        tweak_id: String,
        expected: String,
        actual: String,
    },

    #[error("Transacción abortada y revertida ({tweak_id}): {cause}")]
    TransactionRolledBack { tweak_id: String, cause: String },

    #[error("CRÍTICO — Falló el rollback automático para '{tweak_id}': causa original ({cause}) | error de rollback ({rollback_error})")]
    RollbackFailure {
        tweak_id: String,
        cause: String,
        rollback_error: String,
    },

    #[error("Error de base de datos local (SQLite): {0}")]
    Database(String),

    #[error("Error de catálogo: {0}")]
    Catalog(String),

    #[error("Plataforma no soportada: {0}")]
    UnsupportedPlatform(String),
}

#[derive(Serialize)]
struct SerializedAppError {
    code: &'static str,
    message: String,
}

impl AppError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::GuardViolation(_) => "GUARD_VIOLATION",
            Self::RequiresAbTest(_) => "REQUIRES_AB_TEST",
            Self::StagePending(_) => "STAGE_PENDING",
            Self::TweakNotFound(_) => "TWEAK_NOT_FOUND",
            Self::InvalidPreset(_) => "INVALID_PRESET",
            Self::RegistryApi { .. } => "REGISTRY_API_ERROR",
            Self::PowerApi { .. } => "POWER_API_ERROR",
            Self::Backup(_) => "BACKUP_ERROR",
            Self::RestorePoint { .. } => "RESTORE_POINT_ERROR",
            Self::VerificationMismatch { .. } => "VERIFICATION_MISMATCH",
            Self::TransactionRolledBack { .. } => "TRANSACTION_ROLLED_BACK",
            Self::RollbackFailure { .. } => "ROLLBACK_FAILURE",
            Self::Database(_) => "DATABASE_ERROR",
            Self::Catalog(_) => "CATALOG_ERROR",
            Self::UnsupportedPlatform(_) => "UNSUPPORTED_PLATFORM",
        }
    }
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let payload = SerializedAppError {
            code: self.code(),
            message: self.to_string(),
        };
        payload.serialize(serializer)
    }
}

pub type AppResult<T> = Result<T, AppError>;
