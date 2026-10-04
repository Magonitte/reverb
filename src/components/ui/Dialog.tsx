import {
  useEffect,
  useId,
  useRef,
  type KeyboardEvent as ReactKeyboardEvent,
  type ReactNode,
} from "react";
import { createPortal } from "react-dom";
import { X } from "lucide-react";
import { cn } from "./cn";
import { IconButton } from "./IconButton";
import { useUiStore } from "@/stores/ui";

const FOCUSABLE =
  'a[href],button:not([disabled]),input:not([disabled]),select:not([disabled]),textarea:not([disabled]),[tabindex]:not([tabindex="-1"])';

export interface DialogProps {
  open: boolean;
  onClose: () => void;
  title: string;
  /** Texto do botão de fechar (i18n). */
  closeLabel: string;
  children: ReactNode;
  footer?: ReactNode;
  /** `side` = painel deslizante à direita (Sheet). */
  variant?: "center" | "side";
  className?: string;
}

/** Diálogo modal: prende o foco, fecha com Esc e devolve o foco a quem o abriu. */
export function Dialog({
  open,
  onClose,
  title,
  closeLabel,
  children,
  footer,
  variant = "center",
  className,
}: DialogProps) {
  const panelRef = useRef<HTMLDivElement>(null);
  const onCloseRef = useRef(onClose);
  const titleId = useId();
  useEffect(() => {
    if (!open) return;
    useUiStore.setState((state) => ({ modalCount: state.modalCount + 1 }));
    return () =>
      useUiStore.setState((state) => ({ modalCount: Math.max(0, state.modalCount - 1) }));
  }, [open]);

  useEffect(() => {
    onCloseRef.current = onClose;
  }, [onClose]);

  useEffect(() => {
    if (!open) return;
    const previous = document.activeElement as HTMLElement | null;
    const panel = panelRef.current;
    const first = panel?.querySelector<HTMLElement>("[data-autofocus]") ?? null;
    (first ?? panel?.querySelector<HTMLElement>(FOCUSABLE) ?? panel)?.focus();

    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.stopPropagation();
        onCloseRef.current();
      }
    };
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("keydown", onKey);
      previous?.focus?.();
    };
  }, [open]);

  if (!open) return null;

  const trapTab = (event: ReactKeyboardEvent<HTMLDivElement>) => {
    if (event.key !== "Tab") return;
    const items = Array.from(panelRef.current?.querySelectorAll<HTMLElement>(FOCUSABLE) ?? []);
    if (items.length === 0) {
      event.preventDefault();
      return;
    }
    const firstEl = items[0]!;
    const lastEl = items[items.length - 1]!;
    if (event.shiftKey && document.activeElement === firstEl) {
      event.preventDefault();
      lastEl.focus();
    } else if (!event.shiftKey && document.activeElement === lastEl) {
      event.preventDefault();
      firstEl.focus();
    }
  };

  const side = variant === "side";
  return createPortal(
    <div
      className={cn(
        "fixed inset-0 z-[100] flex bg-[var(--overlay)] backdrop-blur-[6px]",
        side ? "justify-end" : "items-center justify-center p-4",
      )}
      onMouseDown={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div
        ref={panelRef}
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
        tabIndex={-1}
        onKeyDown={trapTab}
        className={cn(
          "glass-elevated flex flex-col overflow-hidden shadow-lg",
          side
            ? "h-full w-[min(480px,100vw)] rounded-l-xl"
            : "max-h-[85vh] w-[min(600px,100%)] rounded-xl",
          className,
        )}
      >
        <div className="flex items-center gap-3 border-b border-glass-border px-6 py-4">
          <h2 id={titleId} className="flex-1 font-display text-base font-semibold">
            {title}
          </h2>
          <IconButton label={closeLabel} size="sm" onClick={onClose}>
            <X />
          </IconButton>
        </div>
        <div
          role="region"
          aria-label={title}
          tabIndex={0}
          className="flex-1 overflow-y-auto px-6 py-5"
        >
          {children}
        </div>
        {footer && (
          <div className="flex justify-end gap-2 border-t border-glass-border px-6 py-4">
            {footer}
          </div>
        )}
      </div>
    </div>,
    document.body,
  );
}
