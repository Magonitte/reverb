import { render } from "@testing-library/react";
import { MemoryRouter, useLocation } from "react-router";
import {
  mockBus,
  resetMockMedia,
  resetMockQueue,
  resetMockSettings,
  resetMockTools,
  resetMockUpdater,
} from "@/lib/ipc/mock";
import { AppRoutes } from "@/routes";
import { useFlowStore } from "@/stores/flow";
import { useHealStore } from "@/stores/heal";
import { useJobsStore } from "@/stores/jobs";
import { resetLibraryStore } from "@/stores/library";
import { resetMockLibrary } from "@/lib/ipc/mock/library";
import { useSettingsStore } from "@/stores/settings";
import { useToolsStore } from "@/stores/tools";
import { useUiStore } from "@/stores/ui";
import { initialUpdaterState, useUpdaterStore } from "@/stores/updater";

function Where() {
  const { pathname } = useLocation();
  return <output data-testid="where">{pathname}</output>;
}

/** App completo (shell + rotas) sobre o backend falso. */
export function renderApp(path = "/") {
  return render(
    <MemoryRouter initialEntries={[path]}>
      <AppRoutes />
      <Where />
    </MemoryRouter>,
  );
}

/** Zera o backend falso e todas as stores entre um teste e outro. */
export function resetFlowTests(): void {
  resetMockLibrary();
  resetLibraryStore();
  resetMockQueue();
  resetMockSettings();
  resetMockTools();
  resetMockUpdater();
  resetMockMedia();
  mockBus.clear();
  useJobsStore.setState({
    jobs: {},
    queue: { paused: false, running: 0, queued: 0, healing: false },
  });
  useToolsStore.setState({ statuses: [], progress: {} });
  useHealStore.setState({ stage: null });
  useUpdaterStore.setState(initialUpdaterState);
  useFlowStore.setState({ preview: null, collection: null });
  useUiStore.setState({
    commandBarOpen: false,
    commandBarText: "",
    commandBarFocusTick: 0,
    commandBarSubmitTick: 0,
    toasts: [],
  });
  useSettingsStore.setState({ settings: null });
}

/** jsdom não calcula layout: dá altura/largura às listas virtualizadas. */
export function stubLayout(): () => void {
  const height = Object.getOwnPropertyDescriptor(HTMLElement.prototype, "offsetHeight");
  const width = Object.getOwnPropertyDescriptor(HTMLElement.prototype, "offsetWidth");
  Object.defineProperty(HTMLElement.prototype, "offsetHeight", { configurable: true, value: 600 });
  Object.defineProperty(HTMLElement.prototype, "offsetWidth", { configurable: true, value: 800 });
  return () => {
    if (height) Object.defineProperty(HTMLElement.prototype, "offsetHeight", height);
    if (width) Object.defineProperty(HTMLElement.prototype, "offsetWidth", width);
  };
}
