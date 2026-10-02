import { HashRouter, useLocation, Navigate } from "react-router";
import { useAppearance } from "@/hooks/useAppearance";
import { AppRoutes } from "@/routes";
import { useSettingsStore } from "@/stores/settings";

function FirstRunRoutes() {
  const settings = useSettingsStore((state) => state.settings);
  const location = useLocation();
  if (!settings) return null;
  if (!settings.onboardingCompleted && location.pathname !== "/onboarding")
    return <Navigate to="/onboarding" replace />;
  return <AppRoutes />;
}

export function App() {
  useAppearance();
  return (
    <HashRouter>
      <FirstRunRoutes />
    </HashRouter>
  );
}
