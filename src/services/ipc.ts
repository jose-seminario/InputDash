import { invoke } from "@tauri-apps/api/core";
import type {
  ChangeLogEntry,
  MutationOutcome,
  StructuredBackendError,
  TweakStatusView,
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

export async function ipcRevertTweak(tweakId: string): Promise<MutationOutcome> {
  return invoke<MutationOutcome>("revert_tweak", { tweakId });
}

export async function ipcGetChangeLogs(limit = 50): Promise<ChangeLogEntry[]> {
  return invoke<ChangeLogEntry[]>("get_change_logs", { limit });
}
