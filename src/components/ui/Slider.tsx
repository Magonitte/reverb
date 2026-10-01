import { useId, type InputHTMLAttributes } from "react";
import { cn } from "./cn";

export interface SliderProps extends Omit<
  InputHTMLAttributes<HTMLInputElement>,
  "type" | "onChange" | "value"
> {
  label: string;
  value: number;
  onChange: (value: number) => void;
  min: number;
  max: number;
  step?: number;
  /** Como mostrar o valor ao lado do rótulo. */
  format?: (value: number) => string;
}

export function Slider({
  label,
  value,
  onChange,
  min,
  max,
  step = 1,
  format,
  className,
  id,
  ...rest
}: SliderProps) {
  const auto = useId();
  const inputId = id ?? auto;
  return (
    <div className={cn("flex flex-col gap-1", className)}>
      <div className="flex items-center justify-between text-xs">
        <label htmlFor={inputId} className="font-medium text-fg-secondary">
          {label}
        </label>
        <span className="font-mono text-fg-muted">{format ? format(value) : value}</span>
      </div>
      <input
        id={inputId}
        type="range"
        min={min}
        max={max}
        step={step}
        value={value}
        onChange={(e) => onChange(Number(e.target.value))}
        className="h-1.5 w-full cursor-pointer appearance-none rounded-full bg-track accent-accent disabled:opacity-50"
        {...rest}
      />
    </div>
  );
}
