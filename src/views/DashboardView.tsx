import { useEffect } from "react";
import { useDiagStore } from "../store/useDiagStore";

export function DashboardView() {
  const {
    hardware,
    pollingResult,
    drivers,
    nicModeration,
    usbTopology,
    networkResult,
    targetHost,
    isHardwareLoading,
    isPollingAnalyzing,
    isNetworkAnalyzing,
    isUsbLoading,
    isDriversLoading,
    pollingSecondsLeft,
    pollingError,
    networkError,
    hardwareError,
    fetchAllDiagnostics,
    startPollingAnalysis,
    startNetworkAnalysis,
    setTargetHost,
    clearPollingResult,
    clearNetworkResult,
    dismissDiagError,
  } = useDiagStore();

  useEffect(() => {
    void fetchAllDiagnostics();
  }, [fetchAllDiagnostics]);

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
      {/* Banner de errores / advertencias */}
      {(pollingError || networkError || hardwareError) && (
        <div className="banner banner-error" role="alert">
          <div>
            <span className="banner-code">[DIAG_AVISO]</span>
            <span>{pollingError || networkError || hardwareError}</span>
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
            disabled={isHardwareLoading || isPollingAnalyzing || isNetworkAnalyzing}
            onClick={() => void fetchAllDiagnostics()}
          >
            {isHardwareLoading ? "Detectando..." : "Actualizar Todo"}
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

      {/* SECCIÓN 2: CONTROLADORES Y ADAPTADOR DE RED */}
      <section className="dashboard-section">
        <h2 className="section-heading">2. Controladores de Red, Audio e Interrupt Moderation</h2>
        <div className="diag-cards-grid">
          {/* NIC Driver & Moderation Card */}
          <div className="diag-card">
            <div className="diag-card-header">
              <span className="diag-card-tag">Adaptador de Red Activo</span>
              <span className="badge badge-default">Net Class Win32</span>
            </div>
            <h3 className="diag-card-title">
              {drivers ? drivers.active_nic_name : "Consultando interfaz..."}
            </h3>
            <div className="diag-metrics-list">
              <div className="diag-metric-row">
                <span className="meta-label">Versión de Controlador NIC:</span>
                <span className="meta-mono">{drivers ? drivers.active_nic_driver : "..."}</span>
              </div>
              <div className="diag-metric-row">
                <span className="meta-label">Interrupt Moderation:</span>
                <span
                  className={`badge ${
                    nicModeration?.is_enabled ? "badge-warn" : "badge-applied"
                  }`}
                >
                  {nicModeration ? nicModeration.value_str : "Consultando..."}
                </span>
              </div>
            </div>

            {nicModeration && (
              <div
                className={`moderation-alert ${
                  nicModeration.is_enabled ? "moderation-warn" : "moderation-good"
                }`}
              >
                <strong>{nicModeration.verdict}</strong>
                <p>{nicModeration.explanation}</p>
              </div>
            )}
          </div>

          {/* Audio Driver Card */}
          <div className="diag-card">
            <div className="diag-card-header">
              <span className="diag-card-tag">Dispositivo de Audio Principal</span>
              <span className="badge badge-default">Media Class Win32</span>
            </div>
            <h3 className="diag-card-title">
              {drivers ? drivers.primary_audio_name : "Consultando audio..."}
            </h3>
            <div className="diag-metrics-list">
              <div className="diag-metric-row">
                <span className="meta-label">Versión de Controlador Audio:</span>
                <span className="meta-mono">{drivers ? drivers.primary_audio_driver : "..."}</span>
              </div>
              <div className="diag-metric-row">
                <span className="meta-label">Estado de Pila de Audio:</span>
                <span className="badge badge-default">Modo Exclusivo WASAPI Listo</span>
              </div>
            </div>
          </div>
        </div>
      </section>

      {/* SECCIÓN 3: TOPOLOGÍA USB Y CONTENCIÓN */}
      <section className="dashboard-section">
        <div className="section-header-row">
          <div>
            <h2 className="section-heading" style={{ margin: 0 }}>
              3. Topología USB y Detección de Contención (Root Hub Contention)
            </h2>
            <p className="tweak-desc" style={{ marginBottom: 0, marginTop: "4px" }}>
              Mapea qué periféricos comparten el mismo controlador raíz USB mediante <code>SetupAPI</code> y <code>CfgMgr32</code>.
              Compartir bus entre mouse y audio USB (transferencias isócronas continuas) provoca micro-cortes e inestabilidad en el sondeo.
            </p>
          </div>
        </div>

        {usbTopology?.any_contention_detected && (
          <div className="banner banner-error" role="alert" style={{ marginTop: "10px" }}>
            <div>
              <span className="banner-code">[ALERTA_CONTENCIÓN]</span>
              <span>
                Se detectó que tu mouse de juego comparte el controlador raíz USB con un dispositivo de audio.
                Conecta el receptor o cable del mouse en otro puerto USB (directo a la placa base, evitando hubs compartidos).
              </span>
            </div>
          </div>
        )}

        <div className="usb-controllers-grid" style={{ marginTop: "12px" }}>
          {isUsbLoading ? (
            <div className="empty-state">Explorando árbol de controladores USB...</div>
          ) : !usbTopology || usbTopology.controllers.length === 0 ? (
            <div className="empty-state">No se pudieron enumerar controladores USB.</div>
          ) : (
            usbTopology.controllers.map((ctrl) => (
              <div
                key={ctrl.controller_id}
                className={`usb-controller-card ${ctrl.has_contention ? "controller-contention" : ""}`}
              >
                <div className="usb-ctrl-header">
                  <div>
                    <h3 className="usb-ctrl-title">{ctrl.controller_name}</h3>
                    <span className="meta-mono" style={{ fontSize: "11px", color: "var(--text-secondary)" }}>
                      {ctrl.controller_id}
                    </span>
                  </div>
                  <span className={`badge ${ctrl.has_contention ? "badge-blocked" : "badge-applied"}`}>
                    {ctrl.has_contention ? "Contención Detectada" : "Canal Aislado"}
                  </span>
                </div>

                <div className="usb-devices-list">
                  {ctrl.devices.length === 0 ? (
                    <div className="empty-state" style={{ padding: "8px" }}>
                      Sin dispositivos secundarios registrados en este concentrador.
                    </div>
                  ) : (
                    ctrl.devices.map((dev) => (
                      <div key={dev.instance_id} className="usb-dev-row">
                        <div className="usb-dev-info">
                          <span className="usb-dev-name">{dev.device_name}</span>
                          <span className="meta-mono" style={{ fontSize: "10px", color: "var(--text-secondary)" }}>
                            {dev.instance_id}
                          </span>
                        </div>
                        <div className="usb-dev-badges">
                          {dev.is_mouse && <span className="badge badge-warn">Mouse</span>}
                          {dev.is_audio && <span className="badge badge-default">Audio USB</span>}
                        </div>
                      </div>
                    ))
                  )}
                </div>

                {ctrl.contention_details && (
                  <div className="inline-error" style={{ marginTop: "8px" }}>
                    {ctrl.contention_details}
                  </div>
                )}
              </div>
            ))
          )}
        </div>
      </section>

      {/* SECCIÓN 4: ANÁLISIS DE POLLING RATE DEL MOUSE */}
      <section className="dashboard-section polling-analysis-section">
        <div className="polling-box-header">
          <div>
            <h2 className="section-heading" style={{ margin: 0 }}>
              4. Medición Científica de Polling Rate del Mouse
            </h2>
            <p className="tweak-desc" style={{ marginBottom: 0, marginTop: "4px" }}>
              Captura ráfagas de paquetes HID en tiempo real con <code>QueryPerformanceCounter</code> durante una ventana
              exacta de <strong>4 segundos</strong>. Al iniciar, mueve el cursor continuamente en círculos sobre la pantalla.
            </p>
          </div>
        </div>

        <div className="polling-action-bar">
          <button
            type="button"
            className="btn btn-primary btn-large"
            disabled={isPollingAnalyzing || isNetworkAnalyzing}
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

        {pollingResult && !isPollingAnalyzing && (
          <div className={`verdict-card verdict-${pollingResult.status}`}>
            <div className="verdict-header">
              <div>
                <span className="verdict-tag">Tarjeta de Veredicto de Sondeo</span>
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

      {/* SECCIÓN 5: ANÁLISIS DE RED (ICMP + UDP JITTER) */}
      <section className="dashboard-section polling-analysis-section">
        <div className="polling-box-header">
          <div>
            <h2 className="section-heading" style={{ margin: 0 }}>
              5. Diagnóstico de Red y Jitter UDP (Tickrate 64)
            </h2>
            <p className="tweak-desc" style={{ marginBottom: 0, marginTop: "4px" }}>
              Ejecuta 20 solicitudes ICMP Echo nativas (<code>IcmpSendEcho</code>) y 50 paquetes UDP a intervalos de 15.6 ms
              (simulando tickrate de juego competitivo) hacia el host seleccionado para medir latencia y dispersión.
            </p>
          </div>
        </div>

        <div className="network-controls-bar">
          <div className="network-input-group">
            <label htmlFor="host-input" className="meta-label">
              Host / IP de Destino:
            </label>
            <input
              id="host-input"
              type="text"
              className="text-input"
              value={targetHost}
              disabled={isNetworkAnalyzing}
              onChange={(e) => setTargetHost(e.target.value)}
              placeholder="8.8.8.8"
            />
          </div>

          <button
            type="button"
            className="btn btn-primary btn-large"
            disabled={isNetworkAnalyzing || isPollingAnalyzing}
            onClick={() => void startNetworkAnalysis()}
          >
            {isNetworkAnalyzing ? "Midiendo Red (ICMP + UDP)..." : "Analizar Red"}
          </button>
        </div>

        {isNetworkAnalyzing && (
          <div className="polling-spinner-container" style={{ marginTop: "14px" }}>
            <div className="polling-spinner" />
            <span className="polling-spinner-text">
              Enviando ráfagas ICMP y paquetes UDP (tickrate 15.6ms)...
            </span>
          </div>
        )}

        {networkResult && !isNetworkAnalyzing && (
          <div className={`verdict-card verdict-${networkResult.status}`}>
            <div className="verdict-header">
              <div>
                <span className="verdict-tag">Veredicto de Calidad de Red</span>
                <h3 className="verdict-title">{networkResult.status_label}</h3>
                <span className="meta-mono" style={{ fontSize: "12px", color: "var(--text-secondary)" }}>
                  Destino: {networkResult.host}
                </span>
              </div>
              <div className="verdict-hz-large">
                <span className="hz-number">{networkResult.icmp.avg_ping_ms}</span>
                <span className="hz-unit">ms ping</span>
              </div>
            </div>

            <div className="verdict-grid">
              <div className="verdict-stat">
                <span className="stat-label">ICMP Ping Mín / Máx:</span>
                <span className="stat-value">
                  {networkResult.icmp.min_ping_ms} / {networkResult.icmp.max_ping_ms} ms
                </span>
                <span className="stat-sub">Rango de ida y vuelta</span>
              </div>
              <div className="verdict-stat">
                <span className="stat-label">ICMP Jitter:</span>
                <span className="stat-value">{networkResult.icmp.jitter_ms} ms</span>
                <span className="stat-sub">Desviación estándar</span>
              </div>
              <div className="verdict-stat">
                <span className="stat-label">Pérdida de Paquetes:</span>
                <span
                  className="stat-value"
                  style={{
                    color: networkResult.icmp.packet_loss_pct > 0 ? "#ef4444" : "var(--text-primary)",
                  }}
                >
                  {networkResult.icmp.packet_loss_pct}%
                </span>
                <span className="stat-sub">{networkResult.icmp.pings_received} / 20 recibidos</span>
              </div>
              <div className="verdict-stat">
                <span className="stat-label">UDP Jitter (Tickrate 64):</span>
                <span className="stat-value">{networkResult.udp.jitter_ms} ms</span>
                <span className="stat-sub">
                  RTT Prom: {networkResult.udp.avg_rtt_ms} ms ({networkResult.udp.packets_received}/50 pkts)
                </span>
              </div>
            </div>

            <div className="verdict-footer">
              <div className="verdict-explanation">
                <p>
                  <strong>Evaluación Técnica: </strong>
                  {networkResult.explanation}
                </p>
              </div>
              <button
                type="button"
                className="btn btn-secondary btn-small"
                onClick={clearNetworkResult}
              >
                Limpiar Red
              </button>
            </div>
          </div>
        )}
      </section>
    </div>
  );
}
