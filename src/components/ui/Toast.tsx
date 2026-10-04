import { useEffect, useRef, useState } from "react";
import { AlertTriangle, CheckCircle2, Info, XCircle, X } from "lucide-react";
import { useTranslation } from "react-i18next";
import { useUiStore, type Toast as ToastData, type ToastTone } from "@/stores/ui";
import { playNotificationSound } from "@/lib/notificationSound";
import { Button } from "./Button";
import { cn } from "./cn";

export const TOAST_DURATION_MS = 8000;
const ICONS: Record<ToastTone, typeof Info> = {
  info: Info,
  success: CheckCircle2,
  warning: AlertTriangle,
  error: XCircle,
};
const TONE_CLASS: Record<ToastTone, string> = {
  info: "text-accent",
  success: "text-success",
  warning: "text-warning",
  error: "text-error",
};

function ToastItem({ toast }: { toast: ToastData }) {
  const { t } = useTranslation();
  const dismiss = useUiStore((s) => s.dismissToast);
  const modalOpen = useUiStore((s) => s.modalCount > 0);
  const Icon = ICONS[toast.tone];
  const duration = toast.durationMs ?? (toast.actionLabel ? 20000 : TOAST_DURATION_MS);
  const [remaining, setRemaining] = useState(duration);
  const [hovered, setHovered] = useState(false);
  const [focused, setFocused] = useState(false);
  const [busy, setBusy] = useState(false);
  const [hidden, setHidden] = useState(document.hidden);
  const acted = useRef(false);
  const sounded = useRef(false);
  const paused = hovered || focused || hidden || busy || modalOpen;
  useEffect(() => {
    if (toast.sound && !sounded.current) {
      sounded.current = true;
      void playNotificationSound().catch(() => {});
    }
  }, [toast.sound]);
  useEffect(() => {
    const changed = () => setHidden(document.hidden);
    document.addEventListener("visibilitychange", changed);
    return () => document.removeEventListener("visibilitychange", changed);
  }, []);
  useEffect(() => {
    if (paused) return;
    let previous = Date.now();
    const timer = setInterval(() => {
      const now = Date.now();
      const elapsed = now - previous;
      previous = now;
      setRemaining((value) => Math.max(0, value - elapsed));
    }, 100);
    return () => clearInterval(timer);
  }, [paused]);
  useEffect(() => {
    if (remaining === 0) dismiss(toast.id);
  }, [remaining, dismiss, toast.id]);
  async function act() {
    if (acted.current) return;
    acted.current = true;
    setBusy(true);
    try {
      await toast.onAction?.();
      dismiss(toast.id);
    } catch {
      acted.current = false;
      setBusy(false);
      useUiStore.getState().pushToast({ message: t("notifications.actionFailed"), tone: "error" });
    }
  }
  return (
    <div
      role={toast.tone === "error" ? "alert" : "status"}
      data-testid="toast"
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
      onFocusCapture={() => setFocused(true)}
      onBlurCapture={(e) => {
        if (!e.currentTarget.contains(e.relatedTarget)) setFocused(false);
      }}
      className="glass-elevated pointer-events-auto relative w-[420px] max-w-full shrink-0 overflow-hidden rounded-xl border border-glass-border p-4 shadow-lg [animation:reverb-toast-in_240ms_var(--ease-out)]"
    >
      <div className="flex items-start gap-3">
        <div className={cn("rounded-lg bg-field p-2", TONE_CLASS[toast.tone])}>
          <Icon className="size-5" aria-hidden="true" />
        </div>
        <div className="min-w-0 flex-1">
          <p className="text-sm font-semibold text-fg">
            {toast.title ?? t(`notifications.titles.${toast.tone}`)}
          </p>
          <p className="mt-1 whitespace-pre-line text-sm leading-relaxed text-fg-secondary">
            {toast.message}
          </p>
          {toast.details && (
            <p className="mt-2 break-all rounded-md bg-field px-2 py-1.5 text-xs leading-relaxed text-fg-muted">
              {toast.details}
            </p>
          )}
        </div>
        <button
          type="button"
          aria-label={t("common.close")}
          disabled={busy}
          className="flex size-8 shrink-0 items-center justify-center rounded-md text-fg-muted hover:bg-hover hover:text-fg"
          onClick={() => dismiss(toast.id)}
        >
          <X className="size-4" />
        </button>
      </div>
      {(toast.actionLabel || toast.secondaryActionLabel) && (
        <div className="mt-4 flex flex-wrap gap-2 pl-12">
          {toast.actionLabel && (
            <Button size="sm" variant="primary" loading={busy} onClick={() => void act()}>
              {toast.actionLabel}
            </Button>
          )}
          {toast.secondaryActionLabel && (
            <Button
              size="sm"
              disabled={busy}
              onClick={() => {
                toast.onSecondaryAction?.();
                dismiss(toast.id);
              }}
            >
              {toast.secondaryActionLabel}
            </Button>
          )}
        </div>
      )}
      <p aria-hidden="true" className="mt-3 text-right text-[11px] text-fg-muted">
        {t(paused ? "notifications.paused" : "notifications.remaining", {
          count: Math.ceil(remaining / 1000),
        })}
      </p>
      <div aria-hidden="true" className="absolute inset-x-0 bottom-0 h-1 bg-field">
        <div
          data-testid="toast-timer"
          className="h-full origin-left bg-accent"
          style={{ transform: `scaleX(${remaining / duration})` }}
        />
      </div>
    </div>
  );
}

export function ToastHost({ floatingWindow = false }: { floatingWindow?: boolean }) {
  const { t } = useTranslation();
  const toasts = useUiStore((s) => s.toasts);
  const modalOpen = useUiStore((s) => s.modalCount > 0);
  return (
    <div
      role="region"
      aria-live="polite"
      aria-label={t("notifications.regionTitle")}
      aria-hidden={modalOpen || undefined}
      inert={modalOpen}
      style={{ visibility: modalOpen ? "hidden" : undefined }}
      className={cn(
        "pointer-events-none z-[200] flex max-h-[calc(100dvh-32px)] max-w-[calc(100vw-32px)] flex-col gap-3 overflow-y-auto",
        floatingWindow
          ? "relative"
          : "fixed bottom-4 right-4 max-sm:bottom-[calc(var(--bottomnav-h)+16px)] max-sm:left-4",
      )}
    >
      {toasts.map((toast) => (
        <ToastItem key={toast.id} toast={toast} />
      ))}
    </div>
  );
}
