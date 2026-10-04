import { NotificationWindow } from "@/components/NotificationWindow";
import { initSettingsStore } from "@/stores/settings";
import { useAppInfoStore } from "@/stores/appInfo";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "@fontsource-variable/inter";
import "@/lib/i18n";
import "@/styles/tokens.css";
import "@/styles/tailwind.css";
import "@/styles/base.css";
import { App } from "@/App";
import { bootstrap } from "@/bootstrap";

const isNotificationWindow = location.hash.startsWith("#/notification");
if (isNotificationWindow) document.documentElement.dataset.notificationWindow = "true";
if (isNotificationWindow)
  void Promise.all([initSettingsStore(), useAppInfoStore.getState().load()]);
else void bootstrap();

createRoot(document.getElementById("root")!).render(
  <StrictMode>{isNotificationWindow ? <NotificationWindow /> : <App />}</StrictMode>,
);
