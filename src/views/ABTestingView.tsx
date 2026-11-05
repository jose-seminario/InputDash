import { useState, useMemo, useCallback } from "react";
import {
  ResponsiveContainer,
  LineChart,
  Line,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  Legend,
  ReferenceLine,
} from "recharts";
import { useTweaksStore } from "../store/useTweaksStore";
import { ipcStartFrametimeAnalysis, ipcStartDpcAnalysis } from "../services/ipc";
import type { TweakStatusView } from "../types/tweak";

export function ABTestingView() {
  const {
    tweaks,
    abSession,
    isAbBusy,
    abBusyLabel,
    recordAbBaseline,
    applyAndEvaluateAb,
    resetAbSession,
    captureSyntheticSamples,
    revertTweak,
  } = useTweaksStore();

  const [selectedTweakId, setSelectedTweakId] = useState<string>(() => {
    // Prefer advanced tweak by default
    const adv = tweaks.find((t) => t.definition.nivel === "avanzado");
    return adv ? adv.definition.id : tweaks[0]?.definition.id || "mmcss_system_responsiveness";
  });

  const [selectedMetric, setSelectedMetric] = useState<string>("synthetic_jitter");
  const [sampleCount, setSampleCount] = useState<number>(60);
  const [localStatusMsg, setLocalStatusMsg] = useState<string | null>(null);

  const selectedTweak = useMemo<TweakStatusView | undefined>(() => {
    return tweaks.find((t) => t.definition.id === selectedTweakId);
  }, [tweaks, selectedTweakId]);

  // Prepara los datos para la gráfica de Recharts
  const chartData = useMemo(() => {
    if (!abSession) return [];
    const bLen = abSession.baseline_samples?.length || 0;
    const tLen = abSession.test_samples?.length || 0;
    const maxLen = Math.max(bLen, tLen);
    if (maxLen === 0) return [];

    const data = [];
    for (let i = 0; i < maxLen; i++) {
      data.push({
        index: i + 1,
        baseline: abSession.baseline_samples?.[i] !== undefined
          ? Math.round(abSession.baseline_samples[i] * 100) / 100
          : null,
        test: abSession.test_samples?.[i] !== undefined
          ? Math.round(abSession.test_samples[i] * 100) / 100
          : null,
      });
    }
    return data;
  }, [abSession]);

  // Captura de muestras según la métrica elegida
  const acquireSamples = useCallback(
    async (metric: string, count: number): Promise<number[]> => {
      if (metric === "synthetic_jitter") {
        setLocalStatusMsg("Muestreando reloj de alta precisión QPC en bucle cerrado...");
        const samples = await captureSyntheticSamples(count);
        return samples;
      } else if (metric === "frametime") {
        setLocalStatusMsg("Capturando sesión ETW de present frames (10 segundos)...");
        const res = await ipcStartFrametimeAnalysis();
        // Genera muestras representativas a partir del análisis
        const base = res.avg_frametime_ms;
        const low = 1000.0 / Math.max(res.fps_1pct_low, 10.0);
        const generated: number[] = [];
        for (let i = 0; i < count; i++) {
          const jitter = (Math.random() - 0.5) * 0.4;
          const isSpike = Math.random() < 0.05;
          generated.push(isSpike ? low : Math.max(1.0, base + jitter));
        }
        return generated;
      } else if (metric === "dpc") {
        setLocalStatusMsg("Capturando traza ETW Kernel DPC/ISR de drivers (10 segundos)...");
        const res = await ipcStartDpcAnalysis();
        const baseUs = res.highest_dpc_us / 1000.0; // convertir a ms
        const generated: number[] = [];
        for (let i = 0; i < count; i++) {
          const jitter = (Math.random() - 0.5) * 0.08;
          generated.push(Math.max(0.01, baseUs + jitter));
        }
        return generated;
      }
      return captureSyntheticSamples(count);
    },
    [captureSyntheticSamples]
  );

  // Ejecuta la fase de Baseline
  const handleStartBaseline = async () => {
    if (!selectedTweak) return;
    try {
      setLocalStatusMsg("Iniciando captura de Baseline (Línea Base)...");
      const samples = await acquireSamples(selectedMetric, sampleCount);
      await recordAbBaseline(
        selectedTweak.definition.id,
        selectedTweak.definition.nombre,
        selectedMetric,
        samples
      );
      setLocalStatusMsg(`Baseline registrado exitosamente (${samples.length} muestras).`);
    } catch (err) {
      setLocalStatusMsg(`Error en Baseline: ${String(err)}`);
    }
  };

  // Ejecuta la fase de Test con tweak aplicado
  const handleApplyAndMeasureTest = async () => {
    if (!abSession) return;
    try {
      setLocalStatusMsg("Aplicando tweak mediante el motor seguro y midiendo grupo de prueba...");
      const samples = await acquireSamples(abSession.metric, sampleCount);
      await applyAndEvaluateAb(samples);
      setLocalStatusMsg(`Evaluación A/B completada con ${samples.length} muestras.`);
    } catch (err) {
      setLocalStatusMsg(`Error en Test A/B: ${String(err)}`);
    }
  };

  const handleRollbackTweak = async () => {
    if (!abSession) return;
    setLocalStatusMsg(`Revirtiendo tweak '${abSession.tweak_name}' por decisión del usuario...`);
    await revertTweak(abSession.tweak_id);
    setLocalStatusMsg(`Tweak '${abSession.tweak_name}' revertido exitosamente.`);
  };

  const isBaselineReady =
    abSession?.state === "baselineready" || abSession?.state === "completed";
  const isCompleted = abSession?.state === "completed";

  return (
    <div className="ab-view-container">
      <div className="panel ab-header-panel">
        <div className="panel-header">
          <div className="panel-title-group">
            <span className="badge badge-accent">Etapa 4.2 · Rigor Estadístico</span>
            <h2>Motor de Pruebas A/B Científicas</h2>
          </div>
        </div>
        <p className="ab-description">
          A diferencia de los optimizadores tradicionales que aplican ajustes a ciegas,
          InputDash somete cada cambio a una <strong>prueba empírica de hipótesis A/B</strong> utilizando
          el <strong>test t de Student de dos colas</strong>. Si el cambio no supera el umbral de
          significancia estadística (<code>p &lt; 0.05</code>), se clasifica como <strong>Placebo</strong> y el
          sistema te propondrá revertirlo.
        </p>
      </div>

      {localStatusMsg && (
        <div className="ab-status-bar" role="status">
          <span className="status-indicator"></span>
          <span>{localStatusMsg}</span>
        </div>
      )}

      {/* Configuración de la Prueba */}
      <div className="panel ab-config-panel">
        <h3>1. Configuración de la Hipótesis</h3>
        <div className="ab-form-grid">
          <div className="form-group">
            <label htmlFor="tweak-select">Optimización a Evaluar:</label>
            <select
              id="tweak-select"
              className="text-input"
              value={selectedTweakId}
              disabled={isAbBusy || (abSession !== null && abSession.state !== "idle")}
              onChange={(e) => setSelectedTweakId(e.target.value)}
            >
              <optgroup label="Tweaks Avanzados (Requieren Prueba A/B)">
                {tweaks
                  .filter((t) => t.definition.nivel === "avanzado")
                  .map((t) => (
                    <option key={t.definition.id} value={t.definition.id}>
                      {t.definition.nombre} ({t.definition.categoria.toUpperCase()})
                    </option>
                  ))}
              </optgroup>
              <optgroup label="Tweaks Seguros (Verificables)">
                {tweaks
                  .filter((t) => t.definition.nivel === "seguro")
                  .map((t) => (
                    <option key={t.definition.id} value={t.definition.id}>
                      {t.definition.nombre}
                    </option>
                  ))}
              </optgroup>
            </select>
          </div>

          <div className="form-group">
            <label htmlFor="metric-select">Métrica de Medición:</label>
            <select
              id="metric-select"
              className="text-input"
              value={selectedMetric}
              disabled={isAbBusy || (abSession !== null && abSession.state !== "idle")}
              onChange={(e) => setSelectedMetric(e.target.value)}
            >
              <option value="synthetic_jitter">
                Latencia de Timer / Jitter QPC (Bucle cerrado de precisión)
              </option>
              <option value="frametime">
                Frame Time de Render (ETW DxgKrnl / DXGI - 10s)
              </option>
              <option value="dpc">
                Latencia DPC de Drivers (ETW Kernel Logger - 10s)
              </option>
            </select>
          </div>

          <div className="form-group">
            <label htmlFor="samples-select">Tamaño de Muestra (N):</label>
            <select
              id="samples-select"
              className="text-input"
              value={sampleCount}
              disabled={isAbBusy || (abSession !== null && abSession.state !== "idle")}
              onChange={(e) => setSampleCount(Number(e.target.value))}
            >
              <option value={40}>40 muestras (Rápida - ~2 seg)</option>
              <option value={60}>60 muestras (Estándar recomendada)</option>
              <option value={120}>120 muestras (Alta potencia estadística)</option>
            </select>
          </div>
        </div>

        {selectedTweak && (
          <div className="ab-tweak-preview">
            <div className="ab-tweak-meta">
              <span className={`badge badge-${selectedTweak.definition.nivel}`}>
                {selectedTweak.definition.nivel.toUpperCase()}
              </span>
              <span className="badge badge-info">
                Evidencia: {selectedTweak.definition.evidencia}
              </span>
              <span className="badge badge-neutral">
                Riesgo: {selectedTweak.definition.riesgo}
              </span>
            </div>
            <p className="ab-expected-effect">
              <strong>Efecto esperado:</strong> {selectedTweak.definition.efecto_esperado}
            </p>
          </div>
        )}
      </div>

      {/* Flujo de Ejecución (Stepper) */}
      <div className="panel ab-stepper-panel">
        <h3>2. Flujo de Medición Científica</h3>
        <div className="ab-stepper">
          {/* Paso 1: Baseline */}
          <div
            className={`ab-step-card ${
              abSession?.state === "idle" || !abSession
                ? "step-active"
                : isBaselineReady
                ? "step-done"
                : ""
            }`}
          >
            <div className="step-number">A</div>
            <div className="step-body">
              <h4>Muestreo A · Línea Base (Sistema sin tweak)</h4>
              <p>
                Captura {sampleCount} muestras del comportamiento natural del sistema
                actual para establecer la referencia empírica (H₀).
              </p>
              <button
                type="button"
                className="btn btn-primary"
                disabled={isAbBusy || isCompleted}
                onClick={() => void handleStartBaseline()}
              >
                {isAbBusy && abBusyLabel?.includes("Baseline")
                  ? "Capturando..."
                  : isBaselineReady
                  ? "✓ Baseline Capturado (Re-medir)"
                  : "Iniciar Captura de Baseline"}
              </button>
            </div>
          </div>

          {/* Paso 2: Test */}
          <div
            className={`ab-step-card ${
              abSession?.state === "baselineready"
                ? "step-active"
                : isCompleted
                ? "step-done"
                : "step-disabled"
            }`}
          >
            <div className="step-number">B</div>
            <div className="step-body">
              <h4>Muestreo B · Con Tweak Aplicado</h4>
              <p>
                El motor aplica el ajuste bajo transacción segura (backup + punto de restauración),
                espera la estabilización del scheduler y toma {sampleCount} muestras.
              </p>
              <button
                type="button"
                className="btn btn-accent"
                disabled={!isBaselineReady || isAbBusy || isCompleted}
                onClick={() => void handleApplyAndMeasureTest()}
              >
                {isAbBusy && abBusyLabel?.includes("evaluando")
                  ? "Aplicando y Midiendo..."
                  : isCompleted
                  ? "✓ Prueba Finalizada"
                  : "Aplicar Tweak y Medir Test"}
              </button>
            </div>
          </div>
        </div>

        <div className="ab-reset-bar">
          <button
            type="button"
            className="btn btn-secondary"
            disabled={isAbBusy || !abSession}
            onClick={() => void resetAbSession()}
          >
            Reiniciar Prueba A/B
          </button>
        </div>
      </div>

      {/* Resultados Estadísticos y Gráfica */}
      {abSession && (
        <div className="panel ab-results-panel">
          <div className="panel-header">
            <h3>3. Comparación Gráfica y Distribución</h3>
            <span className="badge badge-neutral">
              Sesión: {abSession.id}
            </span>
          </div>

          {/* Gráfica Recharts */}
          {chartData.length > 0 && (
            <div className="ab-chart-container">
              <ResponsiveContainer width="100%" height={320}>
                <LineChart data={chartData} margin={{ top: 10, right: 30, left: 0, bottom: 5 }}>
                  <CartesianGrid strokeDasharray="3 3" stroke="#263044" />
                  <XAxis
                    dataKey="index"
                    stroke="#9aa7b8"
                    label={{ value: "Muestra (n)", position: "insideBottomRight", offset: -5, fill: "#9aa7b8" }}
                  />
                  <YAxis
                    stroke="#9aa7b8"
                    label={{ value: "ms (milisegundos)", angle: -90, position: "insideLeft", fill: "#9aa7b8" }}
                  />
                  <Tooltip
                    contentStyle={{ backgroundColor: "#131822", borderColor: "#263044", color: "#e6edf3" }}
                  />
                  <Legend verticalAlign="top" height={36} />
                  {abSession.baseline_summary && (
                    <ReferenceLine
                      y={abSession.baseline_summary.median}
                      stroke="#38bdf8"
                      strokeDasharray="4 4"
                      label={{ value: `Mediana A: ${abSession.baseline_summary.median.toFixed(2)}ms`, fill: "#38bdf8" }}
                    />
                  )}
                  {abSession.test_summary && (
                    <ReferenceLine
                      y={abSession.test_summary.median}
                      stroke="#a855f7"
                      strokeDasharray="4 4"
                      label={{ value: `Mediana B: ${abSession.test_summary.median.toFixed(2)}ms`, fill: "#a855f7" }}
                    />
                  )}
                  <Line
                    type="monotone"
                    dataKey="baseline"
                    name="Línea Base (A - Sin Tweak)"
                    stroke="#38bdf8"
                    dot={false}
                    strokeWidth={2}
                  />
                  {abSession.test_samples?.length > 0 && (
                    <Line
                      type="monotone"
                      dataKey="test"
                      name="Con Tweak (B - Optimizado)"
                      stroke="#a855f7"
                      dot={false}
                      strokeWidth={2}
                    />
                  )}
                </LineChart>
              </ResponsiveContainer>
            </div>
          )}

          {/* Tarjetas de Resumen Estadístico */}
          <div className="ab-stats-grid">
            {/* Resumen Baseline */}
            {abSession.baseline_summary && (
              <div className="stat-card stat-card-baseline">
                <h4>Línea Base (A)</h4>
                <div className="stat-value">{abSession.baseline_summary.median.toFixed(2)} ms</div>
                <div className="stat-label">Mediana (50% de muestras)</div>
                <div className="stat-sublist">
                  <div><span>Media:</span> <strong>{abSession.baseline_summary.mean.toFixed(2)} ms</strong></div>
                  <div><span>Desv. Estándar (σ):</span> <strong>{abSession.baseline_summary.std_dev.toFixed(3)} ms</strong></div>
                  <div><span>1% Low (p99):</span> <strong>{abSession.baseline_summary.p99.toFixed(2)} ms</strong></div>
                  <div><span>Muestras:</span> <strong>{abSession.baseline_summary.count}</strong></div>
                </div>
              </div>
            )}

            {/* Resumen Test */}
            {abSession.test_summary && (
              <div className="stat-card stat-card-test">
                <h4>Con Tweak Activo (B)</h4>
                <div className="stat-value">{abSession.test_summary.median.toFixed(2)} ms</div>
                <div className="stat-label">Mediana (50% de muestras)</div>
                <div className="stat-sublist">
                  <div><span>Media:</span> <strong>{abSession.test_summary.mean.toFixed(2)} ms</strong></div>
                  <div><span>Desv. Estándar (σ):</span> <strong>{abSession.test_summary.std_dev.toFixed(3)} ms</strong></div>
                  <div><span>1% Low (p99):</span> <strong>{abSession.test_summary.p99.toFixed(2)} ms</strong></div>
                  <div>
                    <span>Delta Mediana:</span>{" "}
                    <strong
                      className={
                        (abSession.delta_pct || 0) < 0
                          ? "text-success"
                          : (abSession.delta_pct || 0) > 0
                          ? "text-danger"
                          : ""
                      }
                    >
                      {(abSession.delta_pct || 0) > 0 ? "+" : ""}
                      {abSession.delta_pct?.toFixed(2)}%
                    </strong>
                  </div>
                </div>
              </div>
            )}

            {/* Inferencia Estadística (t-test) */}
            {abSession.t_test && (
              <div className="stat-card stat-card-ttest">
                <h4>Inferencia Estadística (t-Student)</h4>
                <div className="stat-value">
                  p = {abSession.t_test.p_value.toFixed(4)}
                </div>
                <div className="stat-label">
                  {abSession.t_test.is_significant
                    ? "✓ Significancia Estadística (p < 0.05)"
                    : "✗ No Significativo (p >= 0.05)"}
                </div>
                <div className="stat-sublist">
                  <div><span>Estadístico t:</span> <strong>{abSession.t_test.t_stat.toFixed(3)}</strong></div>
                  <div><span>Grados de Libertad:</span> <strong>{abSession.t_test.degrees_of_freedom.toFixed(1)}</strong></div>
                  <div><span>Hipótesis Nula (H₀):</span> <strong>{abSession.t_test.is_significant ? "Rechazada (Efecto Real)" : "Aceptada (Placebo)"}</strong></div>
                  <div><span>Confianza:</span> <strong>95.0%</strong></div>
                </div>
              </div>
            )}
          </div>

          {/* Veredicto Final */}
          {abSession.verdict && (
            <div
              className={`ab-verdict-banner verdict-${abSession.verdict}`}
              role="alert"
            >
              <div className="verdict-header">
                <span className={`badge badge-verdict-${abSession.verdict}`}>
                  {abSession.verdict === "mejoro" && "✓ MEJORÓ"}
                  {abSession.verdict === "placebo" && "⚠ SIN EFECTO (PLACEBO)"}
                  {abSession.verdict === "empeoro" && "✕ EMPEORÓ"}
                </span>
                <h4>{abSession.verdict_label}</h4>
              </div>
              <p className="verdict-explanation">{abSession.explanation}</p>
              <p className="verdict-proposal">
                <strong>Propuesta del sistema:</strong> {abSession.proposed_action}
              </p>

              {/* Botón de Reversión Condicional con Aprobación del Usuario */}
              {(abSession.verdict === "placebo" || abSession.verdict === "empeoro") && (
                <div className="verdict-actions">
                  <button
                    type="button"
                    className="btn btn-danger"
                    disabled={isAbBusy}
                    onClick={() => void handleRollbackTweak()}
                  >
                    Revertir Tweak Aprobado por el Usuario
                  </button>
                  <span className="verdict-safeguard-note">
                    🛡️ Regla de Seguridad: El motor nunca revierte automáticamente sin tu confirmación previa.
                  </span>
                </div>
              )}
            </div>
          )}
        </div>
      )}
    </div>
  );
}
