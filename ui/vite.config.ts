import { readFileSync } from "node:fs";
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

const root = new URL("../", import.meta.url);

/** The app version: `version` of `[workspace.package]` in the workspace Cargo.toml. */
function appVersion(): string {
  const cargo = readFileSync(new URL("Cargo.toml", root), "utf8");
  const section = cargo.split(/^\[workspace\.package\]$/m)[1]?.split(/^\[/m)[0] ?? "";
  const version = /^version\s*=\s*"([^"]+)"/m.exec(section)?.[1];
  if (!version) throw new Error("no [workspace.package] version in Cargo.toml");
  return version;
}

// En développement hors Tauri, /api est servi par `oku-slides serve` (port 4870).
// Sans `changeOrigin` : le serveur exige un en-tête Host égal à l'Origin du navigateur.
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  // Read by src/appInfo.ts: the About dialog's version and changelog.
  define: {
    __APP_VERSION__: JSON.stringify(appVersion()),
    __APP_CHANGELOG__: JSON.stringify(readFileSync(new URL("CHANGELOG.md", root), "utf8")),
  },
  server: {
    port: 5173,
    strictPort: true,
    proxy: { "/api": "http://127.0.0.1:4870" },
  },
  build: { target: "es2022", outDir: "dist", emptyOutDir: true },
});
