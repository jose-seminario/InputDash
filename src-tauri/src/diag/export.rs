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

    // 1. Export JSON
    let json_content = serde_json::to_string_pretty(payload)
        .map_err(|e| AppError::Database(format!("Error serializando reporte JSON: {e}")))?;

    let mut jf = File::create(&json_file_path)
        .map_err(|e| AppError::Database(format!("Error creando archivo {}: {e}", json_file_path.display())))?;
    jf.write_all(json_content.as_bytes())
        .map_err(|e| AppError::Database(format!("Error escribiendo archivo JSON: {e}")))?;

    // 2. Export HTML con estilos CSS embebidos
    let html_content = generate_html_report(payload, &timestamp_str);
    let mut hf = File::create(&html_file_path)
        .map_err(|e| AppError::Database(format!("Error creando archivo {}: {e}", html_file_path.display())))?;
    hf.write_all(html_content.as_bytes())
        .map_err(|e| AppError::Database(format!("Error escribiendo archivo HTML: {e}")))?;

    let json_str = json_file_path.to_string_lossy().to_string();
    let html_str = html_file_path.to_string_lossy().to_string();

    Ok(ExportResult {
        json_path: json_str.clone(),
        html_path: html_str.clone(),
        filename_base,
        message: format!("Reporte guardado exitosamente en: {html_str}"),
    })
}

fn generate_html_report(payload: &FullExportPayload, timestamp: &str) -> String {
    let hardware_json = payload.hardware.as_ref().map(|v| serde_json::to_string_pretty(v).unwrap_or_default()).unwrap_or_else(|| "No disponible".to_string());
    let tel_json = payload.tel.as_ref().map(|v| serde_json::to_string_pretty(v).unwrap_or_default()).unwrap_or_else(|| "No disponible".to_string());
    let network_json = payload.network.as_ref().map(|v| serde_json::to_string_pretty(v).unwrap_or_default()).unwrap_or_else(|| "No disponible".to_string());
    let bufferbloat_json = payload.bufferbloat.as_ref().map(|v| serde_json::to_string_pretty(v).unwrap_or_default()).unwrap_or_else(|| "No disponible".to_string());
    let polling_json = payload.polling.as_ref().map(|v| serde_json::to_string_pretty(v).unwrap_or_default()).unwrap_or_else(|| "No disponible".to_string());
    let ab_json = payload.ab_session.as_ref().map(|v| serde_json::to_string_pretty(v).unwrap_or_default()).unwrap_or_else(|| "No disponible".to_string());
    let blind_json = payload.blind_result.as_ref().map(|v| serde_json::to_string_pretty(v).unwrap_or_default()).unwrap_or_else(|| "No disponible".to_string());

    format!(
        r#"<!DOCTYPE html>
<html lang="es">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Reporte de Diagnóstico InputDash - {timestamp}</title>
    <style>
        :root {{
            --bg: #0b0e14;
            --panel: #131822;
            --panel-alt: #1a2230;
            --border: #263044;
            --text: #e6edf3;
            --text-muted: #9aa7b8;
            --accent: #38bdf8;
            --accent-green: #3fb950;
            --accent-orange: #f59e0b;
            --accent-red: #ef4444;
        }}
        body {{
            background: var(--bg);
            color: var(--text);
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
            line-height: 1.5;
            margin: 0;
            padding: 30px 20px;
        }}
        .container {{
            max-width: 1000px;
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
            margin: 0 0 6px;
            font-size: 28px;
            color: #ffffff;
            letter-spacing: -0.5px;
        }}
        .subtitle {{
            color: var(--text-muted);
            font-size: 14px;
            margin: 0;
        }}
        .badge {{
            display: inline-block;
            background: rgba(56, 189, 248, 0.15);
            color: var(--accent);
            border: 1px solid rgba(56, 189, 248, 0.3);
            padding: 4px 12px;
            border-radius: 6px;
            font-size: 13px;
            font-weight: 600;
        }}
        .section {{
            background: var(--panel);
            border: 1px solid var(--border);
            border-radius: 8px;
            padding: 22px;
            margin-bottom: 24px;
        }}
        .section-header {{
            display: flex;
            align-items: center;
            justify-content: space-between;
            border-bottom: 1px solid var(--border);
            padding-bottom: 12px;
            margin-bottom: 16px;
        }}
        .section-header h2 {{
            margin: 0;
            font-size: 18px;
            color: #ffffff;
        }}
        pre {{
            background: var(--panel-alt);
            border: 1px solid var(--border);
            border-radius: 6px;
            padding: 14px;
            overflow-x: auto;
            font-family: Consolas, "Courier New", monospace;
            font-size: 12.5px;
            color: #c9d1d9;
            max-height: 380px;
        }}
        footer {{
            text-align: center;
            color: var(--text-muted);
            font-size: 12.5px;
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
                <h1>InputDash · Reporte Científico de Latencia</h1>
                <p class="subtitle">Medición y optimización 100% nativa de input lag en Windows 10/11</p>
            </div>
            <div>
                <span class="badge">Fecha: {timestamp}</span>
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

Test Ciego (Double-Blind 30 clicks):
{blind_json}</code></pre>
        </div>

        <footer>
            Generado automáticamente por InputDash · Sin inyección de procesos · 100% APIs Nativas de Windows
        </footer>
    </div>
</body>
</html>"#
    )
}
