import { useId, useRef, type KeyboardEvent, type ReactNode } from "react";
import { cn } from "./cn";

export interface TabItem {
  id: string;
  label: string;
  /** Contador opcional ao lado do rótulo. */
  count?: number;
}

export interface TabsProps {
  tabs: TabItem[];
  value: string;
  onChange: (id: string) => void;
  /** Texto acessível da lista de abas. */
  label: string;
  /** Conteúdo do painel ativo. */
  children?: ReactNode;
  className?: string;
}

/** Abas com teclado: ←/→ navegam (circular), Home/End vão às pontas. */
export function Tabs({ tabs, value, onChange, label, children, className }: TabsProps) {
  const base = useId();
  const refs = useRef<Record<string, HTMLButtonElement | null>>({});

  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    const index = tabs.findIndex((t) => t.id === value);
    const nextIndex: Record<string, number> = {
      ArrowRight: (index + 1) % tabs.length,
      ArrowLeft: (index - 1 + tabs.length) % tabs.length,
      Home: 0,
      End: tabs.length - 1,
    };
    const next = nextIndex[event.key];
    if (next === undefined) return;
    event.preventDefault();
    const target = tabs[next]!;
    onChange(target.id);
    refs.current[target.id]?.focus();
  };

  return (
    <div className={className}>
      <div
        role="tablist"
        aria-label={label}
        onKeyDown={onKeyDown}
        className="flex gap-1 overflow-x-auto border-b border-glass-border"
      >
        {tabs.map((tab) => {
          const selected = tab.id === value;
          return (
            <button
              key={tab.id}
              ref={(el) => {
                refs.current[tab.id] = el;
              }}
              type="button"
              role="tab"
              id={`${base}-tab-${tab.id}`}
              aria-selected={selected}
              aria-controls={`${base}-panel-${tab.id}`}
              tabIndex={selected ? 0 : -1}
              onClick={() => onChange(tab.id)}
              className={cn(
                "relative -mb-px whitespace-nowrap border-b-2 px-4 py-2 text-[13px] font-medium transition-colors duration-[140ms]",
                selected
                  ? "border-accent text-accent"
                  : "border-transparent text-fg-muted hover:text-fg",
              )}
            >
              {tab.label}
              {tab.count !== undefined && (
                <span className="ml-2 rounded-full bg-field px-1.5 text-[10px] text-fg-muted">
                  {tab.count}
                </span>
              )}
            </button>
          );
        })}
      </div>
      {children !== undefined && (
        <div
          role="tabpanel"
          id={`${base}-panel-${value}`}
          aria-labelledby={`${base}-tab-${value}`}
          className="pt-5"
        >
          {children}
        </div>
      )}
    </div>
  );
}
