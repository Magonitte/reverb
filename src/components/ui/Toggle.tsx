import type { ButtonHTMLAttributes } from "react";
import { cn } from "./cn";

export interface ToggleProps extends Omit<
  ButtonHTMLAttributes<HTMLButtonElement>,
  "onChange" | "role"
> {
  checked: boolean;
  onChange: (checked: boolean) => void;
  /** Texto acessível quando não há <label> associado. */
  label?: string;
}

export function Toggle({ checked, onChange, label, className, disabled, ...rest }: ToggleProps) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      aria-label={label}
      disabled={disabled}
      onClick={() => onChange(!checked)}
      className={cn(
        "relative h-6 w-[42px] shrink-0 rounded-full border transition-all duration-[140ms] ease-out disabled:opacity-50 disabled:pointer-events-none",
        checked ? "border-accent bg-accent" : "border-glass-border bg-track",
        className,
      )}
      {...rest}
    >
      <span
        aria-hidden="true"
        className={cn(
          "absolute left-[3px] top-[2.5px] size-[17px] rounded-full bg-white shadow-[0_1px_4px_rgba(0,0,0,0.3)] transition-transform duration-[140ms] ease-out",
          checked && "translate-x-[17px]",
        )}
      />
    </button>
  );
}
