import { useEffect } from "react";
import { AlertTriangle, CheckCircle2, Info, XCircle, X } from "lucide-react";
import { useTranslation } from "react-i18next";
import { useUiStore, type Toast as ToastData, type ToastTone } from "@/stores/ui";
import { cn } from "./cn";

export const TOAST_DURATION_MS = 4000;

const ICONS: Record<ToastTone, typeof Info> = {
  info: Info,
  success: CheckCircle2,
  warning: AlertTriangle,
  error: XCircle,
};

const TONE_CLASS: Record<ToastTone, string> = {
  info: "text-info",
  success: "text-success",
  warning: "text-warning",
  error: "text-error",
};

function ToastItem({ toast }: { toast: ToastData }) {
  const { t } = useTranslation();
  const dismiss = useUiStore((s) => s.dismissToast);
  const Icon = ICONS[toast.tone];

  useEffect(() => {
    const timer = setTimeout(() => dismiss(toast.id), TOAST_DURATION_MS);
    return () => clearTimeout(timer);
  }, [toast.id, dismiss]);

  return (
    <div
      role="status"
      className="glass-elevated pointer-events-auto flex max-w-[360px] items-start gap-3 rounded-lg px-4 py-3 shadow-lg [animation:reverb-toast-in_240ms_var(--ease-out)]"
    >
      <Icon className={cn("mt-0.5 size-4 shrink-0", TONE_CLASS[toast.tone])} aria-hidden="true" />
      <p className="flex-1 text-[13px] text-fg">{toast.message}</p>
      {toast.actionLabel && (
        <button
          type="button"
          className="text-xs font-semibold text-accent hover:text-accent-hover"
          onClick={() => {
            toast.onAction?.();
            dismiss(toast.id);
          }}
        >
          {toast.actionLabel}
        </button>
      )}
      {toast.secondaryActionLabel && (
        <button
          type="button"
          className="text-xs font-semibold text-accent hover:text-accent-hover"
          onClick={() => {
            toast.onSecondaryAction?.();
            dismiss(toast.id);
          }}
        >
          {toast.secondaryActionLabel}
        </button>
      )}
      <button
        type="button"
        aria-label={t("common.close")}
        className="text-fg-muted hover:text-fg"
        onClick={() => dismiss(toast.id)}
      >
        <X className="size-3.5" />
      </button>
    </div>
  );
}

/** Fila de toasts (região `aria-live`); cada um some após 4 s. */
export function ToastHost() {
  const toasts = useUiStore((s) => s.toasts);
  return (
    <div
      aria-live="polite"
      className="pointer-events-none fixed bottom-4 right-4 z-[200] flex flex-col gap-2 max-sm:bottom-[calc(var(--bottomnav-h)+16px)] max-sm:left-4"
    >
      {toasts.map((toast) => (
        <ToastItem key={toast.id} toast={toast} />
      ))}
    </div>
  );
}
