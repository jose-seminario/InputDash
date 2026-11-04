import { create } from "zustand";
import {
  formatIpcError,
  ipcApplyTweak,
  ipcGetChangeLogs,
  ipcListTweaks,
  ipcRevertTweak,
} from "../services/ipc";
import type {
  ChangeLogEntry,
  StructuredBackendError,
  TweakStatusView,
} from "../types/tweak";

interface TweaksState {
  tweaks: TweakStatusView[];
  logs: ChangeLogEntry[];
  isLoading: boolean;
  busyTweakId: string | null;
  errorBanner: StructuredBackendError | null;
  warningMessage: string | null;
  successMessage: string | null;
  refreshAll: () => Promise<void>;
  applyTweak: (tweakId: string) => Promise<void>;
  revertTweak: (tweakId: string) => Promise<void>;
  dismissMessages: () => void;
}

export const useTweaksStore = create<TweaksState>((set) => ({
  tweaks: [],
  logs: [],
  isLoading: false,
  busyTweakId: null,
  errorBanner: null,
  warningMessage: null,
  successMessage: null,

  dismissMessages: () =>
    set({ errorBanner: null, warningMessage: null, successMessage: null }),

  refreshAll: async () => {
    set({ isLoading: true, errorBanner: null });
    try {
      const [tweaks, logs] = await Promise.all([
        ipcListTweaks(),
        ipcGetChangeLogs(50),
      ]);
      set({ tweaks, logs, isLoading: false });
    } catch (err) {
      set({
        isLoading: false,
        errorBanner: formatIpcError(err),
      });
    }
  },

  applyTweak: async (tweakId: string) => {
    set({
      busyTweakId: tweakId,
      errorBanner: null,
      warningMessage: null,
      successMessage: null,
    });
    try {
      const outcome = await ipcApplyTweak(tweakId);
      const logs = await ipcGetChangeLogs(50);
      set((state) => ({
        busyTweakId: null,
        tweaks: state.tweaks.map((item) =>
          item.definition.id === tweakId ? outcome.tweak : item
        ),
        logs,
        warningMessage: outcome.restore_point_warning,
        successMessage: outcome.restore_point_created
          ? `${outcome.message} (Punto de restauración de Windows creado previamente).`
          : outcome.message,
      }));
    } catch (err) {
      const logs = await ipcGetChangeLogs(50).catch(() => null);
      set((state) => ({
        busyTweakId: null,
        logs: logs ?? state.logs,
        errorBanner: formatIpcError(err),
      }));
    }
  },

  revertTweak: async (tweakId: string) => {
    set({
      busyTweakId: tweakId,
      errorBanner: null,
      warningMessage: null,
      successMessage: null,
    });
    try {
      const outcome = await ipcRevertTweak(tweakId);
      const logs = await ipcGetChangeLogs(50);
      set((state) => ({
        busyTweakId: null,
        tweaks: state.tweaks.map((item) =>
          item.definition.id === tweakId ? outcome.tweak : item
        ),
        logs,
        warningMessage: outcome.restore_point_warning,
        successMessage: outcome.message,
      }));
    } catch (err) {
      const logs = await ipcGetChangeLogs(50).catch(() => null);
      set((state) => ({
        busyTweakId: null,
        logs: logs ?? state.logs,
        errorBanner: formatIpcError(err),
      }));
    }
  },
}));
