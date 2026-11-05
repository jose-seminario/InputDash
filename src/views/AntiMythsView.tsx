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
      "InputDash bloquea ofrecer Nagle en juegos UDP para evitar placebo, y solo lo habilita en juegos TCP reales (ej. MMOs). Para shooters UDP lo determinante es eliminar Bufferbloat (SQM en router) y usar conexión cableada Ethernet.",
  },
  {
    id: "dns_and_hitreg",
    title: "Cambiar Servidores DNS (1.1.1.1 / 8.8.8.8) para 'Bajar Ping o Mejorar Hitreg'",
    claim:
      "Mito popular: 'Cambiar las DNS públicas de Cloudflare o Google reduce el ping dentro de la partida y mejora el registro de impactos'.",
    reality:
      "El servidor DNS únicamente resuelve el nombre de dominio a dirección IP una sola vez al conectar o abrir el juego. Durante la partida, todos los paquetes viajan directamente por IP numérica al servidor de juego mediante enrutamiento BGP; el DNS no interviene en ningún momento del gameplay.",
    whatActuallyWorks:
      "Configurar un buen enrutamiento ISP con fibra óptica o usar tecnologías de enrutamiento optimizado hacia servidores de juego dedicados.",
  },
  {
    id: "hpet_and_bcdedit",
    title: "Desactivar HPET o Modificar bcdedit (useplatformclock / disabledynamictick)",
    claim:
      "Mito popular: 'Forzar disabledynamictick yes o useplatformclock false por comando bcdedit elimina micro-retrasos del temporizador del sistema'.",
    reality:
      "Windows 10 y 11 utilizan el TSC (Time Stamp Counter) invariante de la CPU moderna por defecto vía QPC. Forzar parámetros obsoletos de bcdedit desestabiliza la sincronización de reloj del kernel, deforma los frametimes (provocando micro-stuttering) y en muchos equipos causa fallos críticos de arranque (BSOD).",
    whatActuallyWorks:
      "Mantener la gestión de reloj nativa del sistema operativo basada en TSC sin alterar los parámetros de arranque del BCD.",
  },
  {
    id: "telemetry_and_background",
    title: "Eliminar o Deshabilitar Servicios de Telemetría (DiagTrack)",
    claim:
      "Mito popular: 'Desinstalar o romper la telemetría de Windows baja drásticamente la latencia de renderizado de la GPU y la CPU'.",
    reality:
      "La telemetría de Windows se ejecuta con prioridad IDLE/fondo en subprocesos de baja prioridad y no compite con la ruta crítica de renderizado (click-to-photon). Las mediciones científicas con herramientas de hardware como NVIDIA LDAT arrojan una diferencia neta de 0.0 ms. Además, romper estos servicios daña la integridad de Windows Update y componentes del sistema.",
    whatActuallyWorks:
      "Activar el Modo Juego (Game Mode) oficial de Windows, el cual prioriza automáticamente los hilos del proceso en primer plano y pausa tareas de mantenimiento del SO sin dañar dependencias.",
  },
  {
    id: "game_boosters_ram",
    title: "Game Boosters y Limpiadores Periódicos de RAM (EmptyWorkingSet)",
    claim:
      "Mito popular: 'Usar programas que liberan memoria RAM periódicamente evita tirones y hace que los juegos vayan más fluidos'.",
    reality:
      "Windows utiliza la memoria libre de forma inteligente como Standby Cache para almacenar assets, texturas y shaders reutilizados por el motor del juego. 'Liberar RAM' vacía abruptamente esta caché expulsándola hacia el disco o archivo de paginación, provocando hard page faults y caídas severas de los 1% y 0.1% lows (stuttering masivo) en pleno tiroteo.",
    whatActuallyWorks:
      "Permitir que el administrador de memoria virtual del kernel gestione la caché de forma nativa y cerrar aplicaciones pesadas en segundo plano (navegadores con vídeo, programas de render) antes de jugar.",
  },
  {
    id: "more_fps_99_gpu",
    title: "Más FPS Brutos con la GPU al 99% = Menos Input Lag",
    claim:
      "Mito popular: 'Jugar con FPS ilimitados (350 FPS con la GPU al 99-100%) siempre proporciona menor latencia que limitar a 225 FPS'.",
    reality:
      "Cuando la GPU alcanza el 98-100% de saturación, la CPU continúa acumulando cuadros pendientes en la Render Queue del controlador antes de que la GPU finalice el fotograma actual. Esta cola de renderizado en espera añade entre 12 ms y 40 ms de input lag a lo que se visualiza en pantalla.",
    whatActuallyWorks:
      "Activar NVIDIA Reflex (On + Boost) o AMD Anti-Lag 2 dentro del juego para vaciar la cola de renderizado, o aplicar VRR (G-Sync/FreeSync) + V-Sync activado en panel de control + límite de fotogramas ~3 FPS por debajo de la tasa de refresco del monitor.",
  },
  {
    id: "win32_priority_separation",
    title: "Modificar Win32PrioritySeparation en el Registro para 'Priorizar Juegos'",
    claim:
      "Mito popular: 'Ajustar Win32PrioritySeparation a valores esotéricos (como 26 hex o 28 hex) reduce la latencia del ratón y de la CPU'.",
    reality:
      "Las pruebas con instrumental físico (NVIDIA LDAT) y herramientas de telemetría de cuadros (PresentMon) demuestran cero reducción de latencia. Peor aún, forzar una separación arbitraria del quantum de planificación del kernel puede provocar inanición de hilos (thread starvation), donde los hilos del controlador del ratón USB y de la pila de red compiten por tiempo de CPU con el juego, causando micro-congelamientos y tirones.",
    whatActuallyWorks:
      "Mantener la gestión de hilos nativa de Windows 10/11 con Game Mode activo, permitiendo que el planificador del kernel y Windows Thread Director asignen los quantums dinámicamente según la carga.",
  },
  {
    id: "disable_network_offloads",
    title: "Desactivar Hardware Offloading de Red (Checksum / Large Send Offload)",
    claim:
      "Mito popular: 'Desactivar Large Send Offload (LSO) y Checksum Offload en las propiedades del adaptador de red reduce el input lag online'.",
    reality:
      "Las tarjetas de red modernas procesan la verificación y segmentación de paquetes mediante silicio dedicado (ASIC). Forzar a la CPU del sistema a calcular manualmente los checksums y fragmentar paquetes satura los núcleos del procesador por ráfagas de interrupciones innecesarias, degradando los FPS y los frametimes en lugar de reducir la latencia.",
    whatActuallyWorks:
      "Dejar el Hardware Offloading habilitado en el controlador de red. Para optimizar la latencia de red real, configurar SQM (CAKE / FQ-CoDel) en el router contra el Bufferbloat y utilizar conexión Ethernet cableada.",
  },
  {
    id: "mouse_8000hz_myth",
    title: "Ratones de 8000 Hz & Motion Sync: La Falacia del Máximo Polling Rate",
    claim:
      "Mito popular: 'Configurar el ratón en 8000 Hz siempre es mejor y garantiza menor input lag en cualquier juego'.",
    reality:
      "A 8000 Hz el ratón genera 8000 interrupciones por segundo por núcleo, lo que colapsa los controladores del bus USB y satura los hilos principales de la CPU en la mayoría de placas base, provocando micro-stutters y caídas bruscas de FPS durante barridos rápidos. El punto de equilibrio real, seguro y consistente es 2000 Hz. Además, Motion Sync añade algo de latencia a 1000 Hz (~0.5 ms para alinear lecturas con el intervalo de sondeo), pero a 2000 Hz o superior esa penalización es prácticamente nula.",
    whatActuallyWorks:
      "Utilizar 2000 Hz en ratones de alto rendimiento con sensores modernos y un puerto USB 3.0 directo a la placa base, obteniendo latencia sub-milisegundo sin sobrecargar el bus de interrupciones del procesador.",
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
