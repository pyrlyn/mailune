import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App";
import { ENGLISH, loadLanguage } from "./i18n";
import "./tokens.css";

// The dev server has no built worker, and a stale one would serve old code.
if (import.meta.env.PROD && "serviceWorker" in navigator) {
  void navigator.serviceWorker.register("/sw.js");
}

function render(language: string) {
  document.documentElement.lang = language;
  const root = document.getElementById("root");
  if (root) {
    createRoot(root).render(
      <StrictMode>
        <App />
      </StrictMode>,
    );
  }
}

// A catalog that fails to load (offline before it was cached) leaves the
// shell in English rather than blank.
loadLanguage(navigator.language).then(render, () => render(ENGLISH));
