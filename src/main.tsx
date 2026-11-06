import React from "react";
import ReactDOM from "react-dom/client";
import { App } from "./App";
import "./styles.css";

// Interceptar comportamientos web de WebView2 para ofrecer experiencia 100% nativa
window.addEventListener("contextmenu", (e) => {
  const target = e.target as HTMLElement | null;
  const isInput =
    target?.tagName === "INPUT" ||
    target?.tagName === "TEXTAREA" ||
    target?.isContentEditable;
  if (!isInput) {
    e.preventDefault();
  }
});

window.addEventListener("keydown", (e) => {
  // Desactivar recargas y atajos de navegador web
  if (
    e.key === "F5" ||
    (e.ctrlKey &&
      (e.key === "r" ||
        e.key === "R" ||
        e.key === "f" ||
        e.key === "F" ||
        e.key === "p" ||
        e.key === "P" ||
        e.key === "u" ||
        e.key === "U" ||
        e.key === "+" ||
        e.key === "-" ||
        e.key === "=" ||
        e.key === "0"))
  ) {
    e.preventDefault();
  }
});

window.addEventListener("dragstart", (e) => {
  const target = e.target as HTMLElement | null;
  if (target?.tagName !== "INPUT" && target?.tagName !== "TEXTAREA") {
    e.preventDefault();
  }
});

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
);
