use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use chrono::Local;
use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FullExportPayload {
    pub hardware: Option<serde_json::Value>,
    pub network: Option<serde_json::Value>,
    pub bufferbloat: Option<serde_json::Value>,
    pub polling: Option<serde_json::Value>,
    pub tel: Option<serde_json::Value>,
    pub ab_session: Option<serde_json::Value>,
    pub blind_result: Option<serde_json::Value>,
    pub frametime: Option<serde_json::Value>,
    pub dpc: Option<serde_json::Value>,
    pub active_backups: Option<serde_json::Value>,
    pub change_logs: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportResult {
    pub json_path: String,
    pub html_path: String,
    pub filename_base: String,
    pub message: String,
}

fn get_reports_directory() -> PathBuf {
    if let Ok(user_profile) = std::env::var("USERPROFILE") {
        let docs = PathBuf::from(user_profile).join("Documents").join("InputDash_Reports");
        if fs::create_dir_all(&docs).is_ok() {
            return docs;
        }
    }
    let local = PathBuf::from("InputDash_Reports");
    let _ = fs::create_dir_all(&local);
    local
}

pub fn export_report(payload: &FullExportPayload) -> AppResult<ExportResult> {
    let now = Local::now();
    let timestamp_str = now.format("%Y-%m-%d_%H-%M-%S").to_string();
    let filename_base = format!("InputDash_Report_{timestamp_str}");

    let reports_dir = get_reports_directory();

    let json_file_path = reports_dir.join(format!("{filename_base}.json"));
    let html_file_path = reports_dir.join(format!("{filename_base}.html"));

    // 1. Exportar JSON Completo estructurado
    let json_content = serde_json::to_string_pretty(payload)
        .map_err(|e| AppError::Internal(format!("Error serializando JSON de diagnóstico: {e}")))?;

    let mut json_file = File::create(&json_file_path)
        .map_err(|e| AppError::Internal(format!("Error creando archivo JSON en '{}': {e}", json_file_path.display())))?;
    json_file.write_all(json_content.as_bytes())
        .map_err(|e| AppError::Internal(format!("Error escribiendo archivo JSON: {e}")))?;

    // 2. Exportar HTML Autónomo Dark Theme
    let html_content = generate_html_report(payload, &timestamp_str);
    let mut html_file = File::create(&html_file_path)
        .map_err(|e| AppError::Internal(format!("Error creando archivo HTML en '{}': {e}", html_file_path.display())))?;
    html_file.write_all(html_content.as_bytes())
        .map_err(|e| AppError::Internal(format!("Error escribiendo archivo HTML: {e}")))?;

    let json_str = json_file_path.to_string_lossy().to_string();
    let html_str = html_file_path.to_string_lossy().to_string();

    Ok(ExportResult {
        json_path: json_str,
        html_path: html_str.clone(),
        filename_base,
        message: format!("Reporte guardado exitosamente en: {html_str}"),
    })
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn format_json_val(val_opt: &Option<serde_json::Value>) -> String {
    match val_opt {
        Some(v) => html_escape(&serde_json::to_string_pretty(v).unwrap_or_default()),
        None => "No disponible".to_string(),
    }
}

fn generate_html_report(payload: &FullExportPayload, timestamp: &str) -> String {
    let hardware_json = format_json_val(&payload.hardware);
    let tel_json = format_json_val(&payload.tel);
    let network_json = format_json_val(&payload.network);
    let bufferbloat_json = format_json_val(&payload.bufferbloat);
    let polling_json = format_json_val(&payload.polling);
    let ab_json = format_json_val(&payload.ab_session);
    let blind_json = format_json_val(&payload.blind_result);
    let frametime_json = format_json_val(&payload.frametime);
    let dpc_json = format_json_val(&payload.dpc);
    let backups_json = format_json_val(&payload.active_backups);
    let logs_json = format_json_val(&payload.change_logs);

    format!(
        r#"<!DOCTYPE html>
<html lang="es">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Reporte de Diagnóstico InputDash - {timestamp}</title>
    <style>
        :root {{
            --bg: #070205;
            --panel: #11050a;
            --panel-alt: #1a0810;
            --border: rgba(255, 26, 64, 0.35);
            --text: #ffffff;
            --text-muted: #a3a3b5;
            --accent: #ff1a40;
            --accent-green: #00ff88;
            --accent-orange: #ffaa00;
        }}
        body {{
            background: var(--bg);
            color: var(--text);
            font-family: Consolas, "Courier New", monospace, -apple-system, sans-serif;
            line-height: 1.5;
            margin: 0;
            padding: 30px 20px;
        }}
        .container {{
            max-width: 1050px;
            margin: 0 auto;
        }}
        header {{
            border-bottom: 2px solid var(--border);
            padding-bottom: 20px;
            margin-bottom: 30px;
            display: flex;
            justify-content: space-between;
            align-items: flex-end;
            flex-wrap: wrap;
            gap: 16px;
        }}
        h1 {{
            font-size: 24px;
            margin: 0 0 6px 0;
            color: #ffffff;
            letter-spacing: 1.5px;
            text-shadow: 0 0 12px rgba(255, 26, 64, 0.6);
        }}
        .subtitle {{
            color: var(--text-muted);
            margin: 0;
            font-size: 13px;
        }}
        .badge {{
            background: rgba(255, 26, 64, 0.15);
            color: var(--accent);
            border: 1px solid var(--border);
            padding: 4px 10px;
            border-radius: 4px;
            font-weight: 700;
            font-size: 12px;
        }}
        .section {{
            background: var(--panel);
            border: 1px solid var(--border);
            border-radius: 6px;
            padding: 18px 22px;
            margin-bottom: 24px;
        }}
        .section-header h2 {{
            margin: 0 0 12px 0;
            font-size: 16px;
            font-weight: 800;
            color: var(--accent);
            letter-spacing: 1px;
            text-transform: uppercase;
        }}
        pre {{
            background: var(--panel-alt);
            border: 1px solid rgba(255, 26, 64, 0.18);
            border-radius: 4px;
            padding: 14px;
            overflow-x: auto;
            margin: 0;
        }}
        code {{
            font-family: Consolas, monospace;
            font-size: 12px;
            color: #f1f5f9;
        }}
        footer {{
            text-align: center;
            font-size: 12px;
            color: var(--text-muted);
            margin-top: 40px;
            padding-top: 20px;
            border-top: 1px solid var(--border);
        }}
    </style>
</head>
<body>
    <div class="container">
        <header>
            <div>
                <h1>INPUTDASH // REPORTE CIENTÍFICO DE LATENCIA</h1>
                <p class="subtitle">Medición y optimización 100% nativa de input lag en Windows 10/11</p>
            </div>
            <div>
                <span class="badge">TIMESTAMP: {timestamp}</span>
            </div>
        </header>

        <div class="section">
            <div class="section-header">
                <h2>1. Total Effective Lag (TEL) y Cadena Click-to-Photon</h2>
            </div>
            <pre><code>{tel_json}</code></pre>
        </div>

        <div class="section">
            <div class="section-header">
                <h2>2. Hardware Gráfico y Pantalla</h2>
            </div>
            <pre><code>{hardware_json}</code></pre>
        </div>

        <div class="section">
            <div class="section-header">
                <h2>3. Periféricos y Polling Rate USB (WM_INPUT + QPC)</h2>
            </div>
            <pre><code>{polling_json}</code></pre>
        </div>

        <div class="section">
            <div class="section-header">
                <h2>4. Red Local, ICMP y UDP Jitter</h2>
            </div>
            <pre><code>{network_json}</code></pre>
        </div>

        <div class="section">
            <div class="section-header">
                <h2>5. Diagnóstico de Bufferbloat (Reposo vs Saturación)</h2>
            </div>
            <pre><code>{bufferbloat_json}</code></pre>
        </div>

        <div class="section">
            <div class="section-header">
                <h2>6. Pruebas Inferenciales A/B y Test Ciego (Welch t-test)</h2>
            </div>
            <pre><code>A/B Estándar:
{ab_json}

Test Ciego (Double-Blind):
{blind_json}</code></pre>
        </div>

        <div class="section">
            <div class="section-header">
                <h2>7. Telemetría de Fotogramas 3D (ETW Present Frames)</h2>
            </div>
            <pre><code>{frametime_json}</code></pre>
        </div>

        <div class="section">
            <div class="section-header">
                <h2>8. Latencia DPC/ISR del Kernel y Controladores (.sys)</h2>
            </div>
            <pre><code>{dpc_json}</code></pre>
        </div>

        <div class="section">
            <div class="section-header">
                <h2>9. Snapshots Atómicos de Seguridad y Registro Transaccional</h2>
            </div>
            <pre><code>Backups Activos:
{backups_json}

Historial de Cambios (SQLite):
{logs_json}</code></pre>
        </div>

        <footer>
            Generado automáticamente por InputDash · Sin inyección de procesos · 100% APIs Nativas Win32/Kernel
        </footer>
    </div>
</body>
</html>"#
    )
}
