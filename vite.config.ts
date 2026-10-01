import { fileURLToPath, URL } from "node:url";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import { defineConfig } from "vite";

export default defineConfig({
  plugins: [react(), tailwindcss()],
  clearScreen: false,
  envPrefix: ["VITE_", "TAURI_ENV_"],
  resolve: {
    alias: { "@": fileURLToPath(new URL("./src", import.meta.url)) },
  },
  // Pré-empacota tudo na partida: sem isso o Vite redescobre dependências no meio dos testes E2E e recarrega a página.
  optimizeDeps: {
    include: [
      "react",
      "react-dom/client",
      "react-router",
      "react-i18next",
      "i18next",
      "zustand",
      "motion/react",
      "lucide-react",
      "@tanstack/react-virtual",
      "@fontsource-variable/inter",
    ],
  },
  server: { port: 1420, strictPort: true },
});
