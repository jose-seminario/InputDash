export type TweakCategory = "cpu" | "red" | "energia" | "gpu" | "periferico";
export type TweakLevel = "seguro" | "avanzado" | "bloqueado";
export type EvidenceLevel = "verificada" | "posible" | "descartada";
export type RiskLevel = "bajo" | "medio" | "alto";
export type GameProtocol = "udp" | "tcp" | "ambos";
export type RegistryHive = "HKCU" | "HKLM";
export type RegistryValueKind = "DWORD" | "QWORD" | "SZ";
export type ActiveTab = "dashboard" | "optimizar" | "seguridad" | "antimitos";
export type PresetId = "competitivo" | "equilibrado";

export type TweakTarget =
  | {
      tipo: "registry";
      hive: RegistryHive;
      ruta: string;
      valor: string;
      dato: number | string;
      tipo_dato: RegistryValueKind;
    }
  | {
      tipo: "power_plan";
      guid: string;
    }
  | {
      tipo: "power_setting";
      subgroup_guid: string;
      setting_guid: string;
      ac_index: number;
      dc_index: number;
    }
  | {
      tipo: "process";
      accion: string;
      parametro: string;
    }
  | {
      tipo: "blocked";
      motivo_bloqueo: string;
    };

export interface TweakDefinition {
  id: string;
  nombre: string;
  categoria: TweakCategory;
  nivel: TweakLevel;
  evidencia: EvidenceLevel;
  target: TweakTarget;
  defecto: number | string;
  requiere_admin: boolean;
  requiere_reinicio: boolean;
  riesgo: RiskLevel;
  efecto_esperado: string;
  aplica_a: string[];
  juego_protocolo: GameProtocol;
}

export interface TweakStatusView {
  definition: TweakDefinition;
  target_summary: string;
  expected_value_display: string;
  current_os_value: string;
  is_applied: boolean;
  has_backup: boolean;
  backup_original_value: string | null;
  status_error: string | null;
}

export interface MutationOutcome {
  tweak: TweakStatusView;
  all_tweaks: TweakStatusView[];
  restore_point_created: boolean;
  restore_point_warning: string | null;
  message: string;
}

export interface PresetOutcome {
  preset_id: string;
  applied_ids: string[];
  all_tweaks: TweakStatusView[];
  restore_point_created: boolean;
  restore_point_warning: string | null;
  message: string;
}

export interface RevertAllOutcome {
  reverted_count: number;
  reverted_ids: string[];
  errors: string[];
  all_tweaks: TweakStatusView[];
  message: string;
}

export interface SimulationItem {
  tweak_id: string;
  tweak_name: string;
  nivel: TweakLevel;
  target_summary: string;
  current_os_value: string;
  proposed_value: string;
  would_change: boolean;
  requires_admin: boolean;
  reversion_preview: string;
  note: string | null;
}

export interface BackupSnapshot {
  schema_version: number;
  tweak_id: string;
  tweak_name: string;
  created_at_utc: string;
  payload: Record<string, unknown>;
}

export interface ChangeLogEntry {
  id: number;
  timestamp_utc: string;
  tweak_id: string;
  tweak_name: string;
  action: string;
  previous_value: string;
  new_value: string;
  verified: boolean;
  details: string;
}

export interface StructuredBackendError {
  code: string;
  message: string;
}

export interface GpuInfo {
  name: string;
  vram_mb: number;
  driver_version: string;
}

export interface MonitorInfo {
  resolution: string;
  refresh_hz: number;
  supports_vrr: boolean;
}

export interface HidDeviceInfo {
  name: string;
  vid: string;
  pid: string;
  transport: string;
  device_type: string;
  raw_path: string;
}

export interface HardwareDiagnosticInfo {
  gpu: GpuInfo;
  monitor: MonitorInfo;
  mice: HidDeviceInfo[];
  keyboards: HidDeviceInfo[];
}

export interface PollingAnalysisResult {
  measured_hz: number;
  avg_interval_ms: number;
  jitter_ms: number;
  min_interval_ms: number;
  max_interval_ms: number;
  sample_count: number;
  status: "optimo" | "aceptable" | "critico";
  status_label: string;
}

export interface DriversInfo {
  active_nic_name: string;
  active_nic_driver: string;
  primary_audio_name: string;
  primary_audio_driver: string;
}

export interface NicModerationInfo {
  adapter_name: string;
  value_str: string;
  is_enabled: boolean;
  verdict: string;
  explanation: string;
}

export interface IcmpAnalysisResult {
  target_host: string;
  pings_sent: number;
  pings_received: number;
  packet_loss_pct: number;
  avg_ping_ms: number;
  min_ping_ms: number;
  max_ping_ms: number;
  jitter_ms: number;
}

export interface UdpAnalysisResult {
  target_host: string;
  target_port: number;
  packets_sent: number;
  packets_received: number;
  packet_loss_pct: number;
  avg_rtt_ms: number;
  jitter_ms: number;
  min_rtt_ms: number;
  max_rtt_ms: number;
}

export interface NetworkAnalysisResult {
  host: string;
  icmp: IcmpAnalysisResult;
  udp: UdpAnalysisResult;
  status: "optimo" | "aceptable" | "critico";
  status_label: string;
  explanation: string;
}

export interface UsbDeviceNode {
  device_name: string;
  instance_id: string;
  is_mouse: boolean;
  is_audio: boolean;
}

export interface UsbHostControllerGroup {
  controller_id: string;
  controller_name: string;
  devices: UsbDeviceNode[];
  has_contention: boolean;
  contention_details: string | null;
}

export interface UsbTopologyInfo {
  controllers: UsbHostControllerGroup[];
  any_contention_detected: boolean;
}
