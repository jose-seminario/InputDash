import { useEffect } from "react";
import { useTweaksStore } from "./store/useTweaksStore";

export function App() {
  const {
    tweaks,
    logs,
    isLoading,
    busyTweakId,
    errorBanner,
    warningMessage,
    successMessage,
    refreshAll,
    applyTweak,
    revertTweak,
    dismissMessages,
  } = useTweaksStore();

  useEffect(() => {
    void refreshAll();
  }, [refreshAll]);

  return (
    <div className="app-container">
      <header className="app-header">
        <div className="app-title-group">
          <h1>InputDash</h1>
          <p>
            Motor Transaccional de Optimización Nativa (Guard → Backup → Restore
            Point → Apply → Verify → Log)
          </p>
        </div>
        <button
          type="button"
          className="btn btn-secondary"
          disabled={isLoading || busyTweakId !== null}
          onClick={() => void refreshAll()}
        >
          {isLoading ? "Leyendo..." : "Actualizar estado"}
        </button>
      </header>

      {errorBanner && (
        <div className="banner banner-error" role="alert">
          <div>
            <span className="banner-code">[{errorBanner.code}]</span>
            <span>{errorBanner.message}</span>
          </div>
          <button
            type="button"
            className="banner-close"
            onClick={dismissMessages}
          >
            Cerrar
          </button>
        </div>
      )}

      {warningMessage && (
        <div className="banner banner-warning" role="alert">
          <span>{warningMessage}</span>
          <button
            type="button"
            className="banner-close"
            onClick={dismissMessages}
          >
            Cerrar
          </button>
        </div>
      )}

      {successMessage && (
        <div className="banner banner-success" role="status">
          <span>{successMessage}</span>
          <button
            type="button"
            className="banner-close"
            onClick={dismissMessages}
          >
            Cerrar
          </button>
        </div>
      )}

      <h2 className="section-heading">Tweaks Verificados (Aplicar / Revertir)</h2>

      <div className="tweaks-list">
        {tweaks.length === 0 && !isLoading && (
          <div className="logs-panel empty-state">
            No se cargaron tweaks del catálogo.
          </div>
        )}

        {tweaks.map((item) => {
          const { definition } = item;
          const isBusy = busyTweakId === definition.id;
          const fullRegistryPath = `${definition.target.hive}\\${definition.target.ruta}\\${definition.target.valor}`;

          return (
            <article key={definition.id} className="tweak-card">
              <div className="tweak-info">
                <div className="tweak-title-row">
                  <h3 className="tweak-title">{definition.nombre}</h3>
                  <span
                    className={`badge ${
                      item.is_applied ? "badge-applied" : "badge-default"
                    }`}
                  >
                    {item.is_applied ? "Aplicado" : "No aplicado"}
                  </span>
                  <span className="badge badge-default">
                    {definition.categoria} · {definition.nivel}
                  </span>
                  {definition.requiere_admin && (
                    <span className="badge badge-admin">Requiere Admin</span>
                  )}
                </div>

                <p className="tweak-desc">{definition.efecto_esperado}</p>

                <div className="tweak-meta-grid">
                  <div>
                    <span className="meta-label">Clave:</span>
                    <span className="meta-mono">{fullRegistryPath}</span>
                  </div>
                  <div>
                    <span className="meta-label">Valor actual OS:</span>
                    <span className="meta-mono">{item.current_os_value}</span>
                  </div>
                  <div>
                    <span className="meta-label">Objetivo tweak:</span>
                    <span className="meta-mono">
                      {String(definition.target.dato)} (
                      {definition.target.tipo_dato})
                    </span>
                  </div>
                  <div>
                    <span className="meta-label">Backup previo:</span>
                    <span className="meta-mono">
                      {item.has_backup
                        ? item.backup_original_value ?? "Disponible"
                        : "Sin snapshot activo"}
                    </span>
                  </div>
                </div>

                {item.status_error && (
                  <div className="inline-error">
                    Error de lectura: {item.status_error}
                  </div>
                )}
              </div>

              <div className="tweak-actions">
                <button
                  type="button"
                  className="btn btn-primary"
                  disabled={isBusy || busyTweakId !== null || item.is_applied}
                  onClick={() => void applyTweak(definition.id)}
                >
                  {isBusy ? "Procesando..." : "Aplicar"}
                </button>
                <button
                  type="button"
                  className="btn btn-secondary"
                  disabled={isBusy || busyTweakId !== null || !item.has_backup}
                  onClick={() => void revertTweak(definition.id)}
                >
                  Revertir
                </button>
              </div>
            </article>
          );
        })}
      </div>

      <h2 className="section-heading">Registro Transaccional de Cambios (SQLite)</h2>

      <div className="logs-panel">
        {logs.length === 0 ? (
          <div className="empty-state">
            Aún no se han registrado modificaciones en el sistema.
          </div>
        ) : (
          <table className="logs-table">
            <thead>
              <tr>
                <th>Timestamp (UTC)</th>
                <th>Acción</th>
                <th>Tweak</th>
                <th>Valor Previo → Nuevo</th>
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
