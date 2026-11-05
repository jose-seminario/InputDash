import { invoke } from "@tauri-apps/api/core";
import type {
  BackupSnapshot,
  ChangeLogEntry,
  MutationOutcome,
  PresetId,
  PresetOutcome,
  RevertAllOutcome,
  SimulationItem,
  StructuredBackendError,
  TweakStatusView,
  HardwareDiagnosticInfo,
  PollingAnalysisResult,
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
