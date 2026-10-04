import type { ButtonHTMLAttributes, ReactNode } from "react";
import { Loader2 } from "lucide-react";
import { cn } from "./cn";

export type ButtonVariant = "primary" | "secondary" | "ghost" | "danger";
export type ButtonSize = "sm" | "md" | "lg";

const VARIANTS: Record<ButtonVariant, string> = {
  primary:
    "text-accent-on font-semibold bg-gradient-to-br from-accent-hover to-accent-pressed shadow-[0_2px_12px_var(--accent-glow)] hover:shadow-[0_4px_20px_var(--accent-glow)] hover:brightness-105",
  secondary:
    "bg-field text-fg-secondary border border-glass-border hover:bg-field-hover hover:text-fg hover:border-glass-border-hover",
  ghost: "bg-transparent text-fg-muted hover:bg-hover hover:text-fg",
  danger: "bg-error/15 text-error border border-error/30 hover:bg-error/25 hover:border-error/50",
};

const SIZES: Record<ButtonSize, string> = {
  sm: "h-8 px-3 text-xs rounded-sm",
  md: "h-10 px-4 text-sm rounded-md",
  lg: "h-[46px] px-6 text-sm rounded-lg",
};

export interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: ButtonVariant;
  size?: ButtonSize;
  loading?: boolean;
  icon?: ReactNode;
}

export function Button({
  variant = "secondary",
  size = "md",
  loading = false,
  icon,
  disabled,
  className,
  children,
  type = "button",
  ...rest
}: ButtonProps) {
  return (
    <button
      type={type}
      disabled={disabled || loading}
      aria-busy={loading || undefined}
      className={cn(
        "inline-flex items-center justify-center gap-2 font-medium whitespace-nowrap transition-all duration-[140ms] ease-out active:scale-[0.97] disabled:opacity-50 disabled:pointer-events-none",
        VARIANTS[variant],
        SIZES[size],
        className,
      )}
      {...rest}
    >
      {loading ? <Loader2 className="size-4 animate-spin" aria-hidden="true" /> : icon}
      {children}
    </button>
  );
}
