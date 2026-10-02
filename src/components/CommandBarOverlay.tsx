import { useEffect, useRef } from "react";
import { createPortal } from "react-dom";
import { useTranslation } from "react-i18next";
import { useUiStore } from "@/stores/ui";
import { CommandBar } from "./command-bar/CommandBar";

/** Overlay do Ctrl+K: abre, foca e fecha (Esc ou clique fora) ou quando a barra abre Preview/Coleção. */
export function CommandBarOverlay() {
  const { t } = useTranslation();
  const open = useUiStore((s) => s.commandBarOpen);
  const close = useUiStore((s) => s.closeCommandBar);
  const previous = useRef<HTMLElement | null>(null);

  useEffect(() => {
    if (!open) return;
    previous.current = document.activeElement as HTMLElement | null;
    return () => previous.current?.focus?.();
  }, [open]);

  if (!open) return null;
  return createPortal(
    <div
      data-testid="command-bar-overlay"
      className="fixed inset-0 z-[90] flex items-start justify-center bg-[var(--overlay)] px-4 pt-[14vh] backdrop-blur-[6px]"
      onMouseDown={(e) => {
        if (e.target === e.currentTarget) close();
      }}
    >
      <div
        role="dialog"
        aria-modal="true"
        aria-label={t("commandBar.label")}
        data-command-bar-overlay
        className="glass-elevated max-h-[72vh] w-[min(640px,100%)] overflow-y-auto rounded-xl p-4 shadow-lg"
      >
        <CommandBar autoFocus overlay onDone={close} />
        <p className="mt-3 px-1 text-xs text-fg-muted">{t("commandBar.hint")}</p>
      </div>
    </div>,
    document.body,
  );
}
