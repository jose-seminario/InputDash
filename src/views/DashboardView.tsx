import { useEffect, useState } from "react";
import { useDiagStore } from "../store/useDiagStore";
import { useTweaksStore } from "../store/useTweaksStore";
import {
  ipcStartBufferbloatAnalysis,
  ipcExportFullReport,
  listenToUsbDeviceChanges,
  ipcStartRealtimeMonitor,
  ipcStopRealtimeMonitor,
  listenToRealtimeLatency,
} from "../services/ipc";
import type { BufferbloatResult, FullExportPayload, RealtimeLatencyTick } from "../types/tweak";

export function DashboardView() {
  const {
    hardware,
    pollingResult,
    drivers,
    nicModeration,
    usbTopology,
    networkResult,
    frametimeResult,
    dpcResult,
    telReport,
    targetHost,
    isHardwareLoading,
    isPollingAnalyzing,
    isNetworkAnalyzing,
    isUsbLoading,
    isDriversLoading,
    isFrametimeAnalyzing,
    isDpcAnalyzing,
    isTelLoading,
    pollingSecondsLeft,
    frametimeSecondsLeft,
    dpcSecondsLeft,
    pollingError,
    networkError,
    hardwareError,
    frametimeError,
    dpcError,
    telError,
    fetchAllDiagnostics,
    fetchHardwareInfo,
    fetchUsbTopology,
    startPollingAnalysis,
    startNetworkAnalysis,
    startFrametimeAnalysis,
    startDpcAnalysis,
    updateTelReport,
    setTargetHost,
    clearPollingResult,
    clearNetworkResult,
    clearFrametimeResult,
    clearDpcResult,
    dismissDiagError,
  } = useDiagStore();

  const [selectedTickrate, setSelectedTickrate] = useState<number>(64);
  const { backups, logs, abSession } = useTweaksStore();
  const [isBufferbloatAnalyzing, setIsBufferbloatAnalyzing] = useState<boolean>(false);
  const [bufferbloatResult, setBufferbloatResult] = useState<BufferbloatResult | null>(null);
  const [bufferbloatError, setBufferbloatError] = useState<string | null>(null);
  const [isExporting, setIsExporting] = useState<boolean>(false);
  const [exportSuccessMsg, setExportSuccessMsg] = useState<string | null>(null);
  const [usbChangeAlert, setUsbChangeAlert] = useState<string | null>(null);
  const [isRealtimeRunning, setIsRealtimeRunning] = useState<boolean>(false);
  const [realtimeTick, setRealtimeTick] = useState<RealtimeLatencyTick | null>(null);
  const [initialBaselineLatency, setInitialBaselineLatency] = useState<number | null>(null);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    listenToRealtimeLatency((tick) => {
      setRealtimeTick(tick);
      setInitialBaselineLatency((prev) => (prev === null ? tick.total_ms : prev));
    })
      .then((unsub) => {
        unlisten = unsub;
      })
      .catch((err) => {
        console.warn("No se pudo conectar listener de latencia en tiempo real:", err);
      });

    return () => {
      if (unlisten) unlisten();
      void ipcStopRealtimeMonitor();
    };
  }, []);

  const handleToggleRealtime = async () => {
    if (isRealtimeRunning) {
      await ipcStopRealtimeMonitor();
      setIsRealtimeRunning(false);
    } else {
      await ipcStartRealtimeMonitor();
      setIsRealtimeRunning(true);
    }
  };

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    listenToUsbDeviceChanges((msg) => {
      setUsbChangeAlert(msg || "Cambio de periférico detectado. Por favor, vuelve a medir el polling.");
      void fetchUsbTopology();
      void fetchHardwareInfo();
    })
      .then((unsub) => {
        unlisten = unsub;
      })
      .catch((err) => {
        console.warn("No se pudo registrar listener de hot-plug USB:", err);
      });

    return () => {
      if (unlisten) unlisten();
    };
  }, [fetchUsbTopology, fetchHardwareInfo]);

  const isLowPolling = Boolean(pollingResult && pollingResult.measured_hz <= 250);
  const activeUsbMouse = hardware?.mice.find(
    (m) =>
      m.transport === "USB" ||
      m.name.toLowerCase().includes("gaming") ||
      m.name.toLowerCase().includes("mouse") ||
      m.name.toLowerCase().includes("ratón")
  );
  const hasSharedHubOrContention = Boolean(
    usbTopology?.controllers.some((c) => c.devices.length > 1 && c.devices.some((d) => d.is_mouse)) ||
      usbTopology?.any_contention_detected
  );
  const showPortBottleneckAlert = isLowPolling && Boolean(activeUsbMouse);

  const handleStartBufferbloat = async () => {
    setIsBufferbloatAnalyzing(true);
    setBufferbloatError(null);
    try {
      const res = await ipcStartBufferbloatAnalysis(targetHost.trim() || undefined);
      setBufferbloatResult(res);
    } catch (err) {
      setBufferbloatError(String(err));
    } finally {
      setIsBufferbloatAnalyzing(false);
    }
  };

  const handleExportFullReport = async () => {
    setIsExporting(true);
    setExportSuccessMsg(null);
    try {
      const payload: FullExportPayload = {
        hardware: hardware ?? undefined,
        network: networkResult ?? undefined,
        bufferbloat: bufferbloatResult ?? undefined,
        polling: pollingResult ?? undefined,
        tel: telReport ?? undefined,
        ab_session: abSession ?? undefined,
        active_backups: backups ?? undefined,
        change_logs: logs ?? undefined,
      };
      const res = await ipcExportFullReport(payload);
      setExportSuccessMsg(res.message);
    } catch (err) {
      setBufferbloatError(String(err));
    } finally {
      setIsExporting(false);
    }
  };

  useEffect(() => {
    void fetchAllDiagnostics();
  }, [fetchAllDiagnostics]);

  const gpu = hardware?.gpu;
  const monitor = hardware?.monitor;
  const mice = hardware?.mice ?? [];
  const keyboards = hardware?.keyboards ?? [];

  const isAnyLoading = isHardwareLoading || isDriversLoading || isUsbLoading;
  const anyActiveAnalysis =
    isPollingAnalyzing || isNetworkAnalyzing || isFrametimeAnalyzing || isDpcAnalyzing || isBufferbloatAnalyzing;

  const formatVram = (mb: number) => {
    if (mb >= 1024) {
      const gb = (mb / 1024).toFixed(1);
      return `${gb} GB (${mb} MB)`;
    }
    return `${mb} MB`;
  };

  const handleTickrateChange = (newTickrate: number) => {
    setSelectedTickrate(newTickrate);
    void updateTelReport(newTickrate);
  };

  const activeError =
    pollingError ||
    networkError ||
    hardwareError ||
    frametimeError ||
    dpcError ||
    telError;

  return (
    <div className="dashboard-container">
      {/* Banner de errores / advertencias */}
      {activeError && (
        <div className="banner banner-error" role="alert">
          <div>
            <span className="banner-code">[DIAG_AVISO]</span>
            <span>{activeError}</span>
          </div>
          <button type="button" className="banner-close" onClick={dismissDiagError}>
            Cerrar
          </button>
        </div>
      )}

      {/* WIDGET FASE 9: MONITOR DE LATENCIA DEL SISTEMA EN TIEMPO REAL */}
      <section
        className="dashboard-section"
        style={{
          border: "1px solid var(--accent-primary, #3b82f6)",
          background: "rgba(15, 23, 42, 0.65)",
          borderRadius: "8px",
          padding: "16px",
          marginBottom: "16px",
        }}
      >
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", flexWrap: "wrap", gap: "16px" }}>
          <div>
            <div style={{ display: "flex", alignItems: "center", gap: "8px" }}>
              <span className="badge badge-applied" style={{ textTransform: "uppercase", fontSize: "11px" }}>
                Fase 9 · Tiempo Real
              </span>
              <span className="meta-mono" style={{ fontSize: "11px", color: "var(--text-secondary)" }}>
                Latencia Parásita del SO (excluye monitor y red)
              </span>
            </div>
            <div style={{ display: "flex", alignItems: "baseline", gap: "12px", marginTop: "6px" }}>
              <h2 style={{ fontSize: "28px", fontWeight: 700, margin: 0, fontFamily: "monospace" }}>
                Latencia del Sistema:{" "}
                <span style={{ color: isRealtimeRunning ? "#38bdf8" : "var(--text-secondary)" }}>
                  {realtimeTick && isRealtimeRunning ? `${realtimeTick.total_ms} ms` : "-- ms"}
                </span>
              </h2>
              {isRealtimeRunning && initialBaselineLatency !== null && realtimeTick && (
                <span className="badge badge-default" style={{ fontSize: "12px" }}>
                  Actual: <strong>{realtimeTick.total_ms} ms</strong> | Antes: {initialBaselineLatency} ms
                  {realtimeTick.total_ms < initialBaselineLatency && (
                    <span style={{ color: "#4ade80", marginLeft: "4px" }}>
                      ({Math.round(((realtimeTick.total_ms - initialBaselineLatency) / initialBaselineLatency) * 100)}%)
                    </span>
                  )}
                </span>
              )}
            </div>
            <div style={{ display: "flex", gap: "16px", flexWrap: "wrap", marginTop: "8px", fontSize: "12px", color: "var(--text-secondary)" }}>
              <span>🖱️ Mouse HID: <strong style={{ color: "var(--text-primary)" }}>{realtimeTick && isRealtimeRunning ? `${realtimeTick.mouse_ms} ms` : "--"}</strong></span>
              <span>⚙️ DPC / Kernel: <strong style={{ color: "var(--text-primary)" }}>{realtimeTick && isRealtimeRunning ? `${realtimeTick.dpc_ms} ms` : "--"}</strong></span>
              <span>
                🎮 Frame Time: <strong style={{ color: "var(--text-primary)" }}>{realtimeTick && isRealtimeRunning ? `${realtimeTick.frametime_ms} ms` : "--"}</strong>
                <span style={{ marginLeft: "4px", fontSize: "11px", fontStyle: "italic", opacity: 0.85 }}>
                  ({realtimeTick?.is_game_active ? "Juego 3D Activo" : "DWM Compositor de Escritorio"})
                </span>
              </span>
            </div>
            {!realtimeTick?.is_game_active && isRealtimeRunning && (
              <p className="tweak-desc" style={{ margin: "6px 0 0 0", fontSize: "11px", opacity: 0.8 }}>
                ℹ️ Nota: No se detectó un juego 3D abierto en primer plano; el Frame Time mide la tasa de refresco del compositor de ventanas de Windows (DWM).
              </p>
            )}
          </div>

          <div style={{ display: "flex", gap: "8px", alignItems: "center" }}>
            {!isRealtimeRunning ? (
              <button
                type="button"
                className="btn btn-primary btn-large"
                onClick={() => void handleToggleRealtime()}
              >
                ▶ Iniciar Monitoreo
              </button>
            ) : (
              <button
                type="button"
                className="btn btn-secondary btn-large"
                style={{ borderColor: "#ef4444", color: "#f87171" }}
                onClick={() => void handleToggleRealtime()}
              >
                ⏹ Detener Monitoreo
              </button>
            )}
          </div>
        </div>
      </section>

      {/* SECCIÓN 1: SISTEMA */}
      <section className="dashboard-section">
        <div className="section-header-row">
          <h2 className="section-heading" style={{ margin: 0 }}>
            1. Sistema de Renderizado y Pantalla
          </h2>
          <div style={{ display: "flex", gap: "8px", alignItems: "center" }}>
            <button
              type="button"
              className="btn btn-primary btn-small"
              disabled={isExporting}
              onClick={() => void handleExportFullReport()}
            >
              {isExporting ? "Exportando..." : "Exportar Reporte Completo"}
            </button>
            <button
              type="button"
              className="btn btn-secondary btn-small"
              disabled={isAnyLoading || anyActiveAnalysis}
              onClick={() => void fetchAllDiagnostics()}
            >
              {isAnyLoading ? "Detectando..." : "Actualizar Todo"}
            </button>
          </div>
        </div>

        {exportSuccessMsg && (
          <div className="banner banner-success" role="alert" style={{ marginTop: "10px" }}>
            <span>✓ {exportSuccessMsg}</span>
            <button type="button" className="banner-close" onClick={() => setExportSuccessMsg(null)}>
              Cerrar
            </button>
          </div>
        )}

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

      {/* SECCIÓN 2: PERIFÉRICOS DE ENTRADA (MOUSE Y TECLADO HID) */}
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
                    <span>
                      VID: <code className="meta-mono">{m.vid}</code>
                    </span>
                    <span>
                      PID: <code className="meta-mono">{m.pid}</code>
                    </span>
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
                    <span>
                      VID: <code className="meta-mono">{k.vid}</code>
                    </span>
                    <span>
                      PID: <code className="meta-mono">{k.pid}</code>
                    </span>
                  </div>
                </div>
              ))
            )}
          </div>
        </div>
      </section>

      {/* SECCIÓN 3: CONTROLADORES Y ADAPTADOR DE RED */}
      <section className="dashboard-section">
        <h2 className="section-heading">3. Controladores de Red, Audio e Interrupt Moderation</h2>
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

      {/* SECCIÓN 4: TOPOLOGÍA USB Y CONTENCIÓN */}
      <section className="dashboard-section">
        <div className="section-header-row">
          <div>
            <h2 className="section-heading" style={{ margin: 0 }}>
              4. Topología USB y Detección de Contención (Root Hub Contention)
            </h2>
            <p className="tweak-desc" style={{ marginBottom: 0, marginTop: "4px" }}>
              Mapea qué periféricos comparten el mismo controlador raíz USB mediante <code>SetupAPI</code> y <code>CfgMgr32</code>.
              Compartir bus entre mouse y audio USB (transferencias isócronas continuas) provoca micro-cortes e inestabilidad en el sondeo.
            </p>
          </div>
          <button
            type="button"
            className="btn btn-secondary btn-small"
            disabled={isUsbLoading}
            onClick={() => {
              void fetchUsbTopology();
              void fetchHardwareInfo();
            }}
          >
            {isUsbLoading ? "Escaneando..." : "Re-escanear USB"}
          </button>
        </div>

        {usbChangeAlert && (
          <div className="banner banner-warn" role="alert" style={{ marginTop: "10px" }}>
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", width: "100%", gap: "10px" }}>
              <span>🔌 <strong>{usbChangeAlert}</strong> (Topología USB actualizada automáticamente).</span>
              <button
                type="button"
                className="btn btn-small btn-primary"
                onClick={() => {
                  setUsbChangeAlert(null);
                  void startPollingAnalysis();
                }}
              >
                Medir Polling Ahora
              </button>
            </div>
          </div>
        )}

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

      {/* SECCIÓN 5: ANÁLISIS DE POLLING RATE DEL MOUSE */}
      <section className="dashboard-section polling-analysis-section">
        <div className="polling-box-header">
          <div>
            <h2 className="section-heading" style={{ margin: 0 }}>
              5. Medición Científica de Polling Rate del Mouse
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
            disabled={anyActiveAnalysis}
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
                {showPortBottleneckAlert && (
                  <div className="banner banner-warn" role="alert" style={{ margin: "0 0 12px 0", textAlign: "left" }}>
                    <div style={{ display: "flex", flexDirection: "column", gap: "4px" }}>
                      <strong style={{ display: "flex", alignItems: "center", gap: "6px" }}>
                        <span>⚠️</span> Limitación Física de Puerto USB Detectada ({pollingResult.measured_hz} Hz)
                      </strong>
                      <span>
                        Tu mouse <strong>{activeUsbMouse?.name || "USB"}</strong> soporta 1000Hz, pero el puerto actual parece limitado ({pollingResult.measured_hz} Hz)
                        {hasSharedHubOrContention ? " o comparte bus en el concentrador USB" : ""}.
                        Considera conectarlo directamente a un puerto USB 3.0 trasero (directo al chipset/CPU) o usar una tarjeta PCIe USB.
                      </span>
                    </div>
                  </div>
                )}
                <p>
                  <strong>Evaluación Técnica: </strong>
                  {pollingResult.status === "optimo" &&
                    "El ratón entrega una tasa de refresco uniforme con jitter despreciable (< 0.1 ms). Los micro-pulsos viajan limpios al kernel."}
                  {pollingResult.status === "aceptable" &&
                    "Tasa de sondeo entre 500 Hz y 1000 Hz. Apto para juego competitivo con pequeñas variaciones atribuibles a congestión de bus."}
                  {pollingResult.status === "critico" &&
                    "Tasa de sondeo deficiente o transporte Bluetooth. Conecta el receptor directo al puerto nativo de la placa base."}
                </p>
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

      {/* SECCIÓN 6: ANÁLISIS DE FRAME TIME ETW (DXGI / DxgKrnl) */}
      <section className="dashboard-section polling-analysis-section">
        <div className="polling-box-header">
          <div>
            <h2 className="section-heading" style={{ margin: 0 }}>
              6. Medición de Frame Times vía ETW (Microsoft-Windows-DxgKrnl / DXGI)
            </h2>
            <p className="tweak-desc" style={{ marginBottom: 0, marginTop: "4px" }}>
              Rastreo de eventos ETW de presentación 3D durante <strong>10 segundos</strong> con <code>ProcessTrace</code> y guardas RAII.
              Abre un juego 3D o benchmark en pantalla completa o ventana antes de pulsar analizar.
            </p>
          </div>
        </div>

        <div className="polling-action-bar">
          <button
            type="button"
            className="btn btn-primary btn-large"
            disabled={anyActiveAnalysis}
            onClick={() => void startFrametimeAnalysis()}
          >
            {isFrametimeAnalyzing
              ? `Capturando cuadros ETW... (${frametimeSecondsLeft}s)`
              : "Analizar Frame Time (10 seg)"}
          </button>

          {isFrametimeAnalyzing && (
            <div className="polling-spinner-container">
              <div className="polling-spinner" />
              <span className="polling-spinner-text">
                Rastreando eventos de presentación DXGI/DxgKrnl ({frametimeSecondsLeft}s restantes)...
              </span>
            </div>
          )}
        </div>

        {frametimeResult && !isFrametimeAnalyzing && (
          <div className={`verdict-card verdict-${frametimeResult.status}`}>
            <div className="verdict-header">
              <div>
                <span className="verdict-tag">Veredicto de Consistencia Gráfica</span>
                <h3 className="verdict-title">{frametimeResult.status_label}</h3>
                <span className="meta-mono" style={{ fontSize: "11px", color: "var(--text-secondary)" }}>
                  PID del proceso 3D: {frametimeResult.process_id} · {frametimeResult.total_frames} cuadros registrados en 10s
                </span>
              </div>
              <div className="verdict-hz-large">
                <span className="hz-number">{frametimeResult.avg_frametime_ms}</span>
                <span className="hz-unit">ms frame</span>
              </div>
            </div>

            <div className="verdict-grid">
              <div className="verdict-stat">
                <span className="stat-label">FPS Promedio:</span>
                <span className="stat-value">{frametimeResult.avg_fps} FPS</span>
                <span className="stat-sub">1000 / Frame Time medio</span>
              </div>
              <div className="verdict-stat">
                <span className="stat-label">1% Low (Percentil 1%):</span>
                <span
                  className="stat-value"
                  style={{
                    color: frametimeResult.fps_1pct_low < 60 ? "#ef4444" : "var(--text-primary)",
                  }}
                >
                  {frametimeResult.fps_1pct_low} FPS
                </span>
                <span className="stat-sub">Mide consistencia ante micro-stuttering</span>
              </div>
              <div className="verdict-stat">
                <span className="stat-label">0.1% Low (Picos Críticos):</span>
                <span className="stat-value">{frametimeResult.fps_01pct_low} FPS</span>
                <span className="stat-sub">Peores tirones de la muestra</span>
              </div>
              <div className="verdict-stat">
                <span className="stat-label">Rango Min / Max:</span>
                <span className="stat-value">
                  {frametimeResult.min_frametime_ms} / {frametimeResult.max_frametime_ms} ms
                </span>
                <span className="stat-sub">Dispersión del ciclo de renderizado</span>
              </div>
            </div>

            <div className="verdict-footer">
              <div className="verdict-explanation">
                <p>
                  <strong>Diagnóstico de Frame Pacing: </strong>
                  {frametimeResult.explanation}
                </p>
              </div>
              <button
                type="button"
                className="btn btn-secondary btn-small"
                onClick={clearFrametimeResult}
              >
                Limpiar Frame Time
              </button>
            </div>
          </div>
        )}
      </section>

      {/* SECCIÓN 7: ANÁLISIS DE LATENCIA DPC / ISR DEL KERNEL */}
      <section className="dashboard-section polling-analysis-section">
        <div className="polling-box-header">
          <div>
            <h2 className="section-heading" style={{ margin: 0 }}>
              7. Latencia DPC / ISR del Kernel (NT Kernel Logger)
            </h2>
            <p className="tweak-desc" style={{ marginBottom: 0, marginTop: "4px" }}>
              Captura eventos de interrupción diferida (DPC) y llamadas de servicio de interrupción (ISR) durante <strong>10 segundos</strong>.
              Identifica si controladores de audio, red o almacenamiento generan picos de retardo en el hilo del procesador.
            </p>
          </div>
        </div>

        <div className="polling-action-bar">
          <button
            type="button"
            className="btn btn-primary btn-large"
            disabled={anyActiveAnalysis}
            onClick={() => void startDpcAnalysis()}
          >
            {isDpcAnalyzing
              ? `Analizando DPC/ISR en Kernel... (${dpcSecondsLeft}s)`
              : "Analizar DPC / ISR (10 seg)"}
          </button>

          {isDpcAnalyzing && (
            <div className="polling-spinner-container">
              <div className="polling-spinner" />
              <span className="polling-spinner-text">
                Rastreando interrupciones de controladores con NT Kernel Logger ({dpcSecondsLeft}s restantes)...
              </span>
            </div>
          )}
        </div>

        {dpcResult && !isDpcAnalyzing && (
          <div className={`verdict-card verdict-${dpcResult.status}`}>
            <div className="verdict-header">
              <div>
                <span className="verdict-tag">Veredicto de Latencia DPC de Drivers</span>
                <h3 className="verdict-title">{dpcResult.status_label}</h3>
                <span className="meta-mono" style={{ fontSize: "11px", color: "var(--text-secondary)" }}>
                  {dpcResult.total_events} eventos de interrupción analizados en 10s
                </span>
              </div>
              <div className="verdict-hz-large">
                <span className="hz-number">{dpcResult.highest_dpc_us}</span>
                <span className="hz-unit">µs pico</span>
              </div>
            </div>

            <div style={{ marginTop: "12px" }}>
              <strong style={{ fontSize: "12px", textTransform: "uppercase", letterSpacing: "0.5px" }}>
                Top 5 Controladores con Mayor Tiempo DPC Registrado:
              </strong>
              <div className="usb-devices-list" style={{ marginTop: "8px" }}>
                {dpcResult.top_drivers.map((drv) => (
                  <div key={drv.driver_name} className="usb-dev-row">
                    <div className="usb-dev-info">
                      <span className="usb-dev-name" style={{ fontFamily: "monospace", fontWeight: 600 }}>
                        {drv.driver_name}
                      </span>
                      <span className="meta-mono" style={{ fontSize: "11px", color: "var(--text-secondary)" }}>
                        {drv.total_calls} ejecuciones · Promedio: {drv.avg_dpc_us} µs
                      </span>
                    </div>
                    <div className="usb-dev-badges">
                      <span
                        className={`badge ${
                          drv.max_dpc_us < 500
                            ? "badge-applied"
                            : drv.max_dpc_us < 1000
                            ? "badge-warn"
                            : "badge-blocked"
                        }`}
                      >
                        Pico: {drv.max_dpc_us} µs
                      </span>
                    </div>
                  </div>
                ))}
              </div>
            </div>

            <div className="verdict-footer" style={{ marginTop: "14px" }}>
              <div className="verdict-explanation">
                <p>
                  <strong>Diagnóstico de Drivers: </strong>
                  {dpcResult.explanation}
                </p>
              </div>
              <button
                type="button"
                className="btn btn-secondary btn-small"
                onClick={clearDpcResult}
              >
                Limpiar DPC
              </button>
            </div>
          </div>
        )}
      </section>

      {/* SECCIÓN 8: ANÁLISIS DE RED (ICMP + UDP JITTER) */}
      <section className="dashboard-section polling-analysis-section">
        <div className="polling-box-header">
          <div>
            <h2 className="section-heading" style={{ margin: 0 }}>
              8. Diagnóstico de Red y Jitter UDP (Tickrate 64)
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
            disabled={anyActiveAnalysis}
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

      {/* SECCIÓN 9: DIAGNÓSTICO DE BUFFERBLOAT (LATENCIA BAJO CARGA) */}
      <section className="dashboard-section polling-analysis-section">
        <div className="polling-box-header">
          <div>
            <h2 className="section-heading" style={{ margin: 0 }}>
              9. Diagnóstico de Bufferbloat (Latencia Bajo Carga de Red)
            </h2>
            <p className="tweak-desc" style={{ marginBottom: 0, marginTop: "4px" }}>
              Mide la degradación del ping cuando el enlace de red está saturado de tráfico. Compara el RTT en reposo frente al RTT bajo saturación con 4 conexiones simultáneas.
            </p>
          </div>
          <button
            type="button"
            className="btn btn-primary"
            disabled={isBufferbloatAnalyzing || anyActiveAnalysis}
            onClick={() => void handleStartBufferbloat()}
          >
            {isBufferbloatAnalyzing ? "Midiendo Saturación (10s)..." : "Iniciar Test de Bufferbloat (10s)"}
          </button>
        </div>

        {bufferbloatError && (
          <div className="banner banner-error" style={{ marginTop: "12px" }}>
            <span>[BUFFERBLOAT_ERROR] {bufferbloatError}</span>
            <button type="button" className="banner-close" onClick={() => setBufferbloatError(null)}>
              Cerrar
            </button>
          </div>
        )}

        {isBufferbloatAnalyzing && (
          <div className="analysis-progress-panel" style={{ marginTop: "14px" }}>
            <div className="spinner-row">
              <span className="spinner" />
              <span>
                Ejecutando Fase 1 (Reposo) y Fase 2 (Saturación TCP 4 streams)... Espera 10 segundos.
              </span>
            </div>
          </div>
        )}

        {bufferbloatResult && (
          <div className={`verdict-card verdict-${bufferbloatResult.status}`} style={{ marginTop: "14px" }}>
            <div className="verdict-header">
              <div>
                <span className="verdict-tag">Grado de Bufferbloat</span>
                <h3 className="verdict-title">Calificación: Grado {bufferbloatResult.grade}</h3>
                <span className="meta-mono" style={{ fontSize: "11px", color: "var(--text-secondary)" }}>
                  {bufferbloatResult.status_label} · Host: {targetHost.trim() || "8.8.8.8"}
                </span>
              </div>
              <div className="verdict-hz-large">
                <span className="hz-number">+{bufferbloatResult.delta_ms}</span>
                <span className="hz-unit">ms colapso</span>
              </div>
            </div>

            <div className="verdict-grid">
              <div className="verdict-stat">
                <span className="stat-label">Latencia en Reposo (Idle):</span>
                <span className="stat-value">{bufferbloatResult.baseline_ping_ms} ms</span>
                <span className="stat-sub">RTT sin saturación</span>
              </div>
              <div className="verdict-stat">
                <span className="stat-label">Latencia Bajo Carga (Loaded):</span>
                <span className="stat-value">{bufferbloatResult.loaded_ping_ms} ms</span>
                <span className="stat-sub">RTT con saturación TCP</span>
              </div>
              <div className="verdict-stat">
                <span className="stat-label">Incremento Porcentual:</span>
                <span
                  className="stat-value"
                  style={{
                    color: bufferbloatResult.delta_pct > 30 ? "#ef4444" : "#10b981",
                  }}
                >
                  +{bufferbloatResult.delta_pct}%
                </span>
                <span className="stat-sub">Crecimiento de la cola del router</span>
              </div>
              <div className="verdict-stat">
                <span className="stat-label">Recomendación SQM:</span>
                <span className="stat-value" style={{ fontSize: "15px" }}>
                  {bufferbloatResult.delta_ms > 15 ? "Requiere SQM (CAKE / FQ-CoDel)" : "Router Estable"}
                </span>
                <span className="stat-sub">
                  {bufferbloatResult.delta_ms > 15 ? "Alta congestión en cola" : "Sin hinchazón de buffer"}
                </span>
              </div>
            </div>

            <div className="verdict-footer">
              <div className="verdict-explanation">
                <p>
                  <strong>Diagnóstico de Red: </strong>
                  {bufferbloatResult.explanation}
                </p>
                {bufferbloatResult.delta_ms > 15 && (
                  <p style={{ marginTop: "4px", fontSize: "12px", color: "var(--text-secondary)" }}>
                    💡 Ve a la pestaña <strong>Guías In-Game</strong> para aprender a activar SQM / CAKE en tu router doméstico.
                  </p>
                )}
              </div>
              <button
                type="button"
                className="btn btn-secondary btn-small"
                onClick={() => setBufferbloatResult(null)}
              >
                Limpiar Bufferbloat
              </button>
            </div>
          </div>
        )}
      </section>

      {/* SECCIÓN 10: TOTAL ENGAGEMENT LATENCY (TEL) & VENTANA DE VENTAJA */}
      <section className="dashboard-section polling-analysis-section">
        <div className="polling-box-header">
          <div>
            <h2 className="section-heading" style={{ margin: 0 }}>
              10. Total Engagement Latency (TEL) & Ventana de Ventaja Competitiva
            </h2>
            <p className="tweak-desc" style={{ marginBottom: 0, marginTop: "4px" }}>
              Calcula matemáticamente el retardo integral de la cadena: <code>Polling USB</code> + <code>Frame Time</code> + <code>Scanout Monitor</code> + <code>One-Way Ping</code> + <code>Mitad de Tick</code>.
            </p>
          </div>
          <div style={{ display: "flex", gap: "8px", alignItems: "center" }}>
            <span className="meta-label">Tickrate Servidor:</span>
            <button
              type="button"
              className={`btn btn-small ${selectedTickrate === 64 ? "btn-primary" : "btn-secondary"}`}
              onClick={() => handleTickrateChange(64)}
            >
              64 Tick
            </button>
            <button
              type="button"
              className={`btn btn-small ${selectedTickrate === 128 ? "btn-primary" : "btn-secondary"}`}
              onClick={() => handleTickrateChange(128)}
            >
              128 Tick
            </button>
          </div>
        </div>

        {telReport && (
          <div className={`verdict-card verdict-${telReport.verdict}`} style={{ marginTop: "14px" }}>
            <div className="verdict-header">
              <div>
                <span className="verdict-tag">Índice Científico de Latencia Total</span>
                <h3 className="verdict-title">{telReport.verdict_label}</h3>
                <span className="meta-mono" style={{ fontSize: "11px", color: "var(--text-secondary)" }}>
                  {telReport.is_complete
                    ? "✓ Todos los componentes físicos y de red han sido verificados"
                    : `⚠ Diagnóstico Parcial: Faltan ${telReport.missing_components.length} mediciones`}
                </span>
              </div>
              <div className="verdict-hz-large">
                <span className="hz-number">{telReport.total_tel_ms}</span>
                <span className="hz-unit">ms TEL</span>
              </div>
            </div>

            {/* Desglose por componentes */}
            <div style={{ marginTop: "14px" }}>
              <strong style={{ fontSize: "12px", textTransform: "uppercase", letterSpacing: "0.5px" }}>
                Desglose Paso a Paso de la Cadena de Latencia:
              </strong>
              <div className="usb-devices-list" style={{ marginTop: "8px" }}>
                {telReport.components.map((comp) => (
                  <div key={comp.name} className="usb-dev-row">
                    <div className="usb-dev-info">
                      <span className="usb-dev-name">{comp.name}</span>
                      <span className="meta-mono" style={{ fontSize: "11px", color: "var(--text-secondary)" }}>
                        {comp.description} · Fórmula: <code>{comp.formula}</code>
                      </span>
                    </div>
                    <div className="usb-dev-badges">
                      <span className={`badge ${comp.is_measured ? "badge-applied" : "badge-default"}`}>
                        {comp.is_measured ? "Medido" : "Estimado"}
                      </span>
                      <strong className="meta-mono" style={{ minWidth: "65px", textAlign: "right" }}>
                        +{comp.latency_ms} ms
                      </strong>
                    </div>
                  </div>
                ))}
              </div>
            </div>

            {/* Ventana de ventaja vs enemigo */}
            <div style={{ marginTop: "16px" }}>
              <strong style={{ fontSize: "12px", textTransform: "uppercase", letterSpacing: "0.5px" }}>
                Ventana de Ventaja en Duelo de Reacción (vs Ping Enemigo):
              </strong>
              <div className="verdict-grid" style={{ marginTop: "8px" }}>
                {telReport.enemy_comparisons.map((c) => (
                  <div key={c.enemy_ping_ms} className="verdict-stat">
                    <span className="stat-label">Oponente con Ping {c.enemy_ping_ms} ms:</span>
                    <span
                      className="stat-value"
                      style={{
                        color:
                          c.status === "ventaja"
                            ? "#10b981"
                            : c.status === "desventaja"
                            ? "#ef4444"
                            : "var(--text-primary)",
                      }}
                    >
                      {c.advantage_ms > 0 ? `+${c.advantage_ms}` : c.advantage_ms} ms
                    </span>
                    <span className="stat-sub">
                      {c.status === "ventaja"
                        ? "Ventaja de registro en servidor"
                        : c.status === "desventaja"
                        ? "Desventaja de latencia"
                        : "Respuesta sincronizada"}
                    </span>
                  </div>
                ))}
              </div>
            </div>

            <div className="verdict-footer" style={{ marginTop: "16px" }}>
              <div className="verdict-explanation">
                <p>
                  <strong>Evaluación Integral TEL: </strong>
                  {telReport.explanation}
                </p>
              </div>
              <button
                type="button"
                className="btn btn-secondary btn-small"
                disabled={isTelLoading}
                onClick={() => void updateTelReport(selectedTickrate)}
              >
                {isTelLoading ? "Recalculando..." : "Recalcular TEL"}
              </button>
            </div>
          </div>
        )}
      </section>
    </div>
  );
}
