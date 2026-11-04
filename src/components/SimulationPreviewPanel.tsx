import type { SimulationItem } from "../types/tweak";

interface SimulationPreviewPanelProps {
  label: string | null;
  items: SimulationItem[];
  onClose: () => void;
}

export function SimulationPreviewPanel({
  label,
  items,
  onClose,
}: SimulationPreviewPanelProps) {
  return (
    <section className="sim-preview-box">
      <div className="sim-preview-header">
        <div>
          <strong>Vista Previa — Modo Simulación (Solo Lectura)</strong>
          {label && <span className="sim-preview-sub"> · {label}</span>}
        </div>
        <button type="button" className="banner-close" onClick={onClose}>
          Ocultar simulación
        </button>
      </div>

      <table className="logs-table">
        <thead>
          <tr>
            <th>Tweak</th>
            <th>Objetivo Nativo</th>
            <th>Estado Actual OS</th>
            <th>Valor Propuesto</th>
            <th>Efecto Simulado</th>
            <th>Plan de Reversión</th>
          </tr>
        </thead>
        <tbody>
          {items.map((item) => (
            <tr key={item.tweak_id}>
              <td>
                <strong>{item.tweak_name}</strong>
                {item.requires_admin && (
                  <div className="meta-sub-note">Requiere Admin</div>
                )}
              </td>
              <td className="meta-mono">{item.target_summary}</td>
              <td className="meta-mono">{item.current_os_value}</td>
              <td className="meta-mono">{item.proposed_value}</td>
              <td>
                <span
                  className={`badge ${
                    item.would_change ? "badge-warn" : "badge-default"
                  }`}
                >
                  {item.would_change ? "Modificaría valor" : "Ya coincide / Sin cambio"}
                </span>
                {item.note && <div className="meta-sub-note">{item.note}</div>}
              </td>
              <td>{item.reversion_preview}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </section>
  );
}
