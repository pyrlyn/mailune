import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App";
import "./tokens.css";

// The dev server has no built worker, and a stale one would serve old code.
if (import.meta.env.PROD && "serviceWorker" in navigator) {
  void navigator.serviceWorker.register("/sw.js");
}

const root = document.getElementById("root");
if (root) {
  createRoot(root).render(
    <StrictMode>
      <App />
    </StrictMode>,
  );
}
