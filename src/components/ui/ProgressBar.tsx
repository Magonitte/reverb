import { cn } from "./cn";

export type ProgressTone = "download" | "convert" | "done" | "error";

const FILL: Record<ProgressTone, string> = {
  download: "var(--stage-download)",
  convert: "var(--stage-convert)",
  done: "var(--stage-done)",
  error: "var(--stage-error)",
};

export interface ProgressBarProps {
  /** 0–1. `undefined` = indeterminado. */
  value?: number;
  tone?: ProgressTone;
  label: string;
  className?: string;
}

export function ProgressBar({ value, tone = "download", label, className }: ProgressBarProps) {
  const determinate = value !== undefined;
  const pct = determinate ? Math.round(Math.min(1, Math.max(0, value)) * 100) : undefined;
  return (
    <div
      role="progressbar"
      aria-label={label}
      aria-valuemin={0}
      aria-valuemax={100}
      aria-valuenow={pct}
      className={cn("h-1 w-full overflow-hidden rounded-sm bg-track", className)}
    >
      <div
        className={cn(
          "h-full rounded-sm shadow-[0_0_8px_var(--accent-glow)]",
          determinate ? "transition-[width] duration-300 ease-linear" : "w-[35%]",
        )}
        style={{
          width: determinate ? `${pct}%` : undefined,
          background: FILL[tone],
          animation: determinate
            ? undefined
            : "reverb-indeterminate 1.6s var(--ease-in-out) infinite",
        }}
      />
    </div>
  );
}
