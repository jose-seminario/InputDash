import { useState, useEffect, useCallback } from "react";
import {
  ipcGetGameProfiles,
  ipcToggleGameProfile,
  ipcUpdateGameProfile,
  ipcGetCpuTopology,
  ipcGetCoreContention,
  ipcGetTimerResolution,
} from "../services/ipc";
import type {
  GameProfile,
  CpuTopologyInfo,
  CoreContentionInfo,
  TimerResolutionStatus,
} from "../types/tweak";

export function GameProfilesView() {
  const [profiles, setProfiles] = useState<GameProfile[]>([]);
  const [topology, setTopology] = useState<CpuTopologyInfo | null>(null);
  const [contention, setContention] = useState<CoreContentionInfo | null>(null);
  const [timerRes, setTimerRes] = useState<TimerResolutionStatus | null>(null);
  const [isLoading, setIsLoading] = useState<boolean>(true);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);

  // Estado de edición para perfil personalizado
  const [customExeInput, setCustomExeInput] = useState<string>("");

  const refreshData = useCallback(async () => {
    try {
      const [profs, cont, timer] = await Promise.all([
        ipcGetGameProfiles(),
        ipcGetCoreContention(),
        ipcGetTimerResolution(),
      ]);
      setProfiles(profs);
      setContention(cont);
      setTimerRes(timer);
    } catch (err) {
      console.error("Error actualizando perfiles:", err);
    }
  }, []);

  useEffect(() => {
    let isMounted = true;

    async function initialLoad() {
      setIsLoading(true);
      try {
        const [topo, profs, cont, timer] = await Promise.all([
          ipcGetCpuTopology(),
          ipcGetGameProfiles(),
          ipcGetCoreContention(),
          ipcGetTimerResolution(),
        ]);
        if (isMounted) {
          setTopology(topo);
          setProfiles(profs);
          setContention(cont);
          setTimerRes(timer);

          const customProf = profs.find((p) => p.id === "custom");
          if (customProf && customProf.executables.length > 0) {
            setCustomExeInput(customProf.executables[0]);
          }
        }
      } catch (err) {
        if (isMounted) {
          setErrorMsg(String(err));
        }
      } finally {
        if (isMounted) {
          setIsLoading(false);
        }
      }
    }

    void initialLoad();

    // Polling ligero en UI cada 3 segundos sincronizado con el daemon Rust
    const interval = setInterval(() => {
      void refreshData();
    }, 3000);

    return () => {
      isMounted = false;
      clearInterval(interval);
    };
  }, [refreshData]);

  const handleToggleProfile = async (id: string, currentEnabled: boolean) => {
    try {
      const updated = await ipcToggleGameProfile(id, !currentEnabled);
      setProfiles((prev) => prev.map((p) => (p.id === id ? updated : p)));
    } catch (err) {
      setErrorMsg(String(err));
    }
  };

  const handleTogglePriority = async (prof: GameProfile) => {
    try {
      const updated = await ipcUpdateGameProfile({
        ...prof,
        high_priority: !prof.high_priority,
      });
      setProfiles((prev) => prev.map((p) => (p.id === prof.id ? updated : p)));
    } catch (err) {
      setErrorMsg(String(err));
    }
  };

  const handleToggleTimer = async (prof: GameProfile) => {
    try {
      const updated = await ipcUpdateGameProfile({
        ...prof,
        timer_resolution_05ms: !prof.timer_resolution_05ms,
      });
      setProfiles((prev) => prev.map((p) => (p.id === prof.id ? updated : p)));
    } catch (err) {
      setErrorMsg(String(err));
    }
  };

  const handleSaveCustomExe = async (prof: GameProfile) => {
    if (!customExeInput.trim()) return;
    try {
      const cleanExe = customExeInput.trim().replace(/^.*[\\/]/, "");
      const updated = await ipcUpdateGameProfile({
        ...prof,
        executables: [cleanExe],
      });
      setProfiles((prev) => prev.map((p) => (p.id === prof.id ? updated : p)));
    } catch (err) {
      setErrorMsg(String(err));
    }
  };

  return (
    <div className="view-container">
      {/* Encabezado Principal */}
      <div className="view-header">
        <div className="view-title-group">
          <h2>Perfiles Automáticos por Juego</h2>
          <p>
            Detección segura en segundo plano sin inyección ni lectura de memoria. Aplica
            prioridad y Timer Resolution de 0.5 ms de forma dinámica y los restaura al cerrar el juego.
          </p>
        </div>
      </div>

      {errorMsg && (
        <div className="banner banner-error" role="alert">
          <span>{errorMsg}</span>
          <button type="button" className="banner-close" onClick={() => setErrorMsg(null)}>
            Cerrar
          </button>
        </div>
      )}

      {/* Tarjeta de Topología del Procesador */}
      {topology && (
        <div className="panel topology-panel">
          <div className="panel-header">
            <div className="panel-title-group">
              <h3>Topología del Procesador Detectada</h3>
              <span
                className={`badge ${
                  topology.is_hybrid || topology.is_x3d
                    ? "badge-accent"
                    : "badge-neutral"
                }`}
              >
                {topology.architecture_label}
              </span>
            </div>
            {timerRes && (
              <span
                className={`badge ${
                  timerRes.is_active_by_inputdash ? "badge-applied" : "badge-neutral"
                }`}
              >
                Timer Global: {timerRes.current_resolution_ms.toFixed(2)} ms{" "}
                {timerRes.is_active_by_inputdash && "(InputDash 0.5ms)"}
              </span>
            )}
          </div>

          <div className="topology-info-grid">
            <div className="topology-stat">
              <span className="stat-label">Modelo CPU:</span>
              <span className="stat-value-text">{topology.brand || "Intel/AMD CPU"}</span>
            </div>
            <div className="topology-stat">
              <span className="stat-label">Núcleos Físicos:</span>
              <span className="stat-value-text">{topology.physical_cores} Cores</span>
            </div>
            <div className="topology-stat">
              <span className="stat-label">Hilos Lógicos:</span>
              <span className="stat-value-text">{topology.logical_threads} Threads</span>
            </div>
            {topology.is_hybrid && (
              <div className="topology-stat">
                <span className="stat-label">Distribución:</span>
                <span className="stat-value-text">
                  {topology.p_cores} P-cores (Rendimiento) + {topology.e_cores} E-cores (Eficiencia)
                </span>
              </div>
            )}
          </div>

          <p className="topology-recommendation">
            🛡️ <strong>Regla de Afinidad:</strong> {topology.recommendation}
          </p>
        </div>
      )}

      {/* Alerta de Core Contention */}
      {contention && contention.has_contention && (
        <div className="banner banner-warning contention-banner" role="alert">
          <div className="contention-header">
            <span className="badge badge-warn">⚠️ Core Contention Detectado</span>
            <strong>Procesos en segundo plano compitiendo por ciclos de CPU:</strong>
          </div>
          <ul className="contenders-list">
            {contention.contenders.map((c, i) => (
              <li key={i}>{c}</li>
            ))}
          </ul>
          <p className="contention-note">{contention.recommendation}</p>
        </div>
      )}

      {/* Lista de Perfiles de Juegos */}
      <div className="profiles-grid">
        {isLoading && profiles.length === 0 ? (
          <div className="panel loading-panel">
            <p>Consultando monitor de procesos...</p>
          </div>
        ) : (
          profiles.map((prof) => {
            const isActive = prof.runtime_state === "active";
            const isWaiting = prof.runtime_state === "waiting";
            const isHybridOrX3d = topology?.is_hybrid || topology?.is_x3d;

            return (
              <div
                key={prof.id}
                className={`panel profile-card ${
                  isActive ? "profile-card-active" : ""
                } ${!prof.enabled ? "profile-card-disabled" : ""}`}
              >
                <div className="profile-card-header">
                  <div className="profile-title-area">
                    <div className="profile-icon">
                      {prof.id === "cs2" && "🎯"}
                      {prof.id === "valorant" && "🔥"}
                      {prof.id === "warzone" && "🪖"}
                      {prof.id === "custom" && "⚡"}
                    </div>
                    <div>
                      <h3 className="profile-name">{prof.name}</h3>
                      <span className="profile-executables">
                        {prof.executables.join(" · ")}
                      </span>
                    </div>
                  </div>

                  <div className="profile-actions-header">
                    <button
                      type="button"
                      className={`btn ${
                        prof.enabled ? "btn-secondary" : "btn-primary"
                      } btn-small`}
                      onClick={() => void handleToggleProfile(prof.id, prof.enabled)}
                    >
                      {prof.enabled ? "Desactivar" : "Habilitar"}
                    </button>
                  </div>
                </div>

                {/* Banner de Estado en Vivo */}
                <div
                  className={`profile-status-bar ${
                    isActive
                      ? "status-bar-active"
                      : prof.enabled
                      ? "status-bar-waiting"
                      : "status-bar-disabled"
                  }`}
                >
                  {isActive ? (
                    <div className="status-live-group">
                      <span className="status-dot dot-active"></span>
                      <strong>
                        🎮 PERFIL ACTIVO · {prof.detected_exe} (PID: {prof.detected_pid})
                      </strong>
                    </div>
                  ) : prof.enabled ? (
                    <div className="status-live-group">
                      <span className="status-dot dot-waiting"></span>
                      <span>Esperando a que abras {prof.name}...</span>
                    </div>
                  ) : (
                    <span>Perfil inactivo</span>
                  )}
                </div>

                {prof.last_status_message && (
                  <p className="profile-status-subtext">{prof.last_status_message}</p>
                )}

                {/* Edición de ejecutable para Custom */}
                {prof.id === "custom" && (
                  <div className="custom-exe-form">
                    <label htmlFor="custom-exe-input">Nombre del Ejecutable (.exe):</label>
                    <div className="custom-input-group">
                      <input
                        id="custom-exe-input"
                        type="text"
                        className="text-input"
                        placeholder="ejemplo: apex.exe, overwatch.exe"
                        value={customExeInput}
                        onChange={(e) => setCustomExeInput(e.target.value)}
                      />
                      <button
                        type="button"
                        className="btn btn-secondary btn-small"
                        onClick={() => void handleSaveCustomExe(prof)}
                      >
                        Guardar
                      </button>
                    </div>
                  </div>
                )}

                {/* Configuración de Optimizaciones por Perfil */}
                <div className="profile-settings-list">
                  <div className="profile-setting-item">
                    <label className="checkbox-label">
                      <input
                        type="checkbox"
                        checked={prof.high_priority}
                        disabled={!prof.enabled}
                        onChange={() => void handleTogglePriority(prof)}
                      />
                      <span className="setting-title">Prioridad de Proceso Alta (High)</span>
                    </label>
                    <span className="setting-desc">
                      Garantiza asignación preferente en el programador de Windows sin afectar hilos del kernel.
                    </span>
                  </div>

                  <div className="profile-setting-item">
                    <label className="checkbox-label">
                      <input
                        type="checkbox"
                        checked={prof.timer_resolution_05ms}
                        disabled={!prof.enabled}
                        onChange={() => void handleToggleTimer(prof)}
                      />
                      <span className="setting-title">
                        Timer Resolution 0.5 ms (Guard RAII)
                      </span>
                    </label>
                    <span className="setting-desc">
                      Reduce el intervalo de temporización a 0.5 ms únicamente mientras el juego esté abierto;
                      se restaura automáticamente al cerrarlo.
                    </span>
                  </div>

                  <div className="profile-setting-item">
                    <div className="affinity-lock-group">
                      <span className="setting-title">Afinidad de Núcleos (CPU Affinity):</span>
                      {isHybridOrX3d ? (
                        <span className="badge badge-locked">
                          🔒 Afinidad Manual Bloqueada
                        </span>
                      ) : (
                        <span className="badge badge-applied">
                          ✓ Afinidad Automática (Todos los núcleos)
                        </span>
                      )}
                    </div>
                    <span className="setting-desc">
                      {isHybridOrX3d
                        ? "Tu procesador cuenta con P/E-cores o AMD 3D V-Cache. Bloquear afinidad estática degradaría el reparto dinámico del programador del sistema operativo."
                        : "CPU estándar homogénea. El programador asigna automáticamente los núcleos disponibles."}
                    </span>
                  </div>
                </div>
              </div>
            );
          })
        )}
      </div>
    </div>
  );
}
