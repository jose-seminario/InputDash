use chrono::Utc;
use rusqlite::params;
use serde::{Deserialize, Serialize};

use crate::db::Database;
use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeLogEntry {
    pub id: i64,
    pub timestamp_utc: String,
    pub tweak_id: String,
    pub tweak_name: String,
    pub action: String,
    pub previous_value: String,
    pub new_value: String,
    pub verified: bool,
    pub details: String,
}

pub fn record_change(
    db: &Database,
    tweak_id: &str,
    tweak_name: &str,
    action: &str,
    previous_value: &str,
    new_value: &str,
    verified: bool,
    details: &str,
) -> AppResult<()> {
    let timestamp = Utc::now().to_rfc3339();
    db.with_conn(|conn| {
        conn.execute(
            "
            INSERT INTO change_logs (
                timestamp_utc, tweak_id, tweak_name, action,
                previous_value, new_value, verified, details
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            ",
            params![
                timestamp,
                tweak_id,
                tweak_name,
                action,
                previous_value,
                new_value,
                if verified { 1i64 } else { 0i64 },
                details
            ],
        )
        .map_err(|e| AppError::Database(format!("No se pudo insertar registro en log: {e}")))?;
        Ok(())
    })
}

pub fn list_recent_changes(db: &Database, limit: u32) -> AppResult<Vec<ChangeLogEntry>> {
    db.with_conn(|conn| {
        let mut stmt = conn
            .prepare(
                "
                SELECT id, timestamp_utc, tweak_id, tweak_name, action,
                       previous_value, new_value, verified, details
                FROM change_logs
                ORDER BY id DESC
                LIMIT ?1
                ",
            )
            .map_err(|e| AppError::Database(format!("Error preparando consulta de logs: {e}")))?;

        let rows = stmt
            .query_map(params![limit], |row| {
                let verified_int: i64 = row.get(7)?;
                Ok(ChangeLogEntry {
                    id: row.get(0)?,
                    timestamp_utc: row.get(1)?,
                    tweak_id: row.get(2)?,
                    tweak_name: row.get(3)?,
                    action: row.get(4)?,
                    previous_value: row.get(5)?,
                    new_value: row.get(6)?,
                    verified: verified_int != 0,
                    details: row.get(7 + 1)?,
                })
            })
            .map_err(|e| AppError::Database(format!("Error leyendo historial de cambios: {e}")))?;

        let mut entries = Vec::new();
        for item in rows {
            entries.push(
                item.map_err(|e| AppError::Database(format!("Fila inválida en change_logs: {e}")))?,
            );
        }
        Ok(entries)
    })
}
