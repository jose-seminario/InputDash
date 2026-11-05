import { useEffect } from "react";
import { useDiagStore } from "../store/useDiagStore";

export function DashboardView() {
  const {
    hardware,
    pollingResult,
    isHardwareLoading,
    isPollingAnalyzing,
    pollingSecondsLeft,
    pollingError,
    hardwareError,
    fetchHardwareInfo,
    startPollingAnalysis,
    clearPollingResult,
    dismissDiagError,
  } = useDiagStore();

  useEffect(() => {
    if (!hardware) {
      void fetchHardwareInfo();
    }
  }, [hardware, fetchHardwareInfo]);

  const gpu = hardware?.gpu;
  const monitor = hardware?.monitor;
  const mice = hardware?.mice ?? [];
  const keyboards = hardware?.keyboards ?? [];

  const formatVram = (mb: number) => {
    if (mb >= 1024) {
      const gb = (mb / 1024).toFixed(1);
      return `${gb} GB (${mb} MB)`;
    }
    return `${mb} MB`;
  };

  return (
    <div className="dashboard-container">
      {/* Banner de error */}
      {(pollingError || hardwareError) && (
        <div className="banner banner-error" role="alert">
          <div>
            <span className="banner-code">[DIAG_AVISO]</span>
            <span>{pollingError || hardwareError}</span>
          </div>
          <button type="button" className="banner-close" onClick={dismissDiagError}>
            Cerrar
          </button>
        </div>
      )}

      {/* SECCIÓN 1: SISTEMA */}
      <section className="dashboard-section">
        <div className="section-header-row">
          <h2 className="section-heading" style={{ margin: 0 }}>
            1. Sistema de Renderizado y Pantalla
          </h2>
          <button
            type="button"
            className="btn btn-secondary btn-small"
            disabled={isHardwareLoading || isPollingAnalyzing}
            onClick={() => void fetchHardwareInfo()}
          >
            {isHardwareLoading ? "Detectando..." : "Actualizar Hardware"}
          </button>
        </div>

        <div className="diag-cards-grid">
          {/* Tarjeta GPU */}
          <div className="diag-card">
            <div className="diag-card-header">
              <span className="diag-card-tag">Unidad Gráfica (GPU)</span>
              <span className="badge badge-default">DXGI Nativo</span>
            </div>
            <h3 className="diag-card-title">{gpu ? gpu.name : "Consultando GPU..."}</h3>
            <div className="diag-metrics-list">
              <div className="diag-metric-row">
                <span className="meta-label">Memoria VRAM dedicada:</span>
                <span className="meta-mono">{gpu ? formatVram(gpu.vram_mb) : "..."}</span>
              </div>
              <div className="diag-metric-row">
                <span className="meta-label">Versión de Controlador:</span>
                <span className="meta-mono">{gpu ? gpu.driver_version : "..."}</span>
              </div>
            </div>
          </div>

          {/* Tarjeta Monitor */}
          <div className="diag-card">
            <div className="diag-card-header">
              <span className="diag-card-tag">Monitor y Tasa de Refresco</span>
              <span className="badge badge-default">DisplayConfig</span>
            </div>
            <h3 className="diag-card-title">
              {monitor ? `${monitor.refresh_hz} Hz · ${monitor.resolution}` : "Detectando pantalla..."}
            </h3>
            <div className="diag-metrics-list">
              <div className="diag-metric-row">
                <span className="meta-label">Resolución activa:</span>
                <span className="meta-mono">{monitor ? monitor.resolution : "..."}</span>
              </div>
              <div className="diag-metric-row">
                <span className="meta-label">Frecuencia vertical exacta:</span>
                <span className="meta-mono">{monitor ? `${monitor.refresh_hz} Hz` : "..."}</span>
              </div>
              <div className="diag-metric-row">
                <span className="meta-label">Soporte VRR (G-Sync/FreeSync):</span>
                <span className={`badge ${monitor?.supports_vrr ? "badge-applied" : "badge-default"}`}>
                  {monitor?.supports_vrr ? "Compatible (Frecuencia Variable)" : "Modo Fijo / Desactivado"}
                </span>
              </div>
            </div>
          </div>
        </div>
      </section>

      {/* SECCIÓN 2: PERIFÉRICOS */}
      <section className="dashboard-section">
        <h2 className="section-heading">2. Dispositivos de Entrada (Mouse y Teclado HID)</h2>
        <div className="diag-cards-grid">
          {/* Mice */}
          <div className="diag-card">
            <div className="diag-card-header">
              <span className="diag-card-tag">Mouse / Puntero Activo</span>
              <span className="badge badge-default">Raw Input HID</span>
            </div>
            {mice.length === 0 ? (
              <p className="tweak-desc">No se encontraron dispositivos de mouse en la lista Raw Input.</p>
            ) : (
              mice.map((m, idx) => (
                <div key={m.raw_path || idx} className="peripheral-item">
                  <div className="peripheral-title-row">
                    <strong className="peripheral-name">{m.name}</strong>
                    <span
                      className={`badge ${
                        m.transport === "USB"
                          ? "badge-applied"
                          : m.transport === "Bluetooth"
                          ? "badge-warn"
                          : "badge-default"
                      }`}
                    >
                      {m.transport}
                    </span>
                  </div>
                  <div className="peripheral-meta">
                    <span>VID: <code className="meta-mono">{m.vid}</code></span>
                    <span>PID: <code className="meta-mono">{m.pid}</code></span>
                  </div>
                </div>
              ))
            )}
          </div>

          {/* Keyboards */}
          <div className="diag-card">
            <div className="diag-card-header">
              <span className="diag-card-tag">Teclado(s) Detectado(s)</span>
              <span className="badge badge-default">Raw Input HID</span>
            </div>
            {keyboards.length === 0 ? (
              <p className="tweak-desc">No se encontraron teclados en la lista Raw Input.</p>
            ) : (
              keyboards.map((k, idx) => (
                <div key={k.raw_path || idx} className="peripheral-item">
                  <div className="peripheral-title-row">
                    <strong className="peripheral-name">{k.name}</strong>
                    <span className="badge badge-default">{k.transport}</span>
                  </div>
                  <div className="peripheral-meta">
                    <span>VID: <code className="meta-mono">{k.vid}</code></span>
                    <span>PID: <code className="meta-mono">{k.pid}</code></span>
                  </div>
                </div>
              ))
            )}
          </div>
        </div>
      </section>

      {/* SECCIÓN 3: ANÁLISIS DE POLLING RATE */}
      <section className="dashboard-section polling-analysis-section">
        <div className="polling-box-header">
          <div>
            <h2 className="section-heading" style={{ margin: 0 }}>
              3. Medición Científica de Polling Rate del Mouse
            </h2>
            <p className="tweak-desc" style={{ marginBottom: 0, marginTop: "4px" }}>
              Captura ráfagas de paquetes HID en tiempo real con <code>QueryPerformanceCounter</code> durante una ventana
              exacta de <strong>4 segundos</strong>. Al iniciar, mueve el cursor continuamente en círculos sobre la pantalla.
            </p>
          </div>
        </div>

        {/* Action Button & Progress */}
        <div className="polling-action-bar">
          <button
            type="button"
            className="btn btn-primary btn-large"
            disabled={isPollingAnalyzing}
            onClick={() => void startPollingAnalysis()}
          >
            {isPollingAnalyzing ? `Midiendo... ¡Mueve el mouse! (${pollingSecondsLeft}s)` : "Iniciar Análisis de Polling (4 seg)"}
          </button>

          {isPollingAnalyzing && (
            <div className="polling-spinner-container">
              <div className="polling-spinner" />
              <span className="polling-spinner-text">
                Capturando ráfaga HID con QPC · Mantén el mouse en movimiento ({pollingSecondsLeft}s restantes)...
              </span>
            </div>
          )}
        </div>

        {/* Tarjeta de Veredicto (Semáforo) */}
        {pollingResult && !isPollingAnalyzing && (
          <div className={`verdict-card verdict-${pollingResult.status}`}>
            <div className="verdict-header">
              <div>
                <span className="verdict-tag">Tarjeta de Veredicto de Latencia</span>
                <h3 className="verdict-title">{pollingResult.status_label}</h3>
              </div>
              <div className="verdict-hz-large">
                <span className="hz-number">{pollingResult.measured_hz}</span>
                <span className="hz-unit">Hz</span>
              </div>
            </div>

            <div className="verdict-grid">
              <div className="verdict-stat">
                <span className="stat-label">Jitter (Desviación):</span>
                <span className="stat-value">{pollingResult.jitter_ms} ms</span>
                <span className="stat-sub">
                  {pollingResult.jitter_ms < 0.1 ? "Excelente consistencia" : "Variación moderada/alta"}
                </span>
              </div>
              <div className="verdict-stat">
                <span className="stat-label">Intervalo Promedio:</span>
                <span className="stat-value">{pollingResult.avg_interval_ms} ms</span>
                <span className="stat-sub">Tiempo medio entre paquetes</span>
              </div>
              <div className="verdict-stat">
                <span className="stat-label">Intervalo Mín / Máx:</span>
                <span className="stat-value">
                  {pollingResult.min_interval_ms} / {pollingResult.max_interval_ms} ms
                </span>
                <span className="stat-sub">Rango de pulsos capturados</span>
              </div>
              <div className="verdict-stat">
                <span className="stat-label">Muestras Registradas:</span>
                <span className="stat-value">{pollingResult.sample_count}</span>
                <span className="stat-sub">Paquetes analizados en 4s</span>
              </div>
            </div>

            <div className="verdict-footer">
              <div className="verdict-explanation">
                {pollingResult.status === "optimo" && (
                  <p>
                    <strong>Veredicto Óptimo:</strong> El ratón entrega una tasa de refresco cercana a 1000 Hz o superior
                    con jitter inferior a 0.1 ms. La ruta de entrada USB reporta paquetes limpios y uniformes sin micro-cortes.
                  </p>
                )}
                {pollingResult.status === "aceptable" && (
                  <p>
                    <strong>Veredicto Aceptable:</strong> Tasa de sondeo entre 500 Hz y 1000 Hz. Apto para juego competitivo,
                    aunque con pequeñas fluctuaciones de jitter atribuibles a controladores o congestión del bus USB.
                  </p>
                )}
                {pollingResult.status === "critico" && (
                  <p>
                    <strong>Veredicto Crítico:</strong> Tasa de sondeo inferior a 125 Hz o transporte Bluetooth con latencia
                    inconsistente. Recomendación: conectar el receptor a un puerto USB nativo directo (sin hubs) y verificar
                    que la Suspensión Selectiva de USB esté desactivada en la pestaña Optimizar.
                  </p>
                )}
              </div>
              <button
                type="button"
                className="btn btn-secondary btn-small"
                onClick={clearPollingResult}
              >
                Limpiar Resultado
              </button>
            </div>
          </div>
        )}
      </section>
    </div>
  );
}
