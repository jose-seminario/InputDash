export type TweakCategory = "cpu" | "red" | "energia" | "gpu" | "periferico";
export type TweakLevel = "seguro" | "avanzado" | "bloqueado";
export type EvidenceLevel = "verificada" | "posible" | "descartada";
export type RiskLevel = "bajo" | "medio" | "alto";
export type GameProtocol = "udp" | "tcp" | "ambos";
export type RegistryHive = "HKCU" | "HKLM";
export type RegistryValueKind = "DWORD" | "QWORD" | "SZ";
export type ActiveTab = "dashboard" | "optimizar" | "ab_testing" | "seguridad" | "antimitos";
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
  dedicated_vram_mb?: number;
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
  device_name?: string;
  is_connected?: boolean;
}

// Aliases de compatibilidad para periféricos y pantallas
export type MouseDeviceInfo = HidDeviceInfo;
export type KeyboardDeviceInfo = HidDeviceInfo;
export type DisplayInfo = MonitorInfo;

export interface HardwareDiagnosticInfo {
  gpu: GpuInfo;
  monitor: MonitorInfo;
  mice: HidDeviceInfo[];
  keyboards: HidDeviceInfo[];
  displays?: DisplayInfo[];
  primary_display_hz?: number;
  active_mouse_name?: string;
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
  estimated_hz?: number;
  interval_std_dev_ms?: number;
  explanation?: string;
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
  name?: string;
  hardware_id?: string;
}

export interface UsbHostControllerGroup {
  controller_id: string;
  controller_name: string;
  devices: UsbDeviceNode[];
  has_contention: boolean;
  contention_details: string | null;
}

// Aliases de compatibilidad para controladores USB
export type UsbControllerNode = UsbHostControllerGroup;

export interface UsbTopologyInfo {
  controllers: UsbHostControllerGroup[];
  any_contention_detected: boolean;
  has_contention?: boolean;
  mouse_controller_name?: string;
  status?: "optimo" | "advertencia" | "desconocido";
  status_label?: string;
  explanation?: string;
}

export interface FrametimeAnalysisResult {
  process_id: number;
  total_frames: number;
  duration_seconds: number;
  avg_frametime_ms: number;
  min_frametime_ms: number;
  max_frametime_ms: number;
  avg_fps: number;
  fps_1pct_low: number;
  fps_01pct_low: number;
  status: "optimo" | "aceptable" | "critico";
  status_label: string;
  explanation: string;
}

export interface DriverDpcInfo {
  driver_name: string;
  max_dpc_us: number;
  avg_dpc_us: number;
  total_calls: number;
}

export interface DpcAnalysisResult {
  duration_seconds: number;
  total_events: number;
  highest_dpc_us: number;
  top_drivers: DriverDpcInfo[];
  status: "optimo" | "aceptable" | "critico";
  status_label: string;
  explanation: string;
}

export interface TelInputData {
  mouse_avg_interval_ms?: number;
  mouse_polling_hz?: number;
  avg_frametime_ms?: number;
  monitor_refresh_hz?: number;
  network_avg_ping_ms?: number;
  tickrate?: number;
}

export interface TelComponent {
  name: string;
  description: string;
  latency_ms: number;
  is_measured: boolean;
  formula: string;
}

export interface TelEnemyAdvantage {
  enemy_ping_ms: number;
  enemy_tel_ms: number;
  advantage_ms: number;
  status: "ventaja" | "desventaja" | "neutral";
}

export interface TelReportResult {
  total_tel_ms: number;
  is_complete: boolean;
  missing_components: string[];
  components: TelComponent[];
  enemy_comparisons: TelEnemyAdvantage[];
  verdict: "optimo" | "aceptable" | "critico" | "incompleto";
  verdict_label: string;
  explanation: string;
}

// ---------------------------------------------------------------------------
// A/B Testing Types (Etapa 4.2)
// ---------------------------------------------------------------------------

export type AbSessionState = "idle" | "baselineready" | "tweakapplied" | "completed";
export type AbVerdict = "mejoro" | "placebo" | "empeoro";

export interface SampleSummary {
  count: number;
  mean: number;
  median: number;
  min: number;
  max: number;
  std_dev: number;
  p1: number;
  p0_1: number;
  p99: number;
  p99_9: number;
  fps_1pct_low?: number | null;
  fps_01pct_low?: number | null;
}

export interface TTestResult {
  t_stat: number;
  degrees_of_freedom: number;
  p_value: number;
  is_significant: boolean;
}

export interface AbSession {
  id: string;
  tweak_id: string;
  tweak_name: string;
  session_token?: string;
  metric: string;
  state: AbSessionState;
  baseline_samples: number[];
  test_samples: number[];
  baseline_summary?: SampleSummary | null;
  test_summary?: SampleSummary | null;
  t_test?: TTestResult | null;
  delta_pct?: number | null;
  verdict?: AbVerdict | null;
  verdict_label?: string | null;
  explanation?: string | null;
  proposed_action?: string | null;
  can_rollback: boolean;
}

// ---------------------------------------------------------------------------
// Double-Blind & External LDAT Types (Etapa 5)
// ---------------------------------------------------------------------------

export type BlindPhase = "idle" | "statea" | "stateb" | "completed" | "cancelled";

export interface BlindResult {
  tweak_active_in: string; // "A" | "B"
  state_a_mean: number;
  state_a_std: number;
  state_b_mean: number;
  state_b_std: number;
  baseline_mean: number;
  tweak_mean: number;
  delta_pct: number;
  t_stat: number;
  degrees_of_freedom: number;
  p_value: number;
  is_significant: boolean;
  verdict: "mejoro" | "placebo" | "empeoro";
  verdict_label: string;
  explanation: string;
  proposed_action: string;
}

export interface BlindPublicState {
  session_id: string;
  tweak_id: string;
  tweak_name: string;
  phase: BlindPhase;
  clicks_a_count: number;
  clicks_b_count: number;
  target_clicks: number;
  last_click_ms: number | null;
  result: BlindResult | null;
}

export interface ExternalLdatInput {
  mean_a: number;
  std_dev_a: number;
  count_a?: number;
  mean_b: number;
  std_dev_b: number;
  count_b?: number;
}

export interface ExternalLdatResult {
  t_test: TTestResult;
  delta_pct: number;
  verdict: "mejoro" | "placebo" | "empeoro";
  verdict_label: string;
  explanation: string;
}
