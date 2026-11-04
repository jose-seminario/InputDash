import { SimulationPreviewPanel } from "../components/SimulationPreviewPanel";
import { useTweaksStore } from "../store/useTweaksStore";

export function SafetyView() {
  const {
    backups,
    logs,
    busyAction,
    simulationMode,
    simulationResults,
    simulationLabel,
    setSimulationMode,
    clearSimulation,
    simulateAllSafe,
    simulatePreset,
    createManualRestorePoint,
    revertTweak,
    revertAll,
  } = useTweaksStore();

  const anyBusy = busyAction !== null;

  return (
    <div>
      <section className="safety-grid">
        <div className="preset-card">
          <div className="preset-header">
            <h3 className="tweak-title">Modo Simulación (Dry-Run)</h3>
            <span
              className={`badge ${
                simulationMode ? "badge-warn" : "badge-default"
              }`}
            >
              {simulationMode ? "Simulación Activa" : "Ejecución Real"}
            </span>
          </div>
          <p className="tweak-desc">
            Evalúa en solo lectura los valores reales actuales de tu Registro y
            PowrProf mostrando exactamente qué cambiaría y cómo se revertiría,
            sin escribir un solo byte en Windows.
          </p>
          <div className="inline-controls">
            <label className="checkbox-label">
              <input
                type="checkbox"
                checked={simulationMode}
                onChange={(e) => setSimulationMode(e.target.checked)}
              />
              <span>Activar Modo Simulación global al usar Optimizar</span>
            </label>
          </div>
          <div className="button-row">
            <button
              type="button"
              className="btn btn-secondary"
              disabled={anyBusy}
              onClick={() => void simulateAllSafe()}
            >
              {busyAction === "sim_safe"
                ? "Calculando..."
                : "Simular los 6 Seguros"}
            </button>
            <button
              type="button"
              className="btn btn-secondary"
              disabled={anyBusy}
              onClick={() => void simulatePreset("competitivo")}
            >
              Simular Competitivo
            </button>
            <button
              type="button"
              className="btn btn-secondary"
              disabled={anyBusy}
              onClick={() => void simulatePreset("equilibrado")}
            >
              Simular Equilibrado
            </button>
          </div>
        </div>

        <div className="preset-card">
          <div className="preset-header">
            <h3 className="tweak-title">Recuperación y Reversión Total</h3>
            <span className="badge badge-default">
              {backups.length} backup(s) activo(s)
            </span>
          </div>
          <p className="tweak-desc">
            Crea un Punto de Restauración de Windows vía <code>srclient.dll</code>{" "}
            o revierte todos los cambios aplicados iterando sobre el historial
            LIFO de SQLite y verificando cada snapshot JSON atómico.
          </p>
          <div className="button-row">
            <button
              type="button"
              className="btn btn-secondary"
              disabled={anyBusy}
              onClick={() => void createManualRestorePoint()}
            >
              {busyAction === "restore_point"
                ? "Creando punto..."
                : "Crear Punto de Restauración"}
            </button>
            <button
              type="button"
              className="btn btn-danger"
              disabled={anyBusy || backups.length === 0}
              onClick={() => void revertAll()}
            >
              {busyAction === "revert_all"
                ? "Revirtiendo todo..."
                : `Revertir Todo (${backups.length})`}
            </button>
          </div>
        </div>
      </section>

      {simulationResults && (
        <SimulationPreviewPanel
          label={simulationLabel}
          items={simulationResults}
          onClose={clearSimulation}
        />
      )}

      <h2 className="section-heading">
        Snapshots JSON Atómicos Activos ({backups.length})
      </h2>

      <div className="logs-panel">
        {backups.length === 0 ? (
          <div className="empty-state">
            No hay snapshots pendientes: el sistema se encuentra en su estado
            original sin modificaciones de InputDash.
          </div>
        ) : (
          <table className="logs-table">
            <thead>
              <tr>
                <th>Creado (UTC)</th>
                <th>Tweak</th>
                <th>ID</th>
                <th>Acción</th>
              </tr>
            </thead>
            <tbody>
              {backups.map((snap) => (
                <tr key={snap.tweak_id}>
                  <td className="meta-mono">{snap.created_at_utc}</td>
                  <td>{snap.tweak_name}</td>
                  <td className="meta-mono">{snap.tweak_id}</td>
                  <td>
                    <button
                      type="button"
                      className="btn btn-secondary btn-small"
                      disabled={anyBusy}
                      onClick={() => void revertTweak(snap.tweak_id)}
                    >
                      Revertir este ajuste
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>

      <h2 className="section-heading">
        Historial Transaccional de Cambios (SQLite — {logs.length} eventos)
      </h2>

      <div className="logs-panel">
        {logs.length === 0 ? (
          <div className="empty-state">
            Aún no se han registrado operaciones en la base de datos local.
          </div>
        ) : (
          <table className="logs-table">
            <thead>
              <tr>
                <th>Timestamp (UTC)</th>
                <th>Acción</th>
                <th>Ajuste</th>
                <th>Estado Previo → Nuevo</th>
                <th>Verificado</th>
                <th>Detalle</th>
              </tr>
            </thead>
            <tbody>
              {logs.map((entry) => (
                <tr key={entry.id}>
                  <td className="meta-mono">{entry.timestamp_utc}</td>
                  <td className="meta-mono">{entry.action}</td>
                  <td>{entry.tweak_name}</td>
                  <td className="meta-mono">
                    {entry.previous_value} → {entry.new_value}
                  </td>
                  <td>{entry.verified ? "Sí" : "No"}</td>
                  <td>{entry.details}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>
    </div>
  );
}
