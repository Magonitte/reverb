import type { ButtonHTMLAttributes, ReactNode } from "react";
import { cn } from "./cn";

export interface ChipProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  selected?: boolean;
  children: ReactNode;
}

/** Chip selecionável (perfil de saída, filtro). */
export function Chip({
  selected = false,
  className,
  children,
  type = "button",
  ...rest
}: ChipProps) {
  return (
    <button
      type={type}
      aria-pressed={selected}
      className={cn(
        "inline-flex h-[26px] items-center gap-1 rounded-full border px-3 text-[11px] font-medium whitespace-nowrap transition-all duration-[140ms] ease-out disabled:opacity-50 disabled:pointer-events-none",
        selected
          ? "border-accent/30 bg-accent-muted text-accent"
          : "border-glass-border bg-field text-fg-muted hover:text-fg hover:border-glass-border-hover",
        className,
      )}
      {...rest}
    >
      {children}
    </button>
  );
}

export interface ProfileChipOption {
  id: string;
  label: string;
  /** Perfil que recodifica o áudio (mostra aviso). */
  reencodes?: boolean;
}

export interface ProfileChipsProps {
  profiles: ProfileChipOption[];
  value: string;
  onChange: (id: string) => void;
  reencodeLabel: string;
  groupLabel: string;
}

export function ProfileChips({
  profiles,
  value,
  onChange,
  reencodeLabel,
  groupLabel,
}: ProfileChipsProps) {
  return (
    <div role="group" aria-label={groupLabel} className="flex flex-wrap gap-2">
      {profiles.map((p) => (
        <Chip
          key={p.id}
          selected={p.id === value}
          onClick={() => onChange(p.id)}
          title={p.reencodes ? reencodeLabel : undefined}
        >
          {p.label}
        </Chip>
      ))}
    </div>
  );
}
