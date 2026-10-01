import type { ButtonHTMLAttributes, ReactNode } from "react";
import { cn } from "./cn";

export interface IconButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  /** Obrigatório: texto acessível (via i18n) do botão só-ícone. */
  label: string;
  children: ReactNode;
  size?: "sm" | "md";
}

export function IconButton({
  label,
  children,
  size = "md",
  className,
  type = "button",
  ...rest
}: IconButtonProps) {
  return (
    <button
      type={type}
      aria-label={label}
      title={label}
      className={cn(
        "inline-flex items-center justify-center rounded-md border border-glass-border bg-field text-fg-muted transition-all duration-[140ms] ease-out hover:bg-field-hover hover:text-fg hover:border-glass-border-hover active:scale-95 disabled:opacity-50 disabled:pointer-events-none [&_svg]:size-4",
        size === "md" ? "size-9" : "size-7",
        className,
      )}
      {...rest}
    >
      {children}
    </button>
  );
}
