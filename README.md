# InputDash ⚡
> **Herramienta Científica de Diagnóstico, Medición y Optimización de Input Lag para Windows 10/11**

[![Tauri v2](https://img.shields.io/badge/Tauri-v2-blue.svg)](https://tauri.app/)
[![Rust](https://img.shields.io/badge/Rust-2021_Edition-orange.svg)](https://www.rust-lang.org/)
[![React](https://img.shields.io/badge/React-19-61dafb.svg)](https://react.dev/)
[![TypeScript](https://img.shields.io/badge/TypeScript-5.6-blue.svg)](https://www.typescriptlang.org/)
[![Anticheat Safe](https://img.shields.io/badge/Anticheat-100%25_Safe-success.svg)](#seguridad-anticheat-estricta)
[![License](https://img.shields.io/badge/License-MIT-green.svg)](LICENSE)

---

## 🎯 ¿Qué es InputDash?

**InputDash** es una plataforma nativa de escritorio diseñada para jugadores competitivos de shooters (*Counter-Strike 2, Valorant, Apex Legends, Warzone, Overwatch 2*), desarrolladores e investigadores de latencia en Windows 10 y 11.

A diferencia de los "optimizadores milagro" basados en scripts de PowerShell o manipulaciones destructivas del registro, **InputDash no promete magia: mide con instrumental científico**. Su objetivo es erradicar el infame fenómeno del **"disparé primero y el tiro no registró"**, atacando las dos fuentes reales de retraso parásito:
1. **La Cadena Local (*Click-to-Photon*)**: Acumulación de fotogramas en la cola de render (*Render Queue*), contención de bus USB entre ratón y audio, latencia DPC de controladores defectuosos y retrasos de temporizador del kernel.
2. **La Cadena de Red (*Click-to-Server*)**: Picos de retraso provocados por **Bufferbloat** (hinchazón de búfer en routers residenciales), fluctuación (*jitter*) en paquetes UDP y pérdida de sincronización de tickrate.

---

## 🛡️ Seguridad Anticheat Estricta (100% Kernel-Safe)

InputDash fue diseñado desde su primera línea de código respetando las políticas más rigurosas de los motores antitrampas modernos (**Riot Vanguard, Valve Anti-Cheat / VACNet, Easy Anti-Cheat, BattlEye y Call of Duty Ricochet**):

- **Cero Inyección de Código**: No utiliza `CreateRemoteThread`, `WriteProcessMemory` ni inyección de DLLs de ninguna clase.
- **Cero Lectura de Memoria**: No inspecciona ni lee la memoria virtual de los procesos de juego.
- **Cero Ganchos (No Hooks)**: No instala *API hooks* (`SetWindowsHookEx`) ni interceptores de llamadas gráficas.
- **Detección Limpia de Procesos**: La monitorización de juegos utiliza únicamente la API nativa de Windows `CreateToolhelp32Snapshot` con flags de privilegios mínimos (`PROCESS_QUERY_LIMITED_INFORMATION`).
- **Cero Procesos Externos en Backend**: Toda interacción con el sistema operativo se realiza directamente mediante llamadas tipadas a la API Win32 a través de `windows-rs`. Está estrictamente prohibido el uso de `std::process::Command`, `powershell.exe`, `cmd.exe` o ejecutables externos.
- **Seguridad Transaccional**: Todo tweak del sistema pasa por el pipeline obligatorio:
  $$\text{guard.check} \longrightarrow \text{backup.snapshot} \longrightarrow \text{restore\_point} \longrightarrow \text{apply} \longrightarrow \text{verify} \longrightarrow \text{log}$$

---

## 🔬 Funcionalidades Principales

### 1. Diagnóstico de Hardware y Pantalla
- **Detección Nativa de GPU**: Identificación de adaptador dedicado y memoria VRAM mediante interfaces DXGI nativas sin WMI lento.
- **Monitor y Tasa de Refresco**: Consulta de frecuencia vertical exacta en hercios y verificación de compatibilidad con **VRR (Variable Refresh Rate / G-Sync / FreeSync)** a través de `DisplayConfigGetDeviceInfo`.

### 2. Topología USB y Detección de Contención de Bus
- Mapea el árbol jerárquico de concentradores raíz USB con `SetupAPI` y `CfgMgr32`.
- Alerta al usuario si el ratón de alta tasa de sondeo comparte controlador con dispositivos de audio USB (transferencias isócronas continuas), causa principal de micro-tirones y *polling drops*.

### 3. Medición Científica de Polling Rate HID
- Captura ráfagas de paquetes de entrada cruda (`WM_INPUT`) en una ventana de 4 segundos.
- Mide con precisión de microsegundos mediante `QueryPerformanceCounter` (QPC), calculando tasa en Hz, intervalo medio, dispersión mín/máx y varianza (*jitter*).

### 4. Telemetría de Kernel ETW (Event Tracing for Windows)
- **Frame Times 3D**: Sesión en tiempo real con el proveedor `Microsoft-Windows-DxgKrnl` / `DXGI`. Registra cuadros durante 10 segundos calculando FPS promedio, 1% Low, 0.1% Low y diagnóstico de *Frame Pacing*.
- **Latencia DPC / ISR**: Rastreo del kernel mediante `NT Kernel Logger` (`EVENT_TRACE_FLAG_DPC | EVENT_TRACE_FLAG_INTERRUPT`). Identifica con precisión qué controlador (`.sys`) genera picos de interrupción diferida en los núcleos del procesador.
- **Guardas RAII**: Todas las sesiones ETW se abren y cierran mediante bloques RAII (`Drop`), garantizando que jamás queden sesiones colgadas en el sistema.

### 5. Total Engagement Latency (TEL) & Ventana de Ventaja
- Modela matemáticamente el retardo integral de la cadena:
  $$\text{TEL} = \text{Latencia USB} + \text{Frame Time} + \text{Scanout Monitor} + \text{One-Way Ping} + \frac{1}{2}\text{Tick Duration}$$
- Compara la ventana de ventaja competitiva (*Peeker's Advantage*) del usuario frente a oponentes con pings de 20 ms, 50 ms y 80 ms a tickrates de 64 y 128 ticks.

### 6. Motor de Experimentación A/B y Test Doble Ciego
- **Validación Científica Obligatoria**: Los tweaks avanzados no se pueden aplicar permanentemente sin una sesión A/B previa.
- **Test Doble Ciego**: Diana reactiva de 30 clics aleatorizados con el PRNG **SplitMix64**. El usuario no sabe si el tweak está activo en el Estado A o en el Estado B.
- **Análisis Estadístico**: Calcula pruebas de hipótesis rigurosas (*Welch's t-test*, prueba no paramétrica de *Mann-Whitney U* y tamaño del efecto con *Cohen's d*).
- **Importador Físico LDAT / Reflex Analyzer**: Permite ingresar telemetría medida por hardware externo para validación de latencia absoluta.

### 7. Perfiles por Juego, Topología de CPU y Temporizador Multimedia
- **Detección de Arquitectura de CPU**: Distingue automáticamente procesadores híbridos (Intel 12th+ Gen con P-Cores y E-Cores) y procesadores con memoria 3D V-Cache (AMD Ryzen X3D con CCD0 y CCD1).
- **Afinidad y Prioridad Limpias**: Asigna afinidad solo a núcleos de alto rendimiento y eleva la prioridad a `ABOVE_NORMAL_PRIORITY_CLASS`.
- **Temporizador de Alta Precisión RAII**: Ajusta la resolución del temporizador de Windows a **0.5 ms** mediante `NtSetTimerResolution` de forma exclusiva mientras el juego está abierto, restaurándolo inmediatamente al cerrarse.

### 8. Diagnóstico de Bufferbloat en Red
- Test en dos fases sin saturar la red con software pesado:
  - **Fase A (Reposo)**: Medición de RTT base mediante conexión TCP nativa (`TcpStream`).
  - **Fase B (Saturación)**: Descarga concurrente con 4 hilos secundarios midiendo la degradación del ping bajo carga.
- Calificación de Grado (A+ a F), cálculo de $\Delta\text{ ms}$ y recomendación de algoritmos **SQM (CAKE / FQ-CoDel)** para el router.

### 9. Guías Educativas In-Game (Solo Lectura)
- Guías paso a paso respaldadas por instrumental físico:
  - **NVIDIA Reflex & AMD Anti-Lag 2**: Eliminación de la cola de renderizado en escenarios GPU-bound.
  - **El Estándar Competitivo VRR**: G-Sync/FreeSync ON + V-Sync en Panel ON + V-Sync en Juego OFF + FPS Cap 3 cuadros bajo el refresco.
  - **Periféricos y Ratón**: Configuración de Raw Input y ajuste de tiempo de rebote (*Debounce Time*) según el tipo de switch (óptico vs mecánico).
  - **Mitigación de Bufferbloat**: Configuración de Smart Queue Management (SQM) en enrutadores domésticos con la regla del 90-95%.

### 10. Exportador de Reportes Multiformato
- Exporta diagnósticos completos a la carpeta `Documents\InputDash_Reports`:
  - **JSON Formateado**: Ideal para auditorías, benchmarks o análisis de datos.
  - **HTML Autocontenido**: Dashboard interactivo con tema oscuro, estilos CSS embebidos y tipografía legible sin dependencias de internet.

---

## ⚖️ Descargo de Responsabilidad (Física vs. Mito)

> **InputDash optimiza el software y la cola del hardware; no altera las leyes de la física.**

- **Latencia Parásita vs. Latencia Fisiológica**: InputDash reduce a cero el retardo añadido por configuraciones deficientes del sistema operativo y colas de controladores. Sin embargo, no puede reducir el tiempo biológico de reacción humano (~150 ms – 220 ms) ni la velocidad de propagación de la luz en la fibra óptica (~5 ms por cada 1000 km).
- **El Mito de los "Game Boosters"**: InputDash desmitifica prácticas nocivas como desactivar HPET a ciegas, borrar servicios esenciales de telemetría de Windows o cambiar arbitrariamente los servidores DNS (el tráfico de los juegos viaja por paquetes directos UDP; el DNS no afecta el ping in-game).

---

## 🛠️ Instrucciones de Compilación para Desarrolladores

### Prerrequisitos
1. **Windows 10 o Windows 11 (64-bit)**.
2. **Node.js** (v18 o superior) y **npm**.
3. **Rust Toolchain** (versión estable 1.78+ con target `x86_64-pc-windows-msvc`).
4. **Visual Studio C++ Build Tools** (con Windows 10/11 SDK instalado).

### Instalación de Dependencias
```powershell
# Clonar el repositorio
git clone https://github.com/jose-seminario/InputDash.git
cd InputDash

# Instalar dependencias del frontend (React 19 + TypeScript + Zustand)
npm install
```

### Ejecución en Modo Desarrollo
```powershell
# Levanta la interfaz de Vite y compila el backend de Rust en caliente
npm run tauri dev
```

### Compilación del Ejecutable de Producción
```powershell
# Genera el binario portable release optimizado en src-tauri/target/release/
npm run tauri build
```

---

## 📄 Licencia

Distribuido bajo la Licencia **MIT**. Consulta el archivo `LICENSE` para más información.
Desarrollado con rigor de ingeniería de sistemas para la comunidad competitiva de esports.
