import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "@fontsource-variable/inter";
import "@/lib/i18n";
import "@/styles/tokens.css";
import "@/styles/tailwind.css";
import "@/styles/base.css";
import { App } from "@/App";
import { bootstrap } from "@/bootstrap";

void bootstrap();

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
