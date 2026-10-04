import type { ReactNode } from "react";

export interface ScreenHeaderProps {
  title: string;
  subtitle?: string;
  actions?: ReactNode;
}

/** Cabeçalho padrão das telas: título (h1), subtítulo e ações opcionais. */
export function ScreenHeader({ title, subtitle, actions }: ScreenHeaderProps) {
  return (
    <div className="mb-6 flex flex-wrap items-start justify-between gap-4 max-sm:mb-5">
      <div className="min-w-0">
        <h1
          id="screen-title"
          className="font-display text-[28px] font-semibold tracking-[-0.025em] text-fg max-sm:text-2xl"
        >
          {title}
        </h1>
        {subtitle && (
          <p className="mt-2 max-w-2xl text-sm leading-relaxed text-fg-muted">{subtitle}</p>
        )}
      </div>
      {actions && <div className="flex flex-wrap items-center gap-2">{actions}</div>}
    </div>
  );
}
