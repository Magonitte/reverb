import { Suspense } from "react";
import { Outlet } from "react-router";
import { CommandBarOverlay } from "@/components/CommandBarOverlay";
import { ToastHost } from "@/components/ui/Toast";
import { useGlobalShortcuts } from "@/hooks/useGlobalShortcuts";
import { BottomNav } from "./BottomNav";
import { ScreenOutlet } from "./ScreenOutlet";
import { Sidebar } from "./Sidebar";
import { Titlebar } from "./Titlebar";

/** Janela principal: Titlebar + Sidebar (ou BottomNav) + conteúdo da rota. */
export function ShellLayout() {
  useGlobalShortcuts();
  return (
    <div className="relative flex h-full flex-col overflow-hidden">
      <div
        aria-hidden="true"
        className="pointer-events-none absolute inset-0"
        style={{
          background:
            "radial-gradient(ellipse 70% 50% at 20% 15%, var(--groove-1) 0%, transparent 60%), radial-gradient(ellipse 50% 40% at 80% 85%, var(--groove-2) 0%, transparent 55%)",
        }}
      />
      <Titlebar />
      <div className="relative z-[1] flex min-h-0 flex-1 max-sm:flex-col">
        <Sidebar />
        <main
          data-testid="content"
          className="min-h-0 min-w-0 flex-1 overflow-y-auto overflow-x-hidden"
        >
          <ScreenOutlet />
        </main>
        <BottomNav />
      </div>
      <CommandBarOverlay />
      <ToastHost />
    </div>
  );
}

/** Janela sem navegação (onboarding): só a barra de título e a tela. */
export function BareLayout() {
  return (
    <div className="relative flex h-full flex-col overflow-hidden">
      <Titlebar />
      <main
        data-testid="screen"
        data-route="/onboarding"
        className="relative z-[1] min-h-0 flex-1 overflow-y-auto"
      >
        <Suspense fallback={<div data-testid="screen-loading" />}>
          <Outlet />
        </Suspense>
      </main>
      <ToastHost />
    </div>
  );
}
