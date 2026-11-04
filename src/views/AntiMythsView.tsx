import { useTweaksStore } from "../store/useTweaksStore";

interface EducationalPrinciple {
  id: string;
  title: string;
  claim: string;
  reality: string;
  whatActuallyWorks: string;
}

const VERIFIED_PRINCIPLES: EducationalPrinciple[] = [
  {
    id: "nagle_in_udp",
    title: "Algoritmo de Nagle (TCPNoDelay / TcpAckFrequency) en Shooters UDP",
    claim:
      "Mito popular: 'Desactivar Nagle en el Registro hace que las balas registren más rápido en CS2, Valorant o Warzone'.",
    reality:
      "El algoritmo de Nagle y los Delayed ACKs forman parte exclusiva de la máquina de estados del protocolo TCP (orientado a flujo con retransmisión). Todos los shooters competitivos modernos envían coordenadas y disparos mediante datagramas UDP puros, que jamás pasan por Nagle.",
    whatActuallyWorks:
      "InputDash bloquea ofrecer Nagle en juegos UDP para evitar placebo, y solo lo habilita en juegos TCP reales (ej. MMOs). Para shooters UDP lo determinante es eliminar Bufferbloat (SQM en router) y usar Ethernet.",
  },
  {
    id: "more_fps_99_gpu",
    title: "Más FPS Brutos con la GPU al 99% = Menos Input Lag",
    claim:
      "Mito popular: 'Jugar con FPS ilimitados (350 FPS al 99% de GPU) siempre da menor latencia que limitar a 225 FPS'.",
    reality:
      "Cuando la GPU alcanza el 98-100% de utilización, la CPU sigue encolando cuadros en la Render Queue antes de que la GPU termine el anterior. Esa fila de espera suma entre 12 ms y 40 ms de retraso a lo que ves en pantalla.",
    whatActuallyWorks:
      "Activar NVIDIA Reflex (On + Boost) o AMD Anti-Lag 2 dentro del juego, o configurar VRR + V-Sync ON en panel del driver + V-Sync OFF in-game + límite de FPS ~3 cuadros bajo los Hz del monitor.",
  },
];

export function AntiMythsView() {
  const { tweaks } = useTweaksStore();
  const blockedItems = tweaks.filter((t) => t.definition.nivel === "bloqueado");

  return (
    <div>
      <div className="antimyth-intro">
        <h2 className="section-heading" style={{ marginTop: 0 }}>
          Centro Educativo Anti-Mitos y Prácticas Bloqueadas
        </h2>
        <p className="tweak-desc" style={{ marginBottom: 0 }}>
          Gran parte de los supuestos optimizadores aplican scripts destructivos
          sin respaldo empírico. InputDash incluye una lista negra compilada en
          su motor (<code>core/guard.rs</code>) que impide ejecutar cualquiera de
          estos cambios peligrosos o inútiles y explica la evidencia técnica real.
        </p>
      </div>

      <h3 className="section-heading">
        1. Tweaks Bloqueados en InputDash ({blockedItems.length} protecciones activas)
      </h3>

      <div className="tweaks-list">
        {blockedItems.map((item) => {
          const { definition } = item;
          const reason =
            definition.target.tipo === "blocked"
              ? definition.target.motivo_bloqueo
              : "";

          return (
            <article key={definition.id} className="tweak-card tweak-card-blocked">
              <div className="tweak-info">
                <div className="tweak-title-row">
                  <h3 className="tweak-title">{definition.nombre}</h3>
                  <span className="badge badge-blocked">
                    BLOQUEADO / NO APLICABLE
                  </span>
                  <span className="badge badge-default">
                    evidencia: {definition.evidencia} · riesgo evitado:{" "}
                    {definition.riesgo}
                  </span>
                </div>

                <p className="tweak-desc">{definition.efecto_esperado}</p>

                {reason && (
                  <div className="myth-reason-box">
                    <strong>Fundamento técnico del bloqueo: </strong>
                    <span>{reason}</span>
                  </div>
                )}
              </div>
            </article>
          );
        })}
      </div>

      <h3 className="section-heading">
        2. Realidad Física del Input Lag y Registro de Disparos (Hitreg)
      </h3>

      <div className="tweaks-list">
        {VERIFIED_PRINCIPLES.map((item) => (
          <article key={item.id} className="tweak-card">
            <div className="tweak-info">
              <div className="tweak-title-row">
                <h3 className="tweak-title">{item.title}</h3>
                <span className="badge badge-applied">Evidencia Verificada</span>
              </div>
              <p className="tweak-desc">{item.claim}</p>
              <div className="tweak-meta-grid">
                <div>
                  <span className="meta-label">Por qué es falso:</span>
                  <span>{item.reality}</span>
                </div>
                <div>
                  <span className="meta-label">Qué sí reduce la latencia:</span>
                  <span>{item.whatActuallyWorks}</span>
                </div>
              </div>
            </div>
          </article>
        ))}
      </div>
    </div>
  );
}
