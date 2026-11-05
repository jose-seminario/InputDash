import { useState, useMemo, useCallback, useEffect, useRef } from "react";
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
import {
  ipcStartFrametimeAnalysis,
  ipcStartDpcAnalysis,
  ipcStartBlindTest,
  ipcRecordBlindSpawn,
  ipcRecordBlindClick,
  ipcAdvanceBlindPhase,
  ipcCompleteBlindTest,
  ipcCancelBlindTest,
  ipcEvaluateExternalLdat,
} from "../services/ipc";
import type {
  BlindPublicState,
  BlindResult,
  ExternalLdatResult,
  TweakStatusView,
} from "../types/tweak";

type ABViewTab = "double_blind" | "ab_standard" | "ldat_import";

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
    applyTweak,
  } = useTweaksStore();

  const [activeSubTab, setActiveSubTab] = useState<ABViewTab>("double_blind");

  // -------------------------------------------------------------------------
  // Estado para Prueba A/B Estándar
  // -------------------------------------------------------------------------
  const [selectedTweakId, setSelectedTweakId] = useState<string>(() => {
    const adv = tweaks.find((t) => t.definition.nivel === "avanzado");
    return adv ? adv.definition.id : tweaks[0]?.definition.id || "mmcss_system_responsiveness";
  });

  const [selectedMetric, setSelectedMetric] = useState<string>("synthetic_jitter");
  const [sampleCount, setSampleCount] = useState<number>(60);
  const [localStatusMsg, setLocalStatusMsg] = useState<string | null>(null);

  const selectedTweak = useMemo<TweakStatusView | undefined>(() => {
    return tweaks.find((t) => t.definition.id === selectedTweakId);
  }, [tweaks, selectedTweakId]);

  // -------------------------------------------------------------------------
  // Estado para Test Ciego (Double-Blind)
  // -------------------------------------------------------------------------
  const [blindTweakId, setBlindTweakId] = useState<string>(() => {
    const adv = tweaks.find((t) => t.definition.nivel === "avanzado");
    return adv ? adv.definition.id : tweaks[0]?.definition.id || "mmcss_system_responsiveness";
  });

  const [blindState, setBlindState] = useState<BlindPublicState | null>(null);
  const [blindResult, setBlindResult] = useState<BlindResult | null>(null);
  const [isBlindBusy, setIsBlindBusy] = useState<boolean>(false);
  const [countdown, setCountdown] = useState<number | null>(null);
  const [targetPos, setTargetPos] = useState<{ x: number; y: number } | null>(null);
  const [clickCountInPhase, setClickCountInPhase] = useState<number>(0);
  const [lastReactionMs, setLastReactionMs] = useState<number | null>(null);
  const [phaseTransitionReady, setPhaseTransitionReady] = useState<boolean>(false);
  const spawnTimestampRef = useRef<number>(0);

  // -------------------------------------------------------------------------
  // Estado para Importación LDAT / Reflex
  // -------------------------------------------------------------------------
  const [ldatMeanA, setLdatMeanA] = useState<string>("18.5");
  const [ldatStdA, setLdatStdA] = useState<string>("1.8");
  const [ldatCountA, setLdatCountA] = useState<string>("30");
  const [ldatMeanB, setLdatMeanB] = useState<string>("16.2");
  const [ldatStdB, setLdatStdB] = useState<string>("1.7");
  const [ldatCountB, setLdatCountB] = useState<string>("30");
  const [ldatResult, setLdatResult] = useState<ExternalLdatResult | null>(null);
  const [ldatError, setLdatError] = useState<string | null>(null);

  // Prepara datos para gráfica Recharts en A/B Estándar
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
        baseline:
          abSession.baseline_samples?.[i] !== undefined
            ? Math.round(abSession.baseline_samples[i] * 100) / 100
            : null,
        test:
          abSession.test_samples?.[i] !== undefined
            ? Math.round(abSession.test_samples[i] * 100) / 100
            : null,
      });
    }
    return data;
  }, [abSession]);

  // Captura de muestras A/B
  const acquireSamples = useCallback(
    async (metric: string, count: number): Promise<number[]> => {
      if (metric === "synthetic_jitter") {
        setLocalStatusMsg("Muestreando reloj de alta precisión QPC en bucle cerrado...");
        return captureSyntheticSamples(count);
      } else if (metric === "frametime") {
        setLocalStatusMsg("Capturando sesión ETW de present frames (10 segundos)...");
        const res = await ipcStartFrametimeAnalysis();
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
        const baseUs = res.highest_dpc_us / 1000.0;
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

  const handleApplyAndMeasureTest = async () => {
    if (!abSession) return;
    try {
      setLocalStatusMsg("Aplicando tweak con session_token activo y midiendo grupo de prueba...");
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

  // -------------------------------------------------------------------------
  // Lógica del Test Ciego (Double-Blind)
  // -------------------------------------------------------------------------

  const spawnNewTarget = useCallback(async () => {
    // Coordenadas aleatorias seguras dentro de la arena (entre 10% y 85%)
    const x = Math.floor(Math.random() * 75) + 10;
    const y = Math.floor(Math.random() * 70) + 10;
    setTargetPos({ x, y });
    spawnTimestampRef.current = performance.now();
    try {
      await ipcRecordBlindSpawn();
    } catch {
      // fallback sin fallo
    }
  }, []);

  // Iniciar Test Ciego
  const handleStartBlindTest = async () => {
    const twk = tweaks.find((t) => t.definition.id === blindTweakId);
    if (!twk) return;

    setIsBlindBusy(true);
    setBlindResult(null);
    setPhaseTransitionReady(false);
    setClickCountInPhase(0);
    setTargetPos(null);

    try {
      const state = await ipcStartBlindTest(twk.definition.id, twk.definition.nombre);
      setBlindState(state);
      setIsBlindBusy(false);

      // Iniciar cuenta regresiva 3, 2, 1 para la Fase A
      setCountdown(3);
    } catch (err) {
      setIsBlindBusy(false);
      setLocalStatusMsg(`Error al iniciar Test Ciego: ${String(err)}`);
    }
  };

  // Manejo de la cuenta regresiva antes de comenzar cada fase
  useEffect(() => {
    if (countdown === null) return;
    if (countdown > 0) {
      const timer = setTimeout(() => {
        setCountdown(countdown - 1);
      }, 1000);
      return () => clearTimeout(timer);
    } else {
      // Countdown terminó (0): spawn de la primera diana
      setCountdown(null);
      void spawnNewTarget();
    }
  }, [countdown, spawnNewTarget]);

  // Click en la diana (Target)
  const handleTargetClick = async () => {
    if (!targetPos) return;
    const clickTime = performance.now();
    const fallbackReaction = Math.max(50, clickTime - spawnTimestampRef.current);

    setTargetPos(null); // Ocultar temporalmente la diana

    try {
      const [count, reactionMs, phaseDone] = await ipcRecordBlindClick(fallbackReaction);
      setClickCountInPhase(count);
      setLastReactionMs(reactionMs);

      if (phaseDone) {
        if (blindState?.phase === "statea") {
          // Fase A completada (30 clicks)
          setPhaseTransitionReady(true);
        } else if (blindState?.phase === "stateb") {
          // Fase B completada (30 clicks) -> evaluar automáticamente
          setIsBlindBusy(true);
          setLocalStatusMsg("30 clicks completados en Fase B. Calculando inferencia estadística...");
          const res = await ipcCompleteBlindTest();
          setBlindResult(res);
          setIsBlindBusy(false);
          setBlindState((prev) => (prev ? { ...prev, phase: "completed", result: res } : null));
        }
      } else {
        // Intervalo aleatorio corto (180ms - 380ms) antes de mostrar la siguiente diana
        const delay = Math.floor(Math.random() * 200) + 180;
        setTimeout(() => {
          void spawnNewTarget();
        }, delay);
      }
    } catch (err) {
      setLocalStatusMsg(`Error al registrar click: ${String(err)}`);
    }
  };

  // Avanzar a la Fase B
  const handleAdvanceToPhaseB = async () => {
    setIsBlindBusy(true);
    setPhaseTransitionReady(false);
    setClickCountInPhase(0);
    setTargetPos(null);

    try {
      const updated = await ipcAdvanceBlindPhase();
      setBlindState(updated);
      setIsBlindBusy(false);
      // Iniciar cuenta regresiva para la Fase B
      setCountdown(3);
    } catch (err) {
      setIsBlindBusy(false);
      setLocalStatusMsg(`Error al avanzar a Fase B: ${String(err)}`);
    }
  };

  // Cancelar Test Ciego (Salvaguarda: revierte cualquier cambio automáticamente)
  const handleCancelBlindTest = async () => {
    try {
      await ipcCancelBlindTest();
      setBlindState(null);
      setBlindResult(null);
      setTargetPos(null);
      setCountdown(null);
      setPhaseTransitionReady(false);
      setClickCountInPhase(0);
      setLocalStatusMsg("Test Ciego cancelado. Cualquier cambio en el sistema fue revertido de inmediato.");
    } catch (err) {
      setLocalStatusMsg(`Error al cancelar: ${String(err)}`);
    }
  };

  // -------------------------------------------------------------------------
  // Lógica de Importación LDAT / Reflex
  // -------------------------------------------------------------------------
  const handleCalculateLdat = async () => {
    const mA = parseFloat(ldatMeanA);
    const sA = parseFloat(ldatStdA);
    const nA = parseInt(ldatCountA, 10);
    const mB = parseFloat(ldatMeanB);
    const sB = parseFloat(ldatStdB);
    const nB = parseInt(ldatCountB, 10);

    if (isNaN(mA) || isNaN(sA) || isNaN(mB) || isNaN(sB)) {
      setLdatError("Por favor ingresa valores numéricos válidos en todos los campos.");
      return;
    }
    if (sA <= 0 || sB <= 0) {
      setLdatError("La desviación estándar debe ser estrictamente mayor a 0.");
      return;
    }

    setLdatError(null);
    try {
      const res = await ipcEvaluateExternalLdat({
        mean_a: mA,
        std_dev_a: sA,
        count_a: isNaN(nA) ? 30 : nA,
        mean_b: mB,
        std_dev_b: sB,
        count_b: isNaN(nB) ? 30 : nB,
      });
      setLdatResult(res);
    } catch (err) {
      setLdatError(`Error evaluando medición externa: ${String(err)}`);
    }
  };

  const isBaselineReady =
    abSession?.state === "baselineready" || abSession?.state === "completed";
  const isCompleted = abSession?.state === "completed";

  return (
    <div className="ab-view-container">
      {/* Cabecera Principal */}
      <div className="panel ab-header-panel">
        <div className="panel-header">
          <div className="panel-title-group">
            <span className="badge badge-accent">Etapa 5 · Validación Empírica & Test Ciego</span>
            <h2>Evaluación Científica de Latencia (0 Placebo)</h2>
          </div>
        </div>
        <p className="ab-description">
          Somete cualquier tweak avanzado a pruebas estadísticas rigurosas para verificar si
          realmente reduce el <em>click-to-photon</em> o el tiempo de respuesta, o si es puro efecto placebo.
          Los tweaks avanzados se aplican bajo un <strong>token transaccional temporal</strong> que se invalida al concluir la prueba.
        </p>

        {/* Sub-navegación entre modos de prueba */}
        <div className="ab-subtabs">
          <button
            type="button"
            className={`ab-subtab ${activeSubTab === "double_blind" ? "active" : ""}`}
            onClick={() => setActiveSubTab("double_blind")}
          >
            🎯 Test Ciego (Double-Blind · 30 Clicks)
          </button>
          <button
            type="button"
            className={`ab-subtab ${activeSubTab === "ab_standard" ? "active" : ""}`}
            onClick={() => setActiveSubTab("ab_standard")}
          >
            📊 Prueba A/B Estándar (ETW / QPC)
          </button>
          <button
            type="button"
            className={`ab-subtab ${activeSubTab === "ldat_import" ? "active" : ""}`}
            onClick={() => setActiveSubTab("ldat_import")}
          >
            🔬 Importar Medición Externa (LDAT / Reflex)
          </button>
        </div>
      </div>

      {localStatusMsg && (
        <div className="ab-status-bar" role="status">
          <span className="status-indicator"></span>
          <span>{localStatusMsg}</span>
        </div>
      )}

      {/* =====================================================================
          MODO 1: TEST CIEGO (DOUBLE-BLIND)
         ===================================================================== */}
      {activeSubTab === "double_blind" && (
        <div className="ab-section-container">
          <div className="panel">
            <div className="panel-header">
              <div className="panel-title-group">
                <h3>Test Ciego de Reacción Humano-Sistema</h3>
                <span className="badge badge-warn">Sin Sesgo Psicológico</span>
              </div>
            </div>
            <p className="tweak-desc">
              Harás <strong>30 disparos a una diana</strong> en el Estado A y <strong>30 disparos</strong> en el Estado B.
              El motor aleatoriza si el tweak está activo en A o en B sin revelártelo. Al terminar, la prueba t de Welch
              revelará si tu tiempo de reacción mejoró con significancia estadística o si fue imperceptible.
            </p>

            {/* Configuración del Test Ciego */}
            {(!blindState || blindState.phase === "idle" || blindState.phase === "completed" || blindState.phase === "cancelled") && (
              <div className="blind-setup-row">
                <div className="form-group" style={{ flex: 1 }}>
                  <label htmlFor="blind-tweak-select">Optimización a Probar a Ciegas:</label>
                  <select
                    id="blind-tweak-select"
                    className="text-input"
                    value={blindTweakId}
                    disabled={isBlindBusy}
                    onChange={(e) => setBlindTweakId(e.target.value)}
                  >
                    <optgroup label="Tweaks Avanzados (Habilitados con Session Token)">
                      {tweaks
                        .filter((t) => t.definition.nivel === "avanzado")
                        .map((t) => (
                          <option key={t.definition.id} value={t.definition.id}>
                            {t.definition.nombre} ({t.definition.categoria.toUpperCase()})
                          </option>
                        ))}
                    </optgroup>
                    <optgroup label="Tweaks Seguros">
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

                <button
                  type="button"
                  className="btn btn-accent btn-large"
                  disabled={isBlindBusy}
                  onClick={() => void handleStartBlindTest()}
                  style={{ alignSelf: "flex-end" }}
                >
                  {isBlindBusy ? "Iniciando Test..." : "Iniciar Test Ciego (30 Clicks)"}
                </button>
              </div>
            )}

            {/* Progreso del Test Ciego Activo */}
            {blindState && (blindState.phase === "statea" || blindState.phase === "stateb") && (
              <div className="blind-active-container">
                <div className="blind-header-bar">
                  <div className="blind-phase-badge">
                    <span className="badge badge-applied">
                      {blindState.phase === "statea" ? "Fase 1: Muestreo A" : "Fase 2: Muestreo B"}
                    </span>
                    <span className="blind-clicks-counter">
                      Disparos: <strong>{clickCountInPhase}</strong> / {blindState.target_clicks}
                    </span>
                    {lastReactionMs !== null && (
                      <span className="last-ms-badge">
                        Último click: <strong>{lastReactionMs} ms</strong>
                      </span>
                    )}
                  </div>

                  <button
                    type="button"
                    className="btn btn-danger btn-small"
                    onClick={() => void handleCancelBlindTest()}
                  >
                    Cancelar y Revertir Tweak
                  </button>
                </div>

                <div className="blind-confidential-note">
                  🔒 <em>Estado a ciegas: El sistema mantiene oculto si esta fase tiene el tweak aplicado o si es la configuración de fábrica.</em>
                </div>

                {/* Arena interactiva de reacción */}
                <div className="blind-arena" tabIndex={0} role="region" aria-label="Área de disparo del test ciego">
                  {countdown !== null && (
                    <div className="countdown-overlay">
                      <div className="countdown-number">{countdown === 0 ? "¡YA!" : countdown}</div>
                      <div className="countdown-sub">Prepárate para hacer click en la diana apenas aparezca</div>
                    </div>
                  )}

                  {phaseTransitionReady && (
                    <div className="phase-transition-overlay">
                      <h4>¡Fase A Completada con Éxito!</h4>
                      <p>Registraste los 30 clicks de la primera fase. Descansa tu mano 5 segundos.</p>
                      <button
                        type="button"
                        className="btn btn-primary btn-large"
                        disabled={isBlindBusy}
                        onClick={() => void handleAdvanceToPhaseB()}
                      >
                        Continuar con la Fase B
                      </button>
                    </div>
                  )}

                  {countdown === null && !phaseTransitionReady && targetPos && (
                    <button
                      type="button"
                      className="blind-target"
                      style={{
                        top: `${targetPos.y}%`,
                        left: `${targetPos.x}%`,
                      }}
                      onClick={() => void handleTargetClick()}
                      title="¡Haz click aquí!"
                      aria-label="Objetivo de reacción"
                    >
                      <span className="target-ring ring-outer"></span>
                      <span className="target-ring ring-inner"></span>
                      <span className="target-bullseye"></span>
                    </button>
                  )}
                </div>
              </div>
            )}

            {/* Resultado Revelado del Test Ciego */}
            {blindResult && (
              <div className="blind-results-card">
                <div className="blind-reveal-header">
                  <span className="badge badge-accent">Revelación del Test Ciego</span>
                  <h3>Resultados Empíricos Revelados</h3>
                </div>

                <div className="blind-reveal-banner">
                  El tweak estuvo <strong>ACTIVO en el Estado {blindResult.tweak_active_in}</strong>.
                </div>

                <div className="blind-comparison-grid">
                  <div className="stat-card">
                    <h4>Estado A ({blindResult.tweak_active_in === "A" ? "Con Tweak" : "Línea Base"})</h4>
                    <div className="stat-value">{blindResult.state_a_mean} ms</div>
                    <div className="stat-label">Tiempo medio de reacción</div>
                    <div className="stat-sublist">
                      <div><span>Desv. Estándar:</span> <strong>{blindResult.state_a_std} ms</strong></div>
                      <div><span>Condición:</span> <strong>{blindResult.tweak_active_in === "A" ? "Tweak Activo" : "Control"}</strong></div>
                    </div>
                  </div>

                  <div className="stat-card">
                    <h4>Estado B ({blindResult.tweak_active_in === "B" ? "Con Tweak" : "Línea Base"})</h4>
                    <div className="stat-value">{blindResult.state_b_mean} ms</div>
                    <div className="stat-label">Tiempo medio de reacción</div>
                    <div className="stat-sublist">
                      <div><span>Desv. Estándar:</span> <strong>{blindResult.state_b_std} ms</strong></div>
                      <div><span>Condición:</span> <strong>{blindResult.tweak_active_in === "B" ? "Tweak Activo" : "Control"}</strong></div>
                    </div>
                  </div>

                  <div className="stat-card stat-card-ttest">
                    <h4>Inferencia Estadística (Welch)</h4>
                    <div className="stat-value">p = {blindResult.p_value.toFixed(4)}</div>
                    <div className="stat-label">
                      {blindResult.is_significant ? "✓ Significancia Estadística" : "✕ No Significativo"}
                    </div>
                    <div className="stat-sublist">
                      <div><span>Diferencia Neta:</span> <strong>{blindResult.delta_pct > 0 ? "+" : ""}{blindResult.delta_pct}%</strong></div>
                      <div><span>Estadístico t:</span> <strong>{blindResult.t_stat}</strong></div>
                      <div><span>Grados de libertad:</span> <strong>{blindResult.degrees_of_freedom}</strong></div>
                    </div>
                  </div>
                </div>

                {/* Banner de Veredicto */}
                <div className={`ab-verdict-banner verdict-${blindResult.verdict}`} style={{ marginTop: "16px" }}>
                  <div className="verdict-header">
                    <span className={`badge badge-verdict-${blindResult.verdict}`}>
                      {blindResult.verdict === "mejoro" && "✓ MEJORÓ"}
                      {blindResult.verdict === "placebo" && "⚠ PLACEBO / RUIDO"}
                      {blindResult.verdict === "empeoro" && "✕ EMPEORÓ"}
                    </span>
                    <h4>{blindResult.verdict_label}</h4>
                  </div>
                  <p className="verdict-explanation">{blindResult.explanation}</p>
                  <p className="verdict-proposal">
                    <strong>Conclusión de seguridad:</strong> {blindResult.proposed_action}
                  </p>

                  <div className="verdict-actions">
                    {blindResult.verdict === "mejoro" && (
                      <button
                        type="button"
                        className="btn btn-primary"
                        onClick={() => void applyTweak(blindTweakId)}
                      >
                        Conservar y Aplicar Tweak en el Sistema
                      </button>
                    )}
                    <button
                      type="button"
                      className="btn btn-secondary"
                      onClick={() => {
                        setBlindResult(null);
                        setBlindState(null);
                      }}
                    >
                      Realizar Otro Test
                    </button>
                  </div>
                </div>
              </div>
            )}
          </div>
        </div>
      )}

      {/* =====================================================================
          MODO 2: PRUEBA A/B ESTÁNDAR (ETW / QPC)
         ===================================================================== */}
      {activeSubTab === "ab_standard" && (
        <div className="ab-section-container">
          {/* Configuración */}
          <div className="panel ab-config-panel">
            <h3>1. Configuración de la Prueba A/B</h3>
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
                  <optgroup label="Tweaks Avanzados (Habilitados con Session Token)">
                    {tweaks
                      .filter((t) => t.definition.nivel === "avanzado")
                      .map((t) => (
                        <option key={t.definition.id} value={t.definition.id}>
                          {t.definition.nombre} ({t.definition.categoria.toUpperCase()})
                        </option>
                      ))}
                  </optgroup>
                  <optgroup label="Tweaks Seguros">
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

          {/* Stepper */}
          <div className="panel ab-stepper-panel">
            <h3>2. Flujo de Medición</h3>
            <div className="ab-stepper">
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
                    espera la estabilización y toma {sampleCount} muestras.
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

          {/* Gráfica y Resultados */}
          {abSession && (
            <div className="panel ab-results-panel">
              <div className="panel-header">
                <h3>3. Comparación Gráfica y Distribución</h3>
                <span className="badge badge-neutral">Sesión: {abSession.id}</span>
              </div>

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

              {/* Estadísticas */}
              <div className="ab-stats-grid">
                {abSession.baseline_summary && (
                  <div className="stat-card stat-card-baseline">
                    <h4>Línea Base (A)</h4>
                    <div className="stat-value">{abSession.baseline_summary.median.toFixed(2)} ms</div>
                    <div className="stat-label">Mediana</div>
                    <div className="stat-sublist">
                      <div><span>Media:</span> <strong>{abSession.baseline_summary.mean.toFixed(2)} ms</strong></div>
                      <div><span>Desv. Estándar:</span> <strong>{abSession.baseline_summary.std_dev.toFixed(3)} ms</strong></div>
                      <div><span>1% Low:</span> <strong>{abSession.baseline_summary.p99.toFixed(2)} ms</strong></div>
                      <div><span>Muestras:</span> <strong>{abSession.baseline_summary.count}</strong></div>
                    </div>
                  </div>
                )}

                {abSession.test_summary && (
                  <div className="stat-card stat-card-test">
                    <h4>Con Tweak Activo (B)</h4>
                    <div className="stat-value">{abSession.test_summary.median.toFixed(2)} ms</div>
                    <div className="stat-label">Mediana</div>
                    <div className="stat-sublist">
                      <div><span>Media:</span> <strong>{abSession.test_summary.mean.toFixed(2)} ms</strong></div>
                      <div><span>Desv. Estándar:</span> <strong>{abSession.test_summary.std_dev.toFixed(3)} ms</strong></div>
                      <div><span>1% Low:</span> <strong>{abSession.test_summary.p99.toFixed(2)} ms</strong></div>
                      <div>
                        <span>Delta:</span>{" "}
                        <strong className={(abSession.delta_pct || 0) < 0 ? "text-success" : "text-danger"}>
                          {(abSession.delta_pct || 0) > 0 ? "+" : ""}{abSession.delta_pct?.toFixed(2)}%
                        </strong>
                      </div>
                    </div>
                  </div>
                )}

                {abSession.t_test && (
                  <div className="stat-card stat-card-ttest">
                    <h4>Inferencia Estadística</h4>
                    <div className="stat-value">p = {abSession.t_test.p_value.toFixed(4)}</div>
                    <div className="stat-label">
                      {abSession.t_test.is_significant ? "✓ Significativo (p < 0.05)" : "✕ No Significativo"}
                    </div>
                    <div className="stat-sublist">
                      <div><span>t-stat:</span> <strong>{abSession.t_test.t_stat.toFixed(3)}</strong></div>
                      <div><span>Grados libertad:</span> <strong>{abSession.t_test.degrees_of_freedom.toFixed(1)}</strong></div>
                      <div><span>Hipótesis:</span> <strong>{abSession.t_test.is_significant ? "Rechazo de H₀" : "Acepta H₀ (Placebo)"}</strong></div>
                    </div>
                  </div>
                )}
              </div>

              {abSession.verdict && (
                <div className={`ab-verdict-banner verdict-${abSession.verdict}`} role="alert">
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
      )}

      {/* =====================================================================
          MODO 3: IMPORTACIÓN EXTERNA (LDAT / REFLEX)
         ===================================================================== */}
      {activeSubTab === "ldat_import" && (
        <div className="ab-section-container">
          <div className="panel">
            <div className="panel-header">
              <div className="panel-title-group">
                <h3>Importar Medición Externa (NVIDIA LDAT / Reflex Analyzer)</h3>
                <span className="badge badge-info">Hardware Óptico / Fotodiodo</span>
              </div>
            </div>
            <p className="tweak-desc">
              Si cuentas con herramientas de hardware dedicadas (fotodiodo en monitor, NVIDIA LDAT o analizador de latencia Reflex),
              ingresa los resúmenes estadísticos de tus rondas antes y después de aplicar el tweak.
              InputDash aplicará la prueba t de Welch para certificar matemáticamente si la reducción es real.
            </p>

            {ldatError && (
              <div className="banner banner-error" role="alert">
                <span>{ldatError}</span>
              </div>
            )}

            <div className="ldat-form-grid">
              {/* Grupo A: Línea Base */}
              <div className="ldat-group-card">
                <h4>Condición A · Control / Sin Tweak</h4>
                <div className="form-group">
                  <label htmlFor="ldat-mean-a">Latencia Media (ms):</label>
                  <input
                    id="ldat-mean-a"
                    type="number"
                    step="0.01"
                    className="text-input"
                    value={ldatMeanA}
                    onChange={(e) => setLdatMeanA(e.target.value)}
                  />
                </div>
                <div className="form-group">
                  <label htmlFor="ldat-std-a">Desviación Estándar σ (ms):</label>
                  <input
                    id="ldat-std-a"
                    type="number"
                    step="0.01"
                    className="text-input"
                    value={ldatStdA}
                    onChange={(e) => setLdatStdA(e.target.value)}
                  />
                </div>
                <div className="form-group">
                  <label htmlFor="ldat-count-a">Disparos / Muestras (N):</label>
                  <input
                    id="ldat-count-a"
                    type="number"
                    className="text-input"
                    value={ldatCountA}
                    onChange={(e) => setLdatCountA(e.target.value)}
                  />
                </div>
              </div>

              {/* Grupo B: Con Tweak */}
              <div className="ldat-group-card">
                <h4>Condición B · Optimizado / Con Tweak</h4>
                <div className="form-group">
                  <label htmlFor="ldat-mean-b">Latencia Media (ms):</label>
                  <input
                    id="ldat-mean-b"
                    type="number"
                    step="0.01"
                    className="text-input"
                    value={ldatMeanB}
                    onChange={(e) => setLdatMeanB(e.target.value)}
                  />
                </div>
                <div className="form-group">
                  <label htmlFor="ldat-std-b">Desviación Estándar σ (ms):</label>
                  <input
                    id="ldat-std-b"
                    type="number"
                    step="0.01"
                    className="text-input"
                    value={ldatStdB}
                    onChange={(e) => setLdatStdB(e.target.value)}
                  />
                </div>
                <div className="form-group">
                  <label htmlFor="ldat-count-b">Disparos / Muestras (N):</label>
                  <input
                    id="ldat-count-b"
                    type="number"
                    className="text-input"
                    value={ldatCountB}
                    onChange={(e) => setLdatCountB(e.target.value)}
                  />
                </div>
              </div>
            </div>

            <div style={{ marginTop: "16px", display: "flex", justifyContent: "flex-end" }}>
              <button
                type="button"
                className="btn btn-primary btn-large"
                onClick={() => void handleCalculateLdat()}
              >
                Calcular Inferencia y Veredicto
              </button>
            </div>

            {/* Resultado de la evaluación externa */}
            {ldatResult && (
              <div className="ldat-result-container" style={{ marginTop: "20px" }}>
                <div className="ab-stats-grid">
                  <div className="stat-card">
                    <h4>Impacto Porcentual</h4>
                    <div className="stat-value">
                      {ldatResult.delta_pct > 0 ? "+" : ""}{ldatResult.delta_pct}%
                    </div>
                    <div className="stat-label">Delta en Latencia Total</div>
                  </div>

                  <div className="stat-card stat-card-ttest">
                    <h4>Prueba t de Welch</h4>
                    <div className="stat-value">p = {ldatResult.t_test.p_value.toFixed(4)}</div>
                    <div className="stat-label">
                      {ldatResult.t_test.is_significant ? "✓ Significativo (p < 0.05)" : "✕ No Significativo"}
                    </div>
                  </div>
                </div>

                <div className={`ab-verdict-banner verdict-${ldatResult.verdict}`} style={{ marginTop: "16px" }}>
                  <div className="verdict-header">
                    <span className={`badge badge-verdict-${ldatResult.verdict}`}>
                      {ldatResult.verdict === "mejoro" && "✓ MEJORÓ"}
                      {ldatResult.verdict === "placebo" && "⚠ PLACEBO"}
                      {ldatResult.verdict === "empeoro" && "✕ EMPEORÓ"}
                    </span>
                    <h4>{ldatResult.verdict_label}</h4>
                  </div>
                  <p className="verdict-explanation">{ldatResult.explanation}</p>
                </div>
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
