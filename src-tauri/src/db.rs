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
}

fn resolve_portable_data_dir() -> AppResult<PathBuf> {
    if let Ok(exe_path) = env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
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

    if let Ok(local_app_data) = env::var("LOCALAPPDATA") {
        let fallback = PathBuf::from(local_app_data).join("InputDash");
        fs::create_dir_all(&fallback).map_err(|e| {
            AppError::Database(format!(
                "No se pudo crear carpeta de datos en LOCALAPPDATA '{}': {e}",
                fallback.display()
            ))
        })?;
        return Ok(fallback);
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
        ",
    )
    .map_err(|e| AppError::Database(format!("Error inicializando esquema SQLite: {e}")))?;

    Ok(())
}
