import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type {
  AbSession,
  BackupSnapshot,
  BlindPublicState,
  BlindResult,
  BufferbloatResult,
  ChangeLogEntry,
  CoreContentionInfo,
  CpuTopologyInfo,
  DpcAnalysisResult,
  DriversInfo,
  ExportResult,
  ExternalLdatInput,
  ExternalLdatResult,
  FrametimeAnalysisResult,
  FullExportPayload,
  GameProfile,
  HardwareDiagnosticInfo,
  MutationOutcome,
  NetworkAnalysisResult,
  NicModerationInfo,
  PollingAnalysisResult,
  PresetId,
  PresetOutcome,
  ProcessItemView,
  RealtimeLatencyTick,
  RevertAllOutcome,
  SimulationItem,
  StructuredBackendError,
  TelInputData,
  TelReportResult,
  TimerResolutionStatus,
  TweakStatusView,
  UsbTopologyInfo,
} from "../types/tweak";

export function formatIpcError(err: unknown): StructuredBackendError {
  if (typeof err === "object" && err !== null) {
    const candidate = err as Record<string, unknown>;
    if (typeof candidate.code === "string" && typeof candidate.message === "string") {
      return {
        code: candidate.code,
        message: candidate.message,
      };
    }
  }
  if (typeof err === "string") {
    return { code: "IPC_ERROR", message: err };
  }
  if (err instanceof Error) {
    return { code: "RUNTIME_ERROR", message: err.message };
  }
  return {
    code: "UNKNOWN_ERROR",
    message: "Error desconocido al comunicarse con el motor nativo.",
  };
}

export async function ipcListTweaks(): Promise<TweakStatusView[]> {
  return invoke<TweakStatusView[]>("list_tweaks");
}

export async function ipcListValidatedTweaks(): Promise<string[]> {
  return invoke<string[]>("list_validated_tweaks");
}

export async function ipcApplyTweak(tweakId: string): Promise<MutationOutcome> {
  return invoke<MutationOutcome>("apply_tweak", { tweakId });
}

export async function ipcApplyPreset(presetId: PresetId): Promise<PresetOutcome> {
  return invoke<PresetOutcome>("apply_preset", { presetId });
}

export async function ipcRevertTweak(tweakId: string): Promise<MutationOutcome> {
  return invoke<MutationOutcome>("revert_tweak", { tweakId });
}

export async function ipcRevertAllTweaks(): Promise<RevertAllOutcome> {
  return invoke<RevertAllOutcome>("revert_all_tweaks");
}

export async function ipcSimulateTweaks(tweakIds: string[]): Promise<SimulationItem[]> {
  return invoke<SimulationItem[]>("simulate_tweaks", { tweakIds });
}

export async function ipcSimulatePreset(presetId: PresetId): Promise<SimulationItem[]> {
  return invoke<SimulationItem[]>("simulate_preset", { presetId });
}

export async function ipcCreateRestorePoint(): Promise<string> {
  return invoke<string>("create_restore_point");
}

export async function ipcGetActiveBackups(): Promise<BackupSnapshot[]> {
  return invoke<BackupSnapshot[]>("get_active_backups");
}

export async function ipcGetChangeLogs(limit = 100): Promise<ChangeLogEntry[]> {
  return invoke<ChangeLogEntry[]>("get_change_logs", { limit });
}

export async function ipcGetHardwareInfo(): Promise<HardwareDiagnosticInfo> {
  return invoke<HardwareDiagnosticInfo>("get_hardware_info");
}

export async function ipcStartPollingAnalysis(): Promise<PollingAnalysisResult> {
  return invoke<PollingAnalysisResult>("start_polling_analysis");
}

export async function ipcGetDriversInfo(): Promise<DriversInfo> {
  return invoke<DriversInfo>("get_drivers_info");
}

export async function ipcStartNetworkAnalysis(host?: string): Promise<NetworkAnalysisResult> {
  return invoke<NetworkAnalysisResult>("start_network_analysis", { host });
}

export async function ipcGetUsbTopology(): Promise<UsbTopologyInfo> {
  return invoke<UsbTopologyInfo>("get_usb_topology");
}

export async function ipcGetNicModeration(): Promise<NicModerationInfo> {
  return invoke<NicModerationInfo>("get_nic_moderation");
}

export async function ipcStartFrametimeAnalysis(): Promise<FrametimeAnalysisResult> {
  return invoke<FrametimeAnalysisResult>("start_frametime_analysis");
}

export async function ipcStartDpcAnalysis(): Promise<DpcAnalysisResult> {
  return invoke<DpcAnalysisResult>("start_dpc_analysis");
}

export async function ipcCalculateTel(inputData: TelInputData): Promise<TelReportResult> {
  return invoke<TelReportResult>("calculate_tel", { inputData });
}

// ---------------------------------------------------------------------------
// A/B Testing IPC (Etapa 4.2)
// ---------------------------------------------------------------------------

export async function ipcRecordAbBaseline(
  tweakId: string,
  tweakName: string,
  metric: string,
  samples: number[]
): Promise<AbSession> {
  return invoke<AbSession>("record_ab_baseline", {
    tweakId,
    tweakName,
    metric,
    samples,
  });
}

export async function ipcApplyAndEvaluateAb(samples: number[]): Promise<AbSession> {
  return invoke<AbSession>("apply_and_evaluate_ab", { samples });
}

export async function ipcGetAbSession(): Promise<AbSession | null> {
  return invoke<AbSession | null>("get_ab_session");
}

export async function ipcResetAbSession(): Promise<void> {
  return invoke<void>("reset_ab_session");
}

export async function ipcCaptureAbSyntheticSamples(count?: number): Promise<number[]> {
  return invoke<number[]>("capture_ab_synthetic_samples", { count });
}

// ---------------------------------------------------------------------------
// Double-Blind (Test Ciego) & LDAT IPC (Etapa 5)
// ---------------------------------------------------------------------------

export async function ipcStartBlindTest(
  tweakId: string,
  tweakName: string
): Promise<BlindPublicState> {
  return invoke<BlindPublicState>("start_blind_test", { tweakId, tweakName });
}

export async function ipcGetBlindState(): Promise<BlindPublicState> {
  return invoke<BlindPublicState>("get_blind_state");
}

export async function ipcRecordBlindSpawn(): Promise<number> {
  return invoke<number>("record_blind_spawn");
}

export async function ipcRecordBlindClick(
  reactionMs?: number
): Promise<[number, number, boolean]> {
  return invoke<[number, number, boolean]>("record_blind_click", { reactionMs });
}

export async function ipcAdvanceBlindPhase(): Promise<BlindPublicState> {
  return invoke<BlindPublicState>("advance_blind_phase");
}

export async function ipcCompleteBlindTest(): Promise<BlindResult> {
  return invoke<BlindResult>("complete_blind_test");
}

export async function ipcCancelBlindTest(): Promise<void> {
  return invoke<void>("cancel_blind_test");
}

export async function ipcEvaluateExternalLdat(
  input: ExternalLdatInput
): Promise<ExternalLdatResult> {
  return invoke<ExternalLdatResult>("evaluate_external_ldat", { input });
}

// ---------------------------------------------------------------------------
// Perfiles de Juego, Topología y Timer Resolution IPC (Etapa 6)
// ---------------------------------------------------------------------------

export async function ipcGetRunningProcesses(): Promise<ProcessItemView[]> {
  return invoke<ProcessItemView[]>("get_running_processes");
}

export async function ipcGetCpuTopology(): Promise<CpuTopologyInfo> {
  return invoke<CpuTopologyInfo>("get_cpu_topology");
}

export async function ipcGetCoreContention(): Promise<CoreContentionInfo> {
  return invoke<CoreContentionInfo>("get_core_contention");
}

export async function ipcGetGameProfiles(): Promise<GameProfile[]> {
  return invoke<GameProfile[]>("get_game_profiles");
}

export async function ipcToggleGameProfile(
  profileId: string,
  enabled: boolean
): Promise<GameProfile> {
  return invoke<GameProfile>("toggle_game_profile", { profileId, enabled });
}

export async function ipcUpdateGameProfile(profile: GameProfile): Promise<GameProfile> {
  return invoke<GameProfile>("update_game_profile", { profile });
}

export async function ipcApplyCpuSetsIsolate(pid: number): Promise<void> {
  return invoke<void>("apply_cpu_sets_isolate", { pid });
}

export async function ipcGetTimerResolution(): Promise<TimerResolutionStatus | null> {
  return invoke<TimerResolutionStatus | null>("get_timer_resolution");
}

// ---------------------------------------------------------------------------
// Bufferbloat y Exportación de Reporte IPC (Etapa 7)
// ---------------------------------------------------------------------------

export async function ipcStartBufferbloatAnalysis(
  host?: string
): Promise<BufferbloatResult> {
  return invoke<BufferbloatResult>("start_bufferbloat_analysis", { host });
}

export async function ipcExportFullReport(
  payload: FullExportPayload
): Promise<ExportResult> {
  return invoke<ExportResult>("export_full_report", { payload });
}

export async function listenToUsbDeviceChanges(
  callback: (msg: string) => void
): Promise<() => void> {
  return listen<string>("usb-device-changed", (event) => {
    callback(event.payload);
  });
}

// ---------------------------------------------------------------------------
// Monitor de Latencia en Tiempo Real IPC (Fase 9)
// ---------------------------------------------------------------------------

export async function ipcStartRealtimeMonitor(): Promise<void> {
  return invoke("start_realtime_monitor");
}

export async function ipcStopRealtimeMonitor(): Promise<void> {
  return invoke("stop_realtime_monitor");
}

export async function listenToRealtimeLatency(
  callback: (tick: RealtimeLatencyTick) => void
): Promise<() => void> {
  return listen<RealtimeLatencyTick>("realtime-latency-tick", (event) => {
    callback(event.payload);
  });
}

