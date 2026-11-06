import { create } from "zustand";
import {
  formatIpcError,
  ipcCalculateTel,
  ipcGetDriversInfo,
  ipcGetHardwareInfo,
  ipcGetNicModeration,
  ipcGetUsbTopology,
  ipcStartDpcAnalysis,
  ipcStartFrametimeAnalysis,
  ipcStartNetworkAnalysis,
  ipcStartPollingAnalysis,
} from "../services/ipc";
import type {
  DpcAnalysisResult,
  DriversInfo,
  FrametimeAnalysisResult,
  HardwareDiagnosticInfo,
  NetworkAnalysisResult,
  NicModerationInfo,
  PollingAnalysisResult,
  TelReportResult,
  UsbTopologyInfo,
} from "../types/tweak";

interface DiagState {
  hardware: HardwareDiagnosticInfo | null;
  pollingResult: PollingAnalysisResult | null;
  drivers: DriversInfo | null;
  nicModeration: NicModerationInfo | null;
  usbTopology: UsbTopologyInfo | null;
  networkResult: NetworkAnalysisResult | null;
  frametimeResult: FrametimeAnalysisResult | null;
  dpcResult: DpcAnalysisResult | null;
  telReport: TelReportResult | null;
  targetHost: string;

  isHardwareLoading: boolean;
  isPollingAnalyzing: boolean;
  isNetworkAnalyzing: boolean;
  isUsbLoading: boolean;
  isDriversLoading: boolean;
  isFrametimeAnalyzing: boolean;
  isDpcAnalyzing: boolean;
  isTelLoading: boolean;

  pollingSecondsLeft: number;
  frametimeSecondsLeft: number;
  dpcSecondsLeft: number;

  pollingError: string | null;
  networkError: string | null;
  hardwareError: string | null;
  frametimeError: string | null;
  dpcError: string | null;
  telError: string | null;

  fetchHardwareInfo: () => Promise<void>;
  fetchDriversInfo: () => Promise<void>;
  fetchUsbTopology: () => Promise<void>;
  fetchNicModeration: () => Promise<void>;
  fetchAllDiagnostics: () => Promise<void>;

  startPollingAnalysis: () => Promise<void>;
  startNetworkAnalysis: (host?: string) => Promise<void>;
  startFrametimeAnalysis: () => Promise<void>;
  startDpcAnalysis: () => Promise<void>;
  updateTelReport: (customTickrate?: number) => Promise<void>;

  setTargetHost: (host: string) => void;
  clearPollingResult: () => void;
  clearNetworkResult: () => void;
  clearFrametimeResult: () => void;
  clearDpcResult: () => void;
  dismissDiagError: () => void;
}

export const useDiagStore = create<DiagState>((set, get) => ({
  hardware: null,
  pollingResult: null,
  drivers: null,
  nicModeration: null,
  usbTopology: null,
  networkResult: null,
  frametimeResult: null,
  dpcResult: null,
  telReport: null,
  targetHost: "8.8.8.8",

  isHardwareLoading: false,
  isPollingAnalyzing: false,
  isNetworkAnalyzing: false,
  isUsbLoading: false,
  isDriversLoading: false,
  isFrametimeAnalyzing: false,
  isDpcAnalyzing: false,
  isTelLoading: false,

  pollingSecondsLeft: 4,
  frametimeSecondsLeft: 10,
  dpcSecondsLeft: 10,

  pollingError: null,
  networkError: null,
  hardwareError: null,
  frametimeError: null,
  dpcError: null,
  telError: null,

  setTargetHost: (host) => set({ targetHost: host }),

  fetchHardwareInfo: async () => {
    set({ isHardwareLoading: true, hardwareError: null });
    try {
      const hardware = await ipcGetHardwareInfo();
      set({ hardware, isHardwareLoading: false });
      void get().updateTelReport();
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
    } catch {
      set({ isDriversLoading: false });
    }
  },

  fetchNicModeration: async () => {
    try {
      const nicModeration = await ipcGetNicModeration();
      set({ nicModeration });
    } catch {
      // Non-fatal
    }
  },

  fetchUsbTopology: async () => {
    set({ isUsbLoading: true });
    try {
      const usbTopology = await ipcGetUsbTopology();
      set({ usbTopology, isUsbLoading: false });
    } catch {
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
    void get().updateTelReport();
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
      void get().updateTelReport();
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
      void get().updateTelReport();
    } catch (err) {
      const structured = formatIpcError(err);
      set({
        isNetworkAnalyzing: false,
        networkError: structured.message,
      });
    }
  },

  startFrametimeAnalysis: async () => {
    if (get().isFrametimeAnalyzing) return;

    set({
      isFrametimeAnalyzing: true,
      frametimeSecondsLeft: 10,
      frametimeError: null,
    });

    const interval = setInterval(() => {
      set((state) => {
        const next = state.frametimeSecondsLeft - 1;
        return { frametimeSecondsLeft: next >= 0 ? next : 0 };
      });
    }, 1000);

    try {
      const frametimeResult = await ipcStartFrametimeAnalysis();
      clearInterval(interval);
      set({
        frametimeResult,
        isFrametimeAnalyzing: false,
        frametimeSecondsLeft: 0,
      });
      void get().updateTelReport();
    } catch (err) {
      clearInterval(interval);
      const structured = formatIpcError(err);
      set({
        isFrametimeAnalyzing: false,
        frametimeSecondsLeft: 0,
        frametimeError: structured.message,
      });
    }
  },

  startDpcAnalysis: async () => {
    if (get().isDpcAnalyzing) return;

    set({
      isDpcAnalyzing: true,
      dpcSecondsLeft: 10,
      dpcError: null,
    });

    const interval = setInterval(() => {
      set((state) => {
        const next = state.dpcSecondsLeft - 1;
        return { dpcSecondsLeft: next >= 0 ? next : 0 };
      });
    }, 1000);

    try {
      const dpcResult = await ipcStartDpcAnalysis();
      clearInterval(interval);
      set({
        dpcResult,
        isDpcAnalyzing: false,
        dpcSecondsLeft: 0,
      });
    } catch (err) {
      clearInterval(interval);
      const structured = formatIpcError(err);
      set({
        isDpcAnalyzing: false,
        dpcSecondsLeft: 0,
        dpcError: structured.message,
      });
    }
  },

  updateTelReport: async (customTickrate = 64) => {
    const s = get();
    set({ isTelLoading: true, telError: null });

    const input = {
      mouse_avg_interval_ms: s.pollingResult?.avg_interval_ms,
      mouse_polling_hz: s.pollingResult?.measured_hz,
      avg_frametime_ms: s.frametimeResult?.avg_frametime_ms,
      monitor_refresh_hz: s.hardware?.monitor.refresh_hz,
      network_avg_ping_ms: s.networkResult?.icmp.avg_ping_ms,
      tickrate: customTickrate,
    };

    try {
      const telReport = await ipcCalculateTel(input);
      set({ telReport, isTelLoading: false });
    } catch (err) {
      const structured = formatIpcError(err);
      set({ isTelLoading: false, telError: structured.message });
    }
  },

  clearPollingResult: () => {
    set({ pollingResult: null, pollingError: null });
    void get().updateTelReport();
  },
  clearNetworkResult: () => {
    set({ networkResult: null, networkError: null });
    void get().updateTelReport();
  },
  clearFrametimeResult: () => {
    set({ frametimeResult: null, frametimeError: null });
    void get().updateTelReport();
  },
  clearDpcResult: () => set({ dpcResult: null, dpcError: null }),
  dismissDiagError: () =>
    set({
      pollingError: null,
      networkError: null,
      hardwareError: null,
      frametimeError: null,
      dpcError: null,
      telError: null,
    }),
}));
