import type { ReactNode } from "react";

export interface ScreenHeaderProps {
  title: string;
  subtitle?: string;
  actions?: ReactNode;
}

/** Cabeçalho padrão das telas: título (h1), subtítulo e ações opcionais. */
export function ScreenHeader({ title, subtitle, actions }: ScreenHeaderProps) {
  return (
    <div className="mb-8 flex flex-wrap items-start justify-between gap-4 max-sm:mb-5">
      <div className="min-w-0">
        <h1
          id="screen-title"
          className="font-display text-2xl font-semibold tracking-[-0.015em] text-fg"
        >
          {title}
        </h1>
        {subtitle && <p className="mt-1 text-[13px] text-fg-muted">{subtitle}</p>}
      </div>
      {actions}
    </div>
  );
}
