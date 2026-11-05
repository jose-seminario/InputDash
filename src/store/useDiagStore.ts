import { create } from "zustand";
import { formatIpcError, ipcGetHardwareInfo, ipcStartPollingAnalysis } from "../services/ipc";
import type { HardwareDiagnosticInfo, PollingAnalysisResult } from "../types/tweak";

interface DiagState {
  hardware: HardwareDiagnosticInfo | null;
  pollingResult: PollingAnalysisResult | null;
  isHardwareLoading: boolean;
  isPollingAnalyzing: boolean;
  pollingSecondsLeft: number;
  pollingError: string | null;
  hardwareError: string | null;

  fetchHardwareInfo: () => Promise<void>;
  startPollingAnalysis: () => Promise<void>;
  clearPollingResult: () => void;
  dismissDiagError: () => void;
}

export const useDiagStore = create<DiagState>((set, get) => ({
  hardware: null,
  pollingResult: null,
  isHardwareLoading: false,
  isPollingAnalyzing: false,
  pollingSecondsLeft: 4,
  pollingError: null,
  hardwareError: null,

  fetchHardwareInfo: async () => {
    set({ isHardwareLoading: true, hardwareError: null });
    try {
      const hardware = await ipcGetHardwareInfo();
      set({ hardware, isHardwareLoading: false });
    } catch (err) {
      const structured = formatIpcError(err);
      set({
        isHardwareLoading: false,
        hardwareError: `Error consultando hardware: ${structured.message}`,
      });
    }
  },

  startPollingAnalysis: async () => {
    if (get().isPollingAnalyzing) return;

    set({
      isPollingAnalyzing: true,
      pollingSecondsLeft: 4,
      pollingError: null,
    });

    // Countdown timer for 4 seconds
    const interval = setInterval(() => {
      set((state) => {
        const next = state.pollingSecondsLeft - 1;
        return { pollingSecondsLeft: next >= 0 ? next : 0 };
      });
    }, 1000);

    try {
      const pollingResult = await ipcStartPollingAnalysis();
      clearInterval(interval);
      set({
        pollingResult,
        isPollingAnalyzing: false,
        pollingSecondsLeft: 0,
      });
    } catch (err) {
      clearInterval(interval);
      const structured = formatIpcError(err);
      set({
        isPollingAnalyzing: false,
        pollingSecondsLeft: 0,
        pollingError: structured.message,
      });
    }
  },

  clearPollingResult: () => set({ pollingResult: null, pollingError: null }),
  dismissDiagError: () => set({ pollingError: null, hardwareError: null }),
}));
