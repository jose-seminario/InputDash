import { useEffect } from "react";
import { useTweaksStore } from "./store/useTweaksStore";
import { ABTestingView } from "./views/ABTestingView";
import { AntiMythsView } from "./views/AntiMythsView";
import { DashboardView } from "./views/DashboardView";
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

  return (
    <div className="app-container">
      <header className="app-header">
        <div className="app-title-group">
          <h1>InputDash</h1>
          <p>
            Optimizador y Medidor Científico de Input Lag · Windows 10/11 (100%
            APIs Nativas)
          </p>
        </div>

        <div className="header-controls">
          {simulationMode && (
            <button
              type="button"
              className="badge badge-warn badge-clickable"
              onClick={() => setSimulationMode(false)}
              title="Haz clic para volver al modo de ejecución real"
            >
              Modo Simulación ACTIVO (Clic para desactivar)
            </button>
          )}

          <button
            type="button"
            className="btn btn-secondary"
            disabled={isLoading || busyAction !== null}
            onClick={() => void refreshAll()}
          >
            {isLoading ? "Leyendo SO..." : "Actualizar estado"}
          </button>
        </div>
      </header>

      <nav className="nav-tabs" aria-label="Secciones principales">
        <button
          type="button"
          className={`nav-tab ${activeTab === "dashboard" ? "nav-tab-active" : ""}`}
          onClick={() => setActiveTab("dashboard")}
        >
          Dashboard
        </button>
        <button
          type="button"
          className={`nav-tab ${activeTab === "optimizar" ? "nav-tab-active" : ""}`}
          onClick={() => setActiveTab("optimizar")}
        >
          Optimizar
        </button>
        <button
          type="button"
          className={`nav-tab ${activeTab === "ab_testing" ? "nav-tab-active" : ""}`}
          onClick={() => setActiveTab("ab_testing")}
        >
          Prueba A/B
        </button>
        <button
          type="button"
          className={`nav-tab ${activeTab === "seguridad" ? "nav-tab-active" : ""}`}
          onClick={() => setActiveTab("seguridad")}
        >
          Seguridad ({backups.length})
        </button>
        <button
          type="button"
          className={`nav-tab ${activeTab === "antimitos" ? "nav-tab-active" : ""}`}
          onClick={() => setActiveTab("antimitos")}
        >
          Anti-mitos
        </button>
      </nav>

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
            Cerrar
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
            Cerrar
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
            Cerrar
          </button>
        </div>
      )}

      <main>
        {activeTab === "dashboard" && <DashboardView />}
        {activeTab === "optimizar" && <OptimizeView />}
        {activeTab === "ab_testing" && <ABTestingView />}
        {activeTab === "seguridad" && <SafetyView />}
        {activeTab === "antimitos" && <AntiMythsView />}
      </main>
    </div>
  );
}
