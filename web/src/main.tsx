import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App";
import "./tokens.css";

if ("serviceWorker" in navigator) {
  void navigator.serviceWorker.register(new URL("./sw.ts", import.meta.url), { type: "module" });
}

const root = document.getElementById("root");
if (root) {
  createRoot(root).render(
    <StrictMode>
      <App />
    </StrictMode>,
  );
}
