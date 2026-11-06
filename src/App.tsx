import { useEffect } from "react";
import { useTweaksStore } from "./store/useTweaksStore";
import { ABTestingView } from "./views/ABTestingView";
import { AntiMythsView } from "./views/AntiMythsView";
import { DashboardView } from "./views/DashboardView";
import { GameProfilesView } from "./views/GameProfilesView";
import { InGameGuidesView } from "./views/InGameGuidesView";
import { OptimizeView } from "./views/OptimizeView";
import { SafetyView } from "./views/SafetyView";

export function App() {
  const {
    activeTab,
    simulationMode,
    backups,
    isLoading,
    busyAction,
    errorBanner,
    warningMessage,
    successMessage,
    setActiveTab,
    setSimulationMode,
    refreshAll,
    dismissMessages,
  } = useTweaksStore();

  useEffect(() => {
    void refreshAll();
  }, [refreshAll]);

  const getSectionTitle = () => {
    switch (activeTab) {
      case "dashboard":
        return "PANEL DE CONTROL // TELEMETRÍA";
      case "optimizar":
        return "CATÁLOGO DE OPTIMIZACIÓN // REGISTRO";
      case "ab_testing":
        return "LABORATORIO DE PRUEBA A/B // DOBLE CIEGO";
      case "perfiles":
        return "GESTOR DE PERFILES // AISLAMIENTO DE NÚCLEOS";
      case "guias":
        return "MANUAL TÁCTICO // GUÍAS IN-GAME";
      case "seguridad":
        return `PUNTOS DE RESTAURACIÓN // BACKUPS (${backups.length})`;
      case "antimitos":
        return "CENTRO EDUCATIVO // PROTECCIONES Y MITOS";
      default:
        return "SISTEMA TÁCTICO INPUTDASH";
    }
  };

  return (
    <div className="app-layout">
      {/* SIDEBAR TÁCTICO CYBERPUNK */}
      <aside className="app-sidebar">
        <div className="sidebar-brand-box">
          <div className="brand-badge-row">
            <span className="brand-cyber-pill">SYS.HUD // v1.0</span>
            <span className="brand-pulse-dot" title="Telemetría en tiempo real activa"></span>
          </div>
          <h1 className="brand-title">INPUTDASH</h1>
          <p className="brand-tagline">
            OPT. INPUT LAG · WIN 10/11
            <br />
            100% APIS NATIVAS DE KERNEL
          </p>
        </div>

        <nav className="sidebar-nav" aria-label="Secciones tácticas">
          <button
            type="button"
            className={`nav-tab ${activeTab === "dashboard" ? "nav-tab-active" : ""}`}
            onClick={() => setActiveTab("dashboard")}
          >
            <span className="nav-tab-icon">⚡</span>
            <span className="nav-tab-text">Dashboard</span>
          </button>
          <button
            type="button"
            className={`nav-tab ${activeTab === "optimizar" ? "nav-tab-active" : ""}`}
            onClick={() => setActiveTab("optimizar")}
          >
            <span className="nav-tab-icon">🛠️</span>
            <span className="nav-tab-text">Optimizar</span>
          </button>
          <button
            type="button"
            className={`nav-tab ${activeTab === "ab_testing" ? "nav-tab-active" : ""}`}
            onClick={() => setActiveTab("ab_testing")}
          >
            <span className="nav-tab-icon">🧪</span>
            <span className="nav-tab-text">Prueba A/B</span>
          </button>
          <button
            type="button"
            className={`nav-tab ${activeTab === "perfiles" ? "nav-tab-active" : ""}`}
            onClick={() => setActiveTab("perfiles")}
          >
            <span className="nav-tab-icon">🎯</span>
            <span className="nav-tab-text">Perfiles por Juego</span>
          </button>
          <button
            type="button"
            className={`nav-tab ${activeTab === "guias" ? "nav-tab-active" : ""}`}
            onClick={() => setActiveTab("guias")}
          >
            <span className="nav-tab-icon">📖</span>
            <span className="nav-tab-text">Guías In-Game</span>
          </button>
          <button
            type="button"
            className={`nav-tab ${activeTab === "seguridad" ? "nav-tab-active" : ""}`}
            onClick={() => setActiveTab("seguridad")}
          >
            <span className="nav-tab-icon">🛡️</span>
            <span className="nav-tab-text">Seguridad ({backups.length})</span>
          </button>
          <button
            type="button"
            className={`nav-tab ${activeTab === "antimitos" ? "nav-tab-active" : ""}`}
            onClick={() => setActiveTab("antimitos")}
          >
            <span className="nav-tab-icon">🚫</span>
            <span className="nav-tab-text">Anti-mitos</span>
          </button>
        </nav>

        <div className="sidebar-footer-box">
          {simulationMode && (
            <button
              type="button"
              className="badge badge-warn badge-clickable simulation-sidebar-btn"
              onClick={() => setSimulationMode(false)}
              title="Haz clic para volver al modo de ejecución real"
            >
              ⚠️ SIMULACIÓN ACTIVA (DESACTIVAR)
            </button>
          )}

          <button
            type="button"
            className="btn btn-secondary sidebar-action-btn"
            disabled={isLoading || busyAction !== null}
            onClick={() => void refreshAll()}
          >
            {isLoading ? "LEYENDO SO..." : "ACTUALIZAR ESTADO"}
          </button>

          <div className="sidebar-sys-status">
            <span>ESTADO: SEGURO</span>
            <span className="text-cyan">KERNEL // OK</span>
          </div>
        </div>
      </aside>

      {/* ÁREA PRINCIPAL HUD */}
      <div className="app-main-viewport">
        {/* TOP STATUS BAR HUD */}
        <header className="hud-top-bar">
          <div className="hud-section-label">
            <span className="hud-label-prefix">// MOD_ACTIVO //</span>
            <h2>{getSectionTitle()}</h2>
          </div>

          <div className="header-controls">
            {simulationMode && (
              <button
                type="button"
                className="badge badge-warn badge-clickable"
                onClick={() => setSimulationMode(false)}
                title="Haz clic para volver al modo de ejecución real"
              >
                MODO SIMULACIÓN ACTIVO
              </button>
            )}

            <button
              type="button"
              className="btn btn-secondary"
              disabled={isLoading || busyAction !== null}
              onClick={() => void refreshAll()}
            >
              {isLoading ? "LEYENDO SO..." : "ACTUALIZAR ESTADO"}
            </button>
          </div>
        </header>

        {/* NOTIFICACIONES Y BANNERS TÁCTICOS */}
        {errorBanner && (
          <div className="banner banner-error" role="alert">
            <div>
              <span className="banner-code">[{errorBanner.code}]</span>
              <span>{errorBanner.message}</span>
            </div>
            <button
              type="button"
              className="banner-close"
              onClick={dismissMessages}
            >
              CERRAR
            </button>
          </div>
        )}

        {warningMessage && (
          <div className="banner banner-warning" role="alert">
            <span>{warningMessage}</span>
            <button
              type="button"
              className="banner-close"
              onClick={dismissMessages}
            >
              CERRAR
            </button>
          </div>
        )}

        {successMessage && (
          <div className="banner banner-success" role="status">
            <span>{successMessage}</span>
            <button
              type="button"
              className="banner-close"
              onClick={dismissMessages}
            >
              CERRAR
            </button>
          </div>
        )}

        {/* CONTENEDOR DE VISTAS */}
        <main className="app-content-stage">
          {activeTab === "dashboard" && <DashboardView />}
          {activeTab === "optimizar" && <OptimizeView />}
          {activeTab === "ab_testing" && <ABTestingView />}
          {activeTab === "perfiles" && <GameProfilesView />}
          {activeTab === "guias" && <InGameGuidesView />}
          {activeTab === "seguridad" && <SafetyView />}
          {activeTab === "antimitos" && <AntiMythsView />}
        </main>
      </div>
    </div>
  );
}
