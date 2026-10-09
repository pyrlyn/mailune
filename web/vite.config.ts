/// <reference types="vitest/config" />
import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

export default defineConfig({
  plugins: [react()],
  // The contract schema, the sanitised-HTML fixture and the generated
  // catalogs live in the Rust workspace one level up.
  server: { fs: { allow: [".."] } },
  test: {
    environment: "node",
    include: ["src/**/*.test.{ts,tsx}"],
  },
});
