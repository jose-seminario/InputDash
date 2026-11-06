use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use rusqlite::Connection;

use crate::error::{AppError, AppResult};

pub struct Database {
    conn: Mutex<Connection>,
    data_dir: PathBuf,
}

impl Database {
    pub fn initialize() -> AppResult<Self> {
        let data_dir = resolve_portable_data_dir()?;
        let backups_dir = data_dir.join("backups");
        fs::create_dir_all(&backups_dir).map_err(|e| {
            AppError::Backup(format!(
                "No se pudo crear el directorio de backups '{}': {e}",
                backups_dir.display()
            ))
        })?;

        let db_path = data_dir.join("inputdash.db");
        let conn = Connection::open(&db_path).map_err(|e| {
            AppError::Database(format!(
                "No se pudo abrir SQLite en '{}': {e}",
                db_path.display()
            ))
        })?;

        init_schema(&conn)?;

        Ok(Self {
            conn: Mutex::new(conn),
            data_dir,
        })
    }

    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    pub fn backups_dir(&self) -> PathBuf {
        self.data_dir.join("backups")
    }

    pub fn with_conn<F, T>(&self, f: F) -> AppResult<T>
    where
        F: FnOnce(&Connection) -> AppResult<T>,
    {
        let guard = self
            .conn
            .lock()
            .map_err(|_| AppError::Database("Mutex de SQLite envenenado".to_string()))?;
        f(&guard)
    }

    pub fn is_tweak_ab_validated(&self, tweak_id: &str) -> AppResult<bool> {
        self.with_conn(|conn| {
            let mut stmt = conn
                .prepare("SELECT 1 FROM ab_validations WHERE tweak_id = ?1 AND verdict = 'mejoro' LIMIT 1")
                .map_err(|e| AppError::Database(e.to_string()))?;
            let exists = stmt
                .exists([tweak_id])
                .map_err(|e| AppError::Database(e.to_string()))?;
            Ok(exists)
        })
    }

    pub fn record_ab_validation(&self, tweak_id: &str, metric: &str, verdict: &str) -> AppResult<()> {
        let now = chrono::Utc::now().to_rfc3339();
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO ab_validations (tweak_id, metric, verdict, validated_at_utc)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(tweak_id) DO UPDATE SET
                   metric = excluded.metric,
                   verdict = excluded.verdict,
                   validated_at_utc = excluded.validated_at_utc",
                rusqlite::params![tweak_id, metric, verdict, now],
            )
            .map_err(|e| AppError::Database(format!("Error guardando validación A/B: {e}")))?;
            Ok(())
        })
    }

    pub fn list_validated_tweaks(&self) -> AppResult<Vec<String>> {
        self.with_conn(|conn| {
            let mut stmt = conn
                .prepare("SELECT tweak_id FROM ab_validations WHERE verdict = 'mejoro'")
                .map_err(|e| AppError::Database(e.to_string()))?;
            let rows = stmt
                .query_map([], |row| row.get::<_, String>(0))
                .map_err(|e| AppError::Database(e.to_string()))?;
            let mut result = Vec::new();
            for r in rows {
                if let Ok(id) = r {
                    result.push(id);
                }
            }
            Ok(result)
        })
    }

    pub fn save_game_profile(&self, id: &str, config_json: &str) -> AppResult<()> {
        let now = chrono::Utc::now().to_rfc3339();
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO game_profiles_state (id, config_json, updated_at_utc)
                 VALUES (?1, ?2, ?3)
                 ON CONFLICT(id) DO UPDATE SET
                   config_json = excluded.config_json,
                   updated_at_utc = excluded.updated_at_utc",
                rusqlite::params![id, config_json, now],
            )
            .map_err(|e| AppError::Database(format!("Error guardando perfil de juego: {e}")))?;
            Ok(())
        })
    }

    pub fn load_game_profiles(&self) -> AppResult<Vec<(String, String)>> {
        self.with_conn(|conn| {
            let mut stmt = conn
                .prepare("SELECT id, config_json FROM game_profiles_state")
                .map_err(|e| AppError::Database(e.to_string()))?;
            let rows = stmt
                .query_map([], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                })
                .map_err(|e| AppError::Database(e.to_string()))?;
            let mut result = Vec::new();
            for r in rows {
                if let Ok(pair) = r {
                    result.push(pair);
                }
            }
            Ok(result)
        })
    }
}

fn resolve_portable_data_dir() -> AppResult<PathBuf> {
    // 1. Explicit portable mode: if .portable file or portable.txt exists in exe_dir,
    // and the executable is NOT located in Program Files
    if let Ok(exe_path) = env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let marker = exe_dir.join(".portable");
            let marker_alt = exe_dir.join("portable.txt");
            let dir_lower = exe_dir.to_string_lossy().to_lowercase();
            let is_in_program_files = dir_lower.contains("program files");

            if (marker.exists() || marker_alt.exists()) && !is_in_program_files {
                let portable_dir = exe_dir.join("InputDash_data");
                if fs::create_dir_all(&portable_dir).is_ok() {
                    let probe = portable_dir.join(".write_probe");
                    if fs::write(&probe, b"ok").is_ok() {
                        let _ = fs::remove_file(&probe);
                        return Ok(portable_dir);
                    }
                }
            }
        }
    }

    // 2. Standard Windows installation directory (%LOCALAPPDATA%\InputDash)
    if let Ok(local_app_data) = env::var("LOCALAPPDATA") {
        let fallback = PathBuf::from(local_app_data).join("InputDash");
        if fs::create_dir_all(&fallback).is_ok() {
            return Ok(fallback);
        }
    }

    // 3. Fallback: try exe directory InputDash_data if LOCALAPPDATA failed
    if let Ok(exe_path) = env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let portable_dir = exe_dir.join("InputDash_data");
            if fs::create_dir_all(&portable_dir).is_ok() {
                return Ok(portable_dir);
            }
        }
    }

    Err(AppError::Database(
        "No se pudo resolver una ruta de almacenamiento local escribible".to_string(),
    ))
}

fn init_schema(conn: &Connection) -> AppResult<()> {
    conn.execute_batch(
        "
        PRAGMA journal_mode = WAL;
        PRAGMA foreign_keys = ON;

        CREATE TABLE IF NOT EXISTS change_logs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp_utc TEXT NOT NULL,
            tweak_id TEXT NOT NULL,
            tweak_name TEXT NOT NULL,
            action TEXT NOT NULL,
            previous_value TEXT NOT NULL,
            new_value TEXT NOT NULL,
            verified INTEGER NOT NULL,
            details TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS ab_validations (
            tweak_id TEXT PRIMARY KEY,
            metric TEXT NOT NULL,
            verdict TEXT NOT NULL,
            validated_at_utc TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS game_profiles_state (
            id TEXT PRIMARY KEY,
            config_json TEXT NOT NULL,
            updated_at_utc TEXT NOT NULL
        );
        ",
    )
    .map_err(|e| AppError::Database(format!("Error inicializando esquema SQLite: {e}")))?;

    Ok(())
}
