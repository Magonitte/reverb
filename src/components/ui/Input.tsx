import {
  forwardRef,
  useId,
  type InputHTMLAttributes,
  type ReactNode,
  type TextareaHTMLAttributes,
} from "react";
import { Search } from "lucide-react";
import { cn } from "./cn";

const FIELD =
  "w-full rounded-md border border-glass-border bg-field text-fg placeholder:text-fg-dim transition-all duration-[140ms] ease-out hover:bg-field-hover hover:border-glass-border-hover focus:border-glass-border-focus focus:bg-field-hover focus:shadow-[0_0_0_3px_var(--accent-muted)] disabled:opacity-50 disabled:pointer-events-none aria-[invalid=true]:border-error";

export interface InputProps extends InputHTMLAttributes<HTMLInputElement> {
  label?: string;
  hint?: ReactNode;
  error?: string;
  large?: boolean;
}

export const Input = forwardRef<HTMLInputElement, InputProps>(function Input(
  { label, hint, error, large, className, id, ...rest },
  ref,
) {
  const auto = useId();
  const inputId = id ?? auto;
  const describedBy = error ? `${inputId}-err` : hint ? `${inputId}-hint` : undefined;
  return (
    <div className="flex flex-col gap-1">
      {label && (
        <label htmlFor={inputId} className="text-xs font-medium text-fg-secondary">
          {label}
        </label>
      )}
      <input
        ref={ref}
        id={inputId}
        aria-invalid={error ? true : undefined}
        aria-describedby={describedBy}
        className={cn(
          FIELD,
          large ? "h-[52px] rounded-lg px-6 text-sm" : "h-[38px] px-4",
          className,
        )}
        {...rest}
      />
      {error ? (
        <p id={`${inputId}-err`} className="text-xs text-error">
          {error}
        </p>
      ) : (
        hint && (
          <p id={`${inputId}-hint`} className="text-xs text-fg-muted">
            {hint}
          </p>
        )
      )}
    </div>
  );
});

export interface SearchInputProps extends Omit<InputHTMLAttributes<HTMLInputElement>, "type"> {
  large?: boolean;
  wrapperClassName?: string;
}

export const SearchInput = forwardRef<HTMLInputElement, SearchInputProps>(function SearchInput(
  { large, className, wrapperClassName, ...rest },
  ref,
) {
  return (
    <div className={cn("relative", wrapperClassName)}>
      <Search
        className={cn(
          "pointer-events-none absolute top-1/2 -translate-y-1/2 text-fg-dim",
          large ? "left-5 size-5" : "left-3 size-4",
        )}
        aria-hidden="true"
      />
      <input
        ref={ref}
        type="text"
        className={cn(
          FIELD,
          large ? "h-[52px] rounded-lg pl-13 pr-6 text-sm" : "h-[38px] pl-9 pr-4",
          className,
        )}
        {...rest}
      />
    </div>
  );
});

export function Textarea({ className, ...rest }: TextareaHTMLAttributes<HTMLTextAreaElement>) {
  return (
    <textarea
      className={cn(FIELD, "min-h-[120px] resize-y p-4 font-mono text-xs leading-7", className)}
      {...rest}
    />
  );
}
