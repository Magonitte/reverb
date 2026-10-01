import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "@/lib/i18n";
import "@/styles/tailwind.css";
import { App } from "@/App";
import { initSettingsStore } from "@/stores/settings";

void initSettingsStore();

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
