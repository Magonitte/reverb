import { useState, type KeyboardEvent } from "react";
import { Keyboard } from "lucide-react";
import { eventToAccelerator } from "@/lib/shortcuts";
import { cn } from "./cn";

export interface ShortcutRecorderProps {
  value: string;
  onChange: (accelerator: string) => void;
  label: string;
  /** Texto quando vazio e sem gravar. */
  emptyLabel: string;
  /** Texto enquanto espera uma combinação. */
  recordingLabel: string;
}

/** Grava uma combinação de teclas (ex.: "Ctrl+Shift+M"); Esc cancela, Backspace limpa. */
export function ShortcutRecorder({
  value,
  onChange,
  label,
  emptyLabel,
  recordingLabel,
}: ShortcutRecorderProps) {
  const [recording, setRecording] = useState(false);

  const onKeyDown = (event: KeyboardEvent<HTMLButtonElement>) => {
    if (!recording) return;
    event.preventDefault();
    event.stopPropagation();
    if (event.key === "Escape") {
      setRecording(false);
      return;
    }
    if (event.key === "Backspace" || event.key === "Delete") {
      onChange("");
      setRecording(false);
      return;
    }
    const accelerator = eventToAccelerator(event);
    if (accelerator) {
      onChange(accelerator);
      setRecording(false);
    }
  };

  return (
    <button
      type="button"
      aria-label={label}
      onClick={() => setRecording(true)}
      onBlur={() => setRecording(false)}
      onKeyDown={onKeyDown}
      className={cn(
        "inline-flex h-[38px] min-w-44 items-center gap-2 rounded-md border bg-field px-4 font-mono text-xs transition-all duration-[140ms]",
        recording
          ? "border-glass-border-focus text-accent shadow-[0_0_0_3px_var(--accent-muted)]"
          : "border-glass-border text-fg-secondary hover:border-glass-border-hover",
      )}
    >
      <Keyboard className="size-4 text-fg-muted" aria-hidden="true" />
      {recording ? recordingLabel : value || emptyLabel}
    </button>
  );
}
