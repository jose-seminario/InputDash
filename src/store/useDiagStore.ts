import { create } from "zustand";
import {
  formatIpcError,
  ipcGetDriversInfo,
  ipcGetHardwareInfo,
  ipcGetNicModeration,
  ipcGetUsbTopology,
  ipcStartNetworkAnalysis,
  ipcStartPollingAnalysis,
} from "../services/ipc";
import type {
  DriversInfo,
  HardwareDiagnosticInfo,
  NetworkAnalysisResult,
  NicModerationInfo,
  PollingAnalysisResult,
  UsbTopologyInfo,
} from "../types/tweak";

interface DiagState {
  hardware: HardwareDiagnosticInfo | null;
  pollingResult: PollingAnalysisResult | null;
  drivers: DriversInfo | null;
  nicModeration: NicModerationInfo | null;
  usbTopology: UsbTopologyInfo | null;
  networkResult: NetworkAnalysisResult | null;
  targetHost: string;

  isHardwareLoading: boolean;
  isPollingAnalyzing: boolean;
  isNetworkAnalyzing: boolean;
  isUsbLoading: boolean;
  isDriversLoading: boolean;

  pollingSecondsLeft: number;
  pollingError: string | null;
  networkError: string | null;
  hardwareError: string | null;

  fetchHardwareInfo: () => Promise<void>;
  fetchDriversInfo: () => Promise<void>;
  fetchUsbTopology: () => Promise<void>;
  fetchNicModeration: () => Promise<void>;
  fetchAllDiagnostics: () => Promise<void>;

  startPollingAnalysis: () => Promise<void>;
  startNetworkAnalysis: (host?: string) => Promise<void>;
  setTargetHost: (host: string) => void;

  clearPollingResult: () => void;
  clearNetworkResult: () => void;
  dismissDiagError: () => void;
}

export const useDiagStore = create<DiagState>((set, get) => ({
  hardware: null,
  pollingResult: null,
  drivers: null,
  nicModeration: null,
  usbTopology: null,
  networkResult: null,
  targetHost: "8.8.8.8",

  isHardwareLoading: false,
  isPollingAnalyzing: false,
  isNetworkAnalyzing: false,
  isUsbLoading: false,
  isDriversLoading: false,

  pollingSecondsLeft: 4,
  pollingError: null,
  networkError: null,
  hardwareError: null,

  setTargetHost: (host) => set({ targetHost: host }),

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

  fetchDriversInfo: async () => {
    set({ isDriversLoading: true });
    try {
      const drivers = await ipcGetDriversInfo();
      set({ drivers, isDriversLoading: false });
    } catch (err) {
      set({ isDriversLoading: false });
    }
  },

  fetchNicModeration: async () => {
    try {
      const nicModeration = await ipcGetNicModeration();
      set({ nicModeration });
    } catch (err) {
      // Non-fatal
    }
  },

  fetchUsbTopology: async () => {
    set({ isUsbLoading: true });
    try {
      const usbTopology = await ipcGetUsbTopology();
      set({ usbTopology, isUsbLoading: false });
    } catch (err) {
      set({ isUsbLoading: false });
    }
  },

  fetchAllDiagnostics: async () => {
    await Promise.all([
      get().fetchHardwareInfo(),
      get().fetchDriversInfo(),
      get().fetchNicModeration(),
      get().fetchUsbTopology(),
    ]);
  },

  startPollingAnalysis: async () => {
    if (get().isPollingAnalyzing) return;

    set({
      isPollingAnalyzing: true,
      pollingSecondsLeft: 4,
      pollingError: null,
    });

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

  startNetworkAnalysis: async (hostParam) => {
    if (get().isNetworkAnalyzing) return;

    const host = hostParam || get().targetHost || "8.8.8.8";
    set({
      isNetworkAnalyzing: true,
      networkError: null,
    });

    try {
      const networkResult = await ipcStartNetworkAnalysis(host);
      set({
        networkResult,
        isNetworkAnalyzing: false,
      });
    } catch (err) {
      const structured = formatIpcError(err);
      set({
        isNetworkAnalyzing: false,
        networkError: structured.message,
      });
    }
  },

  clearPollingResult: () => set({ pollingResult: null, pollingError: null }),
  clearNetworkResult: () => set({ networkResult: null, networkError: null }),
  dismissDiagError: () =>
    set({ pollingError: null, networkError: null, hardwareError: null }),
}));
