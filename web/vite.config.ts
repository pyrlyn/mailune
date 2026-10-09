/// <reference types="vitest/config" />
import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

export default defineConfig({
  plugins: [react()],
  // The contract schema, the sanitised-HTML fixture and the generated
  // catalogs live in the Rust workspace one level up.
  server: { fs: { allow: [".."] } },
  build: {
    rolldownOptions: {
      input: { main: "index.html", sw: "src/sw.ts" },
      // The worker keeps a fixed name at the root so its scope is the whole app.
      output: { entryFileNames: (chunk) => (chunk.name === "sw" ? "sw.js" : "assets/[name]-[hash].js") },
    },
  },
  test: {
    environment: "node",
    // Component tests run offline: happy-dom must not load a frame, script
    // or style sheet even if a test hands it a URL.
    environmentOptions: {
      happyDOM: {
        settings: {
          disableIframePageLoading: true,
          disableJavaScriptFileLoading: true,
          disableCSSFileLoading: true,
          disableJavaScriptEvaluation: true,
        },
      },
    },
    include: ["src/**/*.test.{ts,tsx}"],
  },
});
