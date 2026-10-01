import { useId, type ReactNode, type SelectHTMLAttributes } from "react";
import { ChevronDown } from "lucide-react";
import { cn } from "./cn";

export interface SelectOption {
  value: string;
  label: string;
}

export interface SelectProps extends Omit<SelectHTMLAttributes<HTMLSelectElement>, "children"> {
  label?: string;
  options: SelectOption[];
  hint?: ReactNode;
}

export function Select({ label, options, hint, className, id, ...rest }: SelectProps) {
  const auto = useId();
  const selectId = id ?? auto;
  return (
    <div className="flex flex-col gap-1">
      {label && (
        <label htmlFor={selectId} className="text-xs font-medium text-fg-secondary">
          {label}
        </label>
      )}
      <div className="relative">
        <select
          id={selectId}
          className={cn(
            "h-[38px] w-full appearance-none rounded-md border border-glass-border bg-field pl-4 pr-9 text-fg transition-all duration-[140ms] hover:border-glass-border-hover focus:border-glass-border-focus disabled:opacity-50 disabled:pointer-events-none [&>option]:bg-bg-base [&>option]:text-fg",
            className,
          )}
          {...rest}
        >
          {options.map((o) => (
            <option key={o.value} value={o.value}>
              {o.label}
            </option>
          ))}
        </select>
        <ChevronDown
          className="pointer-events-none absolute right-3 top-1/2 size-4 -translate-y-1/2 text-fg-muted"
          aria-hidden="true"
        />
      </div>
      {hint && <p className="text-xs text-fg-muted">{hint}</p>}
    </div>
  );
}
