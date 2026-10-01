import { useId, type InputHTMLAttributes, type ReactNode } from "react";
import { Check } from "lucide-react";
import { cn } from "./cn";

export interface CheckboxProps extends Omit<
  InputHTMLAttributes<HTMLInputElement>,
  "type" | "onChange"
> {
  checked: boolean;
  onChange: (checked: boolean) => void;
  children?: ReactNode;
  /** Texto acessível quando não há conteúdo visível. */
  label?: string;
}

export function Checkbox({
  checked,
  onChange,
  children,
  label,
  className,
  id,
  disabled,
  ...rest
}: CheckboxProps) {
  const auto = useId();
  const inputId = id ?? auto;
  return (
    <label
      htmlFor={inputId}
      className={cn(
        "inline-flex items-center gap-2 rounded-md px-1 py-1 text-[13px] text-fg-secondary",
        disabled ? "opacity-50" : "cursor-pointer hover:bg-hover",
        className,
      )}
    >
      <span className="relative inline-flex size-[18px] shrink-0">
        <input
          id={inputId}
          type="checkbox"
          checked={checked}
          disabled={disabled}
          aria-label={children ? undefined : label}
          onChange={(e) => onChange(e.target.checked)}
          className="peer absolute inset-0 m-0 size-full cursor-pointer appearance-none rounded-sm border border-glass-border-hover bg-field checked:border-accent checked:bg-accent disabled:cursor-not-allowed"
          {...rest}
        />
        <Check
          aria-hidden="true"
          strokeWidth={3}
          className="pointer-events-none absolute inset-[3px] size-3 text-accent-on opacity-0 peer-checked:opacity-100"
        />
      </span>
      {children}
    </label>
  );
}
