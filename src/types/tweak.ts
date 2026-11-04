export type TweakCategory = "cpu" | "red" | "energia" | "gpu" | "periferico";
export type TweakLevel = "seguro" | "avanzado" | "bloqueado";
export type EvidenceLevel = "verificada" | "posible" | "descartada";
export type RiskLevel = "bajo" | "medio" | "alto";
export type GameProtocol = "udp" | "tcp" | "ambos";
export type RegistryHive = "HKCU" | "HKLM";
export type RegistryValueKind = "DWORD" | "QWORD" | "SZ";

export interface RegistryTarget {
  tipo: "registry";
  hive: RegistryHive;
  ruta: string;
  valor: string;
  dato: number | string;
  tipo_dato: RegistryValueKind;
}

export interface TweakDefinition {
  id: string;
  nombre: string;
  categoria: TweakCategory;
  nivel: TweakLevel;
  evidencia: EvidenceLevel;
  target: RegistryTarget;
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
  current_os_value: string;
  is_applied: boolean;
  has_backup: boolean;
  backup_original_value: string | null;
  status_error: string | null;
}

export interface MutationOutcome {
  tweak: TweakStatusView;
  restore_point_created: boolean;
  restore_point_warning: string | null;
  message: string;
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
