import { SimulationPreviewPanel } from "../components/SimulationPreviewPanel";
import { useTweaksStore } from "../store/useTweaksStore";

export function OptimizeView() {
  const {
    tweaks,
    busyAction,
    simulationMode,
    simulationResults,
    simulationLabel,
    clearSimulation,
    applyTweak,
    applyPreset,
    revertTweak,
  } = useTweaksStore();

  const safeTweaks = tweaks.filter((t) => t.definition.nivel === "seguro");
  const advancedTweaks = tweaks.filter((t) => t.definition.nivel === "avanzado");
  const anyBusy = busyAction !== null;

  return (
    <div>
      <section className="presets-grid">
        <div className="preset-card">
          <div className="preset-header">
            <h3 className="tweak-title">Preset Competitivo</h3>
            <span className="badge badge-applied">Recomendado Shooters</span>
          </div>
          <p className="tweak-desc">
            Activa plan <strong>Ultimate Performance</strong> (vía PowrProf),
            desactiva Suspensión Selectiva USB, desactiva Power Throttling,
            activa Game Mode y apaga Game DVR en segundo plano.
          </p>
          <button
            type="button"
            className="btn btn-primary"
            disabled={anyBusy}
            onClick={() => void applyPreset("competitivo")}
          >
            {busyAction === "preset:competitivo" || busyAction === "sim_competitivo"
              ? "Procesando..."
              : simulationMode
              ? "Simular Preset Competitivo"
              : "Aplicar Preset Competitivo"}
          </button>
        </div>

        <div className="preset-card">
          <div className="preset-header">
            <h3 className="tweak-title">Preset Equilibrado</h3>
            <span className="badge badge-default">Uso Diario / Laptop</span>
          </div>
          <p className="tweak-desc">
            Activa plan <strong>High Performance</strong>, desactiva Suspensión
            Selectiva USB, activa Game Mode y apaga Game DVR manteniendo gestión
            térmica moderada del procesador.
          </p>
          <button
            type="button"
            className="btn btn-secondary"
            disabled={anyBusy}
            onClick={() => void applyPreset("equilibrado")}
          >
            {busyAction === "preset:equilibrado" || busyAction === "sim_equilibrado"
              ? "Procesando..."
              : simulationMode
              ? "Simular Preset Equilibrado"
              : "Aplicar Preset Equilibrado"}
          </button>
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
        Optimizaciones Seguras ({safeTweaks.length}) — Backup Automático y Reversión Exacta
      </h2>

      <div className="tweaks-list">
        {safeTweaks.map((item) => {
          const { definition } = item;
          const isThisBusy = busyAction === definition.id;

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
                    {definition.categoria} · evidencia {definition.evidencia} · riesgo{" "}
                    {definition.riesgo}
                  </span>
                  {definition.requiere_admin && (
                    <span className="badge badge-warn">Admin</span>
                  )}
                  {definition.requiere_reinicio && (
                    <span className="badge badge-default">Reinicio</span>
                  )}
                </div>

                <p className="tweak-desc">{definition.efecto_esperado}</p>

                <div className="tweak-meta-grid">
                  <div>
                    <span className="meta-label">Objetivo:</span>
                    <span className="meta-mono">{item.target_summary}</span>
                  </div>
                  <div>
                    <span className="meta-label">Valor actual OS:</span>
                    <span className="meta-mono">{item.current_os_value}</span>
                  </div>
                  <div>
                    <span className="meta-label">Valor optimizado:</span>
                    <span className="meta-mono">{item.expected_value_display}</span>
                  </div>
                  <div>
                    <span className="meta-label">Snapshot reversión:</span>
                    <span className="meta-mono">
                      {item.has_backup
                        ? item.backup_original_value ?? "Snapshot activo"
                        : "Sin snapshot activo"}
                    </span>
                  </div>
                </div>

                {item.status_error && (
                  <div className="inline-error">
                    Aviso de lectura: {item.status_error}
                  </div>
                )}
              </div>

              <div className="tweak-actions">
                <button
                  type="button"
                  className="btn btn-primary"
                  disabled={anyBusy || (!simulationMode && item.is_applied)}
                  onClick={() => void applyTweak(definition.id)}
                >
                  {isThisBusy
                    ? "Procesando..."
                    : simulationMode
                    ? "Simular"
                    : "Aplicar"}
                </button>
                <button
                  type="button"
                  className="btn btn-secondary"
                  disabled={anyBusy || simulationMode || !item.has_backup}
                  onClick={() => void revertTweak(definition.id)}
                >
                  Revertir
                </button>
              </div>
            </article>
          );
        })}
      </div>

      <h2 className="section-heading">
        Ajustes Avanzados ({advancedTweaks.length}) — Candado Científico A/B Obligatorio
      </h2>
      <p className="section-subdesc">
        Siguiendo la arquitectura de InputDash, ningún ajuste avanzado se aplica
        a ciegas: exigen sesión comparativa Baseline vs Test con veredicto
        estadístico (Etapa 5).
      </p>

      <div className="tweaks-list">
        {advancedTweaks.map((item) => {
          const { definition } = item;

          return (
            <article key={definition.id} className="tweak-card tweak-card-muted">
              <div className="tweak-info">
                <div className="tweak-title-row">
                  <h3 className="tweak-title">{definition.nombre}</h3>
                  <span className="badge badge-warn">Avanzado · Opt-In</span>
                  <span className="badge badge-default">
                    {definition.categoria} · riesgo {definition.riesgo}
                  </span>
                  {definition.juego_protocolo === "tcp" && (
                    <span className="badge badge-default">Solo Juegos TCP</span>
                  )}
                  {definition.requiere_reinicio && (
                    <span className="badge badge-default">Requiere Reinicio</span>
                  )}
                </div>

                <p className="tweak-desc">{definition.efecto_esperado}</p>

                <div className="tweak-meta-grid">
                  <div>
                    <span className="meta-label">Objetivo:</span>
                    <span className="meta-mono">{item.target_summary}</span>
                  </div>
                  <div>
                    <span className="meta-label">Estado actual OS:</span>
                    <span className="meta-mono">{item.current_os_value}</span>
                  </div>
                  <div>
                    <span className="meta-label">Propuesta A/B:</span>
                    <span className="meta-mono">{item.expected_value_display}</span>
                  </div>
                </div>
              </div>

              <div className="tweak-actions tweak-actions-wide">
                <button type="button" className="btn btn-secondary" disabled>
                  Requiere A/B (Próximamente)
                </button>
              </div>
            </article>
          );
        })}
      </div>
    </div>
  );
}
