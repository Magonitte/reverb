import { HashRouter } from "react-router";
import { useAppearance } from "@/hooks/useAppearance";
import { AppRoutes } from "@/routes";

export function App() {
  useAppearance();
  return (
    <HashRouter>
      <AppRoutes />
    </HashRouter>
  );
}
