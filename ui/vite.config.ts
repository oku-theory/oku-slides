import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// En développement hors Tauri, /api est servi par `oku-slides serve` (port 4870).
// Sans `changeOrigin` : le serveur exige un en-tête Host égal à l'Origin du navigateur.
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
    proxy: { "/api": "http://127.0.0.1:4870" },
  },
  build: { target: "es2022", outDir: "dist", emptyOutDir: true },
});
