import type { HTMLAttributes } from "react";
import { cn } from "./cn";

export type BadgeTone = "accent" | "success" | "warning" | "error" | "info" | "neutral";

const TONES: Record<BadgeTone, string> = {
  accent: "bg-accent-muted text-accent",
  success: "bg-success/15 text-success",
  warning: "bg-warning/15 text-warning",
  error: "bg-error/15 text-error",
  info: "bg-info/15 text-info",
  neutral: "bg-field text-fg-muted",
};

export interface BadgeProps extends HTMLAttributes<HTMLSpanElement> {
  tone?: BadgeTone;
}

export function Badge({ tone = "accent", className, ...rest }: BadgeProps) {
  return (
    <span
      className={cn(
        "inline-flex min-w-[18px] items-center justify-center rounded-full px-1.5 text-[10px] font-semibold leading-[18px]",
        TONES[tone],
        className,
      )}
      {...rest}
    />
  );
}

export interface StatusDotProps {
  tone?: "success" | "warning" | "error" | "info" | "accent" | "idle";
  /** Texto acessível (i18n). */
  label: string;
  pulse?: boolean;
}

const DOT: Record<NonNullable<StatusDotProps["tone"]>, string> = {
  success: "bg-success",
  warning: "bg-warning",
  error: "bg-error",
  info: "bg-info",
  accent: "bg-accent",
  idle: "bg-fg-dim",
};

export function StatusDot({ tone = "idle", label, pulse }: StatusDotProps) {
  return (
    <span
      role="img"
      aria-label={label}
      className={cn("inline-block size-2 rounded-full", DOT[tone], pulse && "animate-pulse")}
    />
  );
}
