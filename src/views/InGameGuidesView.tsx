import { useState } from "react";

export function InGameGuidesView() {
  const [activeCategory, setActiveCategory] = useState<
    "reflex" | "vrr" | "mouse" | "router" | "hvci"
  >("reflex");

  return (
    <div className="view-container">
      {/* Encabezado */}
      <div className="view-header">
        <div className="view-title-group">
          <h2>Guías Educativas de Configuración In-Game</h2>
          <p>
            Optimizaciones comprobadas por instrumental físico (LDAT y Reflex Analyzer).
            InputDash <strong>no modifica los archivos del juego</strong>; estas configuraciones deben ajustarse dentro de cada título y en el panel de tu GPU.
          </p>
        </div>
      </div>

      {/* Selector de Guías */}
      <div className="ab-subtabs">
        <button
          type="button"
          className={`ab-subtab ${activeCategory === "reflex" ? "active" : ""}`}
          onClick={() => setActiveCategory("reflex")}
        >
          ⚡ NVIDIA Reflex / AMD Anti-Lag 2
        </button>
        <button
          type="button"
          className={`ab-subtab ${activeCategory === "vrr" ? "active" : ""}`}
          onClick={() => setActiveCategory("vrr")}
        >
          🖥️ VRR + V-Sync + Límite de FPS
        </button>
        <button
          type="button"
          className={`ab-subtab ${activeCategory === "mouse" ? "active" : ""}`}
          onClick={() => setActiveCategory("mouse")}
        >
          🖱️ Periféricos, Raw Input y Debounce
        </button>
        <button
          type="button"
          className={`ab-subtab ${activeCategory === "router" ? "active" : ""}`}
          onClick={() => setActiveCategory("router")}
        >
          🌐 Mitigación de Bufferbloat (SQM)
        </button>
        <button
          type="button"
          className={`ab-subtab ${activeCategory === "hvci" ? "active" : ""}`}
          onClick={() => setActiveCategory("hvci")}
        >
          🛡️ Integridad de Memoria (HVCI)
        </button>
      </div>

      {/* 1. GUÍA: NVIDIA REFLEX Y AMD ANTI-LAG 2 */}
      {activeCategory === "reflex" && (
        <div className="guide-content-panel panel">
          <div className="guide-header">
            <h3>NVIDIA Reflex y AMD Anti-Lag 2: Eliminación de la Cola de Render</h3>
            <span className="badge badge-accent">Impacto: 15 ms a 45 ms de Reducción</span>
          </div>

          <div className="guide-box guide-box-highlight">
            <h4>¿Por qué es la optimización más importante de la era moderna?</h4>
            <p>
              Cuando tu tarjeta gráfica trabaja al <strong>98% – 100% de uso (GPU-Bound)</strong>, el procesador (CPU)
              prepara los cuadros más rápido de lo que la GPU puede dibujarlos. Sin sincronización, el motor gráfico
              almacena entre 2 y 4 fotogramas en una cola previa (<em>Render Queue</em>).
            </p>
            <p>
              <strong>Consecuencia:</strong> Cuando haces clic para disparar, tu disparo se introduce al final de esa cola.
              Aunque veas 200 FPS en pantalla, estás experimentando un retardo artificial de hasta <strong>40 milisegundos</strong>.
            </p>
          </div>

          <div className="guide-grid-2col">
            <div className="guide-step-card">
              <h4>Configuración Recomendada en el Juego</h4>
              <ul className="guide-checklist">
                <li>
                  <strong>NVIDIA Reflex:</strong> Selecciona <code>Activado (ON)</code> o <code>Activado + Aumento (ON + BOOST)</code> en el menú de gráficos del juego (CS2, Valorant, Apex, Warzone, Fortnite).
                </li>
                <li>
                  <strong>¿Cuándo usar BOOST?:</strong> Si tu juego tiene caídas bruscas a escenarios dependientes de la CPU (como tiroteos con múltiples granadas de humo), <em>Boost</em> obliga a la GPU a mantener sus frecuencias máximas de reloj, evitando caídas de voltaje de transición.
                </li>
                <li>
                  <strong>AMD Anti-Lag 2:</strong> En juegos compatibles con Anti-Lag 2 integrado en el motor (como CS2), actívalo dentro de las opciones de vídeo del juego.
                </li>
              </ul>
            </div>

            <div className="guide-step-card">
              <h4>Regla de Oro de los FPS</h4>
              <p>
                <strong>Tener más FPS NO garantiza menor latencia si la GPU está al 99%:</strong>
              </p>
              <div className="comparison-metric-box">
                <div>
                  <span className="label">180 FPS al 99% GPU (Sin Reflex)</span>
                  <span className="value text-danger">~42 ms de Latencia Total</span>
                </div>
                <div>
                  <span className="label">160 FPS con Reflex Activo</span>
                  <span className="value text-success">~14 ms de Latencia Total</span>
                </div>
              </div>
              <p className="note-text">
                Reflex sincroniza el hilo de simulación de la CPU exactamente en el instante en que la GPU está lista para renderizar el siguiente cuadro, reduciendo la cola a 0 cuadros.
              </p>
            </div>
          </div>
        </div>
      )}

      {/* 2. GUÍA: VRR + V-SYNC + FPS CAP */}
      {activeCategory === "vrr" && (
        <div className="guide-content-panel panel">
          <div className="guide-header">
            <h3>El Estándar Competitivo: VRR + V-Sync en Panel + FPS Cap</h3>
            <span className="badge badge-accent">Cero Tearing + Cero Retardo de Buffer</span>
          </div>

          <div className="guide-box">
            <h4>El Gran Mito: "¿V-Sync siempre añade input lag?"</h4>
            <p>
              <strong>Falso en pantallas con VRR (G-Sync / FreeSync):</strong> El input lag del V-Sync tradicional de 60 Hz ocurría porque la GPU debía esperar pasivamente al refresco fijo del monitor.
              En un monitor moderno con tasa de refresco variable (VRR), el monitor es el que se sincroniza a la velocidad exacta de la GPU.
            </p>
          </div>

          <div className="guide-steps-list">
            <div className="step-item">
              <span className="step-number">1</span>
              <div className="step-content">
                <h4>Paso 1: G-Sync o FreeSync ACTIVADO en el Controlador de la GPU</h4>
                <p>
                  Abre el <em>Panel de Control de NVIDIA</em> o <em>AMD Software</em> y activa G-Sync / FreeSync tanto para pantalla completa como para modo ventana.
                </p>
              </div>
            </div>

            <div className="step-item">
              <span className="step-number">2</span>
              <div className="step-content">
                <h4>Paso 2: Sincronización Vertical ACTIVADA en el Panel de la GPU</h4>
                <p>
                  En <em>Configuración 3D Global</em> del panel de la GPU, establece <strong>Sincronización Vertical: Activado</strong>.
                  Esto previene el desgarro de imagen (*frame tearing*) en la parte inferior del panel sin añadir colas de doble búfer.
                </p>
              </div>
            </div>

            <div className="step-item">
              <span className="step-number">3</span>
              <div className="step-content">
                <h4>Paso 3: Sincronización Vertical DESACTIVADA en el Menú del Juego</h4>
                <p>
                  Dentro del juego, asegúrate siempre de que el V-Sync interno esté en <strong>Desactivado (OFF)</strong>.
                  La sincronización debe ser gestionada únicamente a nivel del driver.
                </p>
              </div>
            </div>

            <div className="step-item">
              <span className="step-number">4</span>
              <div className="step-content">
                <h4>Paso 4: Limitar los FPS 3 Cuadros por Debajo del Refresco Máximo</h4>
                <p>
                  Para evitar que los FPS alcancen el techo del monitor (donde el módulo VRR se desactiva y se crea búfer de V-Sync), aplica un límite de FPS:
                </p>
                <div className="fps-cap-table">
                  <div><span>Monitor 144 Hz</span> ➔ <strong>Limitar a 141 FPS</strong></div>
                  <div><span>Monitor 165 Hz</span> ➔ <strong>Limitar a 160 FPS</strong></div>
                  <div><span>Monitor 240 Hz</span> ➔ <strong>Limitar a 237 FPS</strong></div>
                  <div><span>Monitor 360 Hz</span> ➔ <strong>Limitar a 355 FPS</strong></div>
                </div>
                <p className="note-text">
                  *Nota: Si el juego cuenta con NVIDIA Reflex, el límite de cuadros se aplica automáticamente sin necesidad de limitador externo.*
                </p>
              </div>
            </div>
          </div>
        </div>
      )}

      {/* 3. GUÍA: PERIFÉRICOS Y MOUSE */}
      {activeCategory === "mouse" && (
        <div className="guide-content-panel panel">
          <div className="guide-header">
            <h3>Optimización de Ratón: Polling Rate, Debounce y Raw Input</h3>
            <span className="badge badge-accent">Latencia Periférica</span>
          </div>

          <div className="guide-grid-2col">
            <div className="guide-step-card">
              <h4>1. Entrada Directa (Raw Input)</h4>
              <p>
                Asegúrate de que tus shooters utilicen <strong>Raw Input (Entrada sin procesar)</strong>.
                Esta API lee directamente los paquetes hid del hardware mediante <code>WM_INPUT</code>, ignorando cualquier escalado del cursor o retraso del gestor de ventanas de Windows (DWM).
              </p>
              <div className="guide-alert-box">
                <strong>Verificación en Windows:</strong> Abre <code>main.cpl</code> (Propiedades del Mouse), ve a la pestaña <em>Opciones de Puntero</em> y <strong>desmarca</strong> la casilla <em>"Mejorar la precisión del puntero"</em>. Esta opción introduce aceleración no lineal parásita.
              </div>
            </div>

            <div className="guide-step-card">
              <h4>2. Tiempo de Rebote (Debounce Time)</h4>
              <p>
                El <em>Debounce Time</em> es el tiempo que el microcontrolador del ratón espera antes de validar un clic para evitar dobles clics accidentales por contacto metálico.
              </p>
              <ul className="guide-checklist">
                <li>
                  <strong>Switches Ópticos (Razer, Logitech Gen 2, Zowie):</strong> Puedes configurar el debounce en <strong>0 ms o 1 ms</strong> de forma completamente segura, ya que el haz de luz infrarroja no sufre de rebote mecánico.
                </li>
                <li>
                  <strong>Switches Mecánicos (Kailh, Omron):</strong> El valor ideal es entre <strong>2 ms y 4 ms</strong>. Bajarlo a 0 ms causará que el ratón dispare por accidente al apoyarlo con fuerza en la alfombrilla (*slam click*).
                </li>
              </ul>
            </div>
          </div>
        </div>
      )}

      {/* 4. GUÍA: BUFFERBLOAT Y ROUTER */}
      {activeCategory === "router" && (
        <div className="guide-content-panel panel">
          <div className="guide-header">
            <h3>Solución Definitiva de Red: Smart Queue Management (SQM)</h3>
            <span className="badge badge-accent">Click-to-Server Latency</span>
          </div>

          <div className="guide-box guide-box-highlight">
            <h4>¿Por qué cambiar DNS o poner IP fija no soluciona el lag?</h4>
            <p>
              Un mito popular es que cambiar los servidores DNS reduce el ping. La realidad técnica es que el DNS solo traduce el nombre del servidor a una dirección IP una única vez al conectarte. Durante la partida, todos los disparos viajan mediante paquetes directos UDP.
            </p>
            <p>
              La causa real del <strong>90% de los picos de ping y de disparos que no hacen daño</strong> es el <strong>Bufferbloat</strong>: tu módem/router doméstico almacena cientos de paquetes en búfer cuando alguien descarga o sube datos, retrasando tus paquetes del juego.
            </p>
          </div>

          <div className="guide-step-card">
            <h4>Cómo Activar SQM en tu Enrutador</h4>
            <ol className="sqm-steps-list">
              <li>
                <strong>Verifica tu Router:</strong> Accede a la interfaz web de tu router (habitualmente <code>192.168.1.1</code> o <code>192.168.0.1</code>).
              </li>
              <li>
                <strong>Busca la opción QoS o SQM:</strong> En routers ASUS (Adaptive QoS / Cake), Netgear (DumaOS), GL.iNet o cualquier router con firmware <em>OpenWrt</em>, busca la sección <strong>Smart Queue Management (SQM)</strong>.
              </li>
              <li>
                <strong>Algoritmo de Colas:</strong> Selecciona el algoritmo <strong>CAKE</strong> (o <strong>FQ-CoDel</strong> como alternativa).
              </li>
              <li>
                <strong>Regla del 90-95%:</strong> Configura el límite de velocidad de descarga y subida al <strong>90% - 95%</strong> de la velocidad real contratada con tu proveedor. Al dejar un 5% de margen, el router asume el control del búfer antes de que el módem del proveedor lo sature.
              </li>
            </ol>
          </div>
        </div>
      )}

      {/* 5. GUÍA: INTEGRIDAD DE MEMORIA (HVCI) Y VBS */}
      {activeCategory === "hvci" && (
        <div className="guide-content-panel panel">
          <div className="guide-header">
            <h3>Integridad de Memoria (HVCI) y Seguridad Basada en Virtualización (VBS)</h3>
            <span className="badge badge-accent">+5% a +25% en 1% Lows (Estabilidad)</span>
          </div>

          <div className="guide-box guide-box-highlight">
            <h4>¿Por qué la virtualización del kernel impacta el rendimiento de juegos?</h4>
            <p>
              En Windows 11, la <strong>Seguridad Basada en Virtualización (VBS)</strong> y la <strong>Integridad de Memoria (HVCI / Hypervisor-Protected Code Integrity)</strong> vienen habilitadas por defecto. Estas funciones aíslan el subsistema del kernel dentro de un hipervisor seguro (SLAT/EPT).
            </p>
            <p>
              <strong>Impacto empírico:</strong> Aunque los FPS promedio se mantienen similares, las constantes traducciones de tablas de páginas entre el hipervisor y el procesador provocan micro-pausas periódicas de la CPU. Pruebas de telemetría demuestran que desactivar HVCI proporciona una <strong>mejora del 5% al 25% en los percentiles 1% y 0.1% lows</strong>, erradicando micro-stutters durante escenarios de alta densidad de físicas y partículas.
            </p>
          </div>

          <div className="guide-grid-2col">
            <div className="guide-step-card">
              <h4>Pasos para Desactivar HVCI en Windows 11</h4>
              <ol className="sqm-steps-list">
                <li>
                  Abre el menú de inicio y escribe <strong>Seguridad de Windows</strong> (Windows Security).
                </li>
                <li>
                  Dirígete a la pestaña <strong>Seguridad del Dispositivo</strong> (Device Security).
                </li>
                <li>
                  En la sección <em>Aislamiento del núcleo</em>, haz clic en <strong>Detalles de aislamiento del núcleo</strong>.
                </li>
                <li>
                  Cambia el interruptor de <strong>Integridad de memoria</strong> a la posición <code>Desactivado (OFF)</code>.
                </li>
                <li>
                  <strong>Reinicia el sistema</strong> para que los cambios se apliquen a nivel de arranque del hipervisor.
                </li>
              </ol>
            </div>

            <div className="guide-step-card">
              <h4>Advertencia Crítica de Seguridad (Trade-off)</h4>
              <div className="guide-alert-box">
                <p>
                  <strong>¿Cuándo conviene desactivarlo?:</strong> Exclusivamente en <strong>equipos dedicados a gaming competitivo y eSports</strong> donde se busque la máxima estabilidad en frametimes y se practique navegación segura.
                </p>
                <p style={{ marginTop: "0.5rem" }}>
                  <strong>¿Cuándo NO desactivarlo?:</strong> Si utilizas este equipo para <strong>trabajo corporativo, transacciones bancarias críticas o manejo de software no confiable</strong>, mantén la Integridad de Memoria ACTIVADA. HVCI es una de las barreras más sólidas de Windows contra exploits de inyección de código a nivel de kernel y drivers maliciosos (Bring Your Own Vulnerable Driver).
                </p>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
