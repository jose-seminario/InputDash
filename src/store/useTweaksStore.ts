import { create } from "zustand";
import {
  formatIpcError,
  ipcApplyAndEvaluateAb,
  ipcApplyPreset,
  ipcApplyTweak,
  ipcCaptureAbSyntheticSamples,
  ipcCreateRestorePoint,
  ipcGetAbSession,
  ipcGetActiveBackups,
  ipcGetChangeLogs,
  ipcListTweaks,
  ipcRecordAbBaseline,
  ipcResetAbSession,
  ipcRevertAllTweaks,
  ipcRevertTweak,
  ipcSimulatePreset,
  ipcSimulateTweaks,
} from "../services/ipc";
import type {
  AbSession,
  ActiveTab,
  BackupSnapshot,
  ChangeLogEntry,
  PresetId,
  SimulationItem,
  StructuredBackendError,
  TweakStatusView,
} from "../types/tweak";

interface TweaksState {
  activeTab: ActiveTab;
  simulationMode: boolean;
  simulationResults: SimulationItem[] | null;
  simulationLabel: string | null;
  tweaks: TweakStatusView[];
  backups: BackupSnapshot[];
  logs: ChangeLogEntry[];
  isLoading: boolean;
  busyAction: string | null;
  errorBanner: StructuredBackendError | null;
  warningMessage: string | null;
  successMessage: string | null;

  // Estado A/B Testing (Etapa 4.2)
  abSession: AbSession | null;
  isAbBusy: boolean;
  abBusyLabel: string | null;

  setActiveTab: (tab: ActiveTab) => void;
  setSimulationMode: (enabled: boolean) => void;
  clearSimulation: () => void;
  dismissMessages: () => void;

  refreshAll: () => Promise<void>;
  applyTweak: (tweakId: string) => Promise<void>;
  applyPreset: (presetId: PresetId) => Promise<void>;
  revertTweak: (tweakId: string) => Promise<void>;
  revertAll: () => Promise<void>;
  simulateAllSafe: () => Promise<void>;
  simulatePreset: (presetId: PresetId) => Promise<void>;
  createManualRestorePoint: () => Promise<void>;

  // Acciones A/B Testing
  refreshAbSession: () => Promise<void>;
  recordAbBaseline: (
    tweakId: string,
    tweakName: string,
    metric: string,
    samples: number[]
  ) => Promise<void>;
  applyAndEvaluateAb: (testSamples: number[]) => Promise<void>;
  resetAbSession: () => Promise<void>;
  captureSyntheticSamples: (count?: number) => Promise<number[]>;
}

export const useTweaksStore = create<TweaksState>((set, get) => ({
  activeTab: "optimizar",
  simulationMode: false,
  simulationResults: null,
  simulationLabel: null,
  tweaks: [],
  backups: [],
  logs: [],
  isLoading: false,
  busyAction: null,
  errorBanner: null,
  warningMessage: null,
  successMessage: null,

  abSession: null,
  isAbBusy: false,
  abBusyLabel: null,

  setActiveTab: (tab) => set({ activeTab: tab }),

  setSimulationMode: (enabled) =>
    set({
      simulationMode: enabled,
      simulationResults: enabled ? get().simulationResults : null,
      simulationLabel: enabled ? get().simulationLabel : null,
    }),

  clearSimulation: () => set({ simulationResults: null, simulationLabel: null }),

  dismissMessages: () =>
    set({ errorBanner: null, warningMessage: null, successMessage: null }),

  refreshAll: async () => {
    set({ isLoading: true, errorBanner: null });
    try {
      const [tweaks, backups, logs, abSession] = await Promise.all([
        ipcListTweaks(),
        ipcGetActiveBackups(),
        ipcGetChangeLogs(100),
        ipcGetAbSession().catch(() => null),
      ]);
      set({ tweaks, backups, logs, abSession, isLoading: false });
    } catch (err) {
      set({
        isLoading: false,
        errorBanner: formatIpcError(err),
      });
    }
  },

  applyTweak: async (tweakId: string) => {
    const { simulationMode } = get();
    set({
      busyAction: tweakId,
      errorBanner: null,
      warningMessage: null,
      successMessage: null,
    });

    if (simulationMode) {
      try {
        const items = await ipcSimulateTweaks([tweakId]);
        set({
          busyAction: null,
          simulationResults: items,
          simulationLabel: `Simulación individual (${tweakId})`,
          successMessage:
            "Modo Simulación: vista previa calculada en solo lectura. No se modificó ningún valor de Windows.",
        });
      } catch (err) {
        set({ busyAction: null, errorBanner: formatIpcError(err) });
      }
      return;
    }

    try {
      const outcome = await ipcApplyTweak(tweakId);
      const [backups, logs] = await Promise.all([
        ipcGetActiveBackups(),
        ipcGetChangeLogs(100),
      ]);
      set({
        busyAction: null,
        tweaks: outcome.all_tweaks,
        backups,
        logs,
        warningMessage: outcome.restore_point_warning,
        successMessage: outcome.restore_point_created
          ? `${outcome.message} (Punto de restauración de Windows creado automáticamente).`
          : outcome.message,
      });
    } catch (err) {
      const logs = await ipcGetChangeLogs(100).catch(() => null);
      set((state) => ({
        busyAction: null,
        logs: logs ?? state.logs,
        errorBanner: formatIpcError(err),
      }));
    }
  },

  applyPreset: async (presetId: PresetId) => {
    const { simulationMode } = get();
    if (simulationMode) {
      await get().simulatePreset(presetId);
      return;
    }

    set({
      busyAction: `preset:${presetId}`,
      errorBanner: null,
      warningMessage: null,
      successMessage: null,
    });

    try {
      const outcome = await ipcApplyPreset(presetId);
      const [backups, logs] = await Promise.all([
        ipcGetActiveBackups(),
        ipcGetChangeLogs(100),
      ]);
      set({
        busyAction: null,
        tweaks: outcome.all_tweaks,
        backups,
        logs,
        warningMessage: outcome.restore_point_warning,
        successMessage: outcome.restore_point_created
          ? `${outcome.message} (Punto de restauración creado previamente).`
          : outcome.message,
      });
    } catch (err) {
      const [tweaks, backups, logs] = await Promise.all([
        ipcListTweaks().catch(() => null),
        ipcGetActiveBackups().catch(() => null),
        ipcGetChangeLogs(100).catch(() => null),
      ]);
      set((state) => ({
        busyAction: null,
        tweaks: tweaks ?? state.tweaks,
        backups: backups ?? state.backups,
        logs: logs ?? state.logs,
        errorBanner: formatIpcError(err),
      }));
    }
  },

  revertTweak: async (tweakId: string) => {
    set({
      busyAction: tweakId,
      errorBanner: null,
      warningMessage: null,
      successMessage: null,
    });
    try {
      const outcome = await ipcRevertTweak(tweakId);
      const [backups, logs] = await Promise.all([
        ipcGetActiveBackups(),
        ipcGetChangeLogs(100),
      ]);
      set({
        busyAction: null,
        tweaks: outcome.all_tweaks,
        backups,
        logs,
        successMessage: outcome.message,
      });
    } catch (err) {
      const logs = await ipcGetChangeLogs(100).catch(() => null);
      set((state) => ({
        busyAction: null,
        logs: logs ?? state.logs,
        errorBanner: formatIpcError(err),
      }));
    }
  },

  revertAll: async () => {
    set({
      busyAction: "revert_all",
      errorBanner: null,
      warningMessage: null,
      successMessage: null,
    });
    try {
      const outcome = await ipcRevertAllTweaks();
      const [backups, logs] = await Promise.all([
        ipcGetActiveBackups(),
        ipcGetChangeLogs(100),
      ]);
      set({
        busyAction: null,
        tweaks: outcome.all_tweaks,
        backups,
        logs,
        warningMessage:
          outcome.errors.length > 0 ? outcome.errors.join(" | ") : null,
        successMessage: outcome.message,
      });
    } catch (err) {
      set({
        busyAction: null,
        errorBanner: formatIpcError(err),
      });
    }
  },

  simulateAllSafe: async () => {
    const safeIds = get()
      .tweaks.filter((t) => t.definition.nivel === "seguro")
      .map((t) => t.definition.id);

    set({
      busyAction: "sim_safe",
      errorBanner: null,
      warningMessage: null,
      successMessage: null,
    });
    try {
      const items = await ipcSimulateTweaks(safeIds);
      set({
        busyAction: null,
        simulationResults: items,
        simulationLabel: "Simulación completa de los 6 tweaks seguros",
        successMessage:
          "Simulación ejecutada en solo lectura (cero escrituras en Registro ni PowrProf).",
      });
    } catch (err) {
      set({ busyAction: null, errorBanner: formatIpcError(err) });
    }
  },

  simulatePreset: async (presetId: PresetId) => {
    set({
      busyAction: `sim_${presetId}`,
      errorBanner: null,
      warningMessage: null,
      successMessage: null,
    });
    try {
      const items = await ipcSimulatePreset(presetId);
      const label =
        presetId === "competitivo" ? "Preset Competitivo" : "Preset Equilibrado";
      set({
        busyAction: null,
        simulationResults: items,
        simulationLabel: `Simulación de ${label}`,
        successMessage: `Vista previa de ${label} generada sin modificar el sistema.`,
      });
    } catch (err) {
      set({ busyAction: null, errorBanner: formatIpcError(err) });
    }
  },

  createManualRestorePoint: async () => {
    set({
      busyAction: "restore_point",
      errorBanner: null,
      warningMessage: null,
      successMessage: null,
    });
    try {
      const msg = await ipcCreateRestorePoint();
      const logs = await ipcGetChangeLogs(100);
      set({
        busyAction: null,
        logs,
        successMessage: msg,
      });
    } catch (err) {
      set({
        busyAction: null,
        errorBanner: formatIpcError(err),
      });
    }
  },

  refreshAbSession: async () => {
    try {
      const session = await ipcGetAbSession();
      set({ abSession: session });
    } catch (err) {
      set({ errorBanner: formatIpcError(err) });
    }
  },

  recordAbBaseline: async (tweakId, tweakName, metric, samples) => {
    set({ isAbBusy: true, abBusyLabel: "Registrando Baseline..." });
    try {
      const session = await ipcRecordAbBaseline(tweakId, tweakName, metric, samples);
      set({ abSession: session, isAbBusy: false, abBusyLabel: null });
    } catch (err) {
      set({ isAbBusy: false, abBusyLabel: null, errorBanner: formatIpcError(err) });
    }
  },

  applyAndEvaluateAb: async (testSamples) => {
    set({ isAbBusy: true, abBusyLabel: "Aplicando tweak y evaluando hipótesis A/B..." });
    try {
      const session = await ipcApplyAndEvaluateAb(testSamples);
      const tweaks = await ipcListTweaks();
      const backups = await ipcGetActiveBackups();
      const logs = await ipcGetChangeLogs(100);
      set({
        abSession: session,
        tweaks,
        backups,
        logs,
        isAbBusy: false,
        abBusyLabel: null,
      });
    } catch (err) {
      set({ isAbBusy: false, abBusyLabel: null, errorBanner: formatIpcError(err) });
    }
  },

  resetAbSession: async () => {
    try {
      await ipcResetAbSession();
      set({ abSession: null });
    } catch (err) {
      set({ errorBanner: formatIpcError(err) });
    }
  },

  captureSyntheticSamples: async (count) => {
    return ipcCaptureAbSyntheticSamples(count);
  },
}));
