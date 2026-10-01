import { useRef, type ReactNode } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import { cn } from "./cn";

export interface VirtualColumn<T> {
  key: string;
  header: string;
  /** Largura CSS da coluna (ex.: "2fr", "80px"). */
  width?: string;
  render: (row: T) => ReactNode;
}

export interface VirtualTableProps<T> {
  rows: T[];
  columns: VirtualColumn<T>[];
  rowKey: (row: T) => string;
  rowHeight?: number;
  /** Texto acessível da tabela. */
  label: string;
  className?: string;
}

/** Tabela virtualizada: só as linhas visíveis entram no DOM. */
export function VirtualTable<T>({
  rows,
  columns,
  rowKey,
  rowHeight = 48,
  label,
  className,
}: VirtualTableProps<T>) {
  const scrollRef = useRef<HTMLDivElement>(null);
  // eslint-disable-next-line react-hooks/incompatible-library -- API do TanStack Virtual; não passamos o virtualizer adiante.
  const virtualizer = useVirtualizer({
    count: rows.length,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => rowHeight,
    overscan: 8,
  });
  const template = columns.map((c) => c.width ?? "1fr").join(" ");

  return (
    <div
      role="table"
      aria-label={label}
      aria-rowcount={rows.length}
      className={cn("glass flex min-h-0 flex-col rounded-lg", className)}
    >
      <div role="rowgroup">
        <div
          role="row"
          className="grid items-center gap-3 border-b border-glass-border px-4 py-2 text-left text-xs font-medium text-fg-muted"
          style={{ gridTemplateColumns: template }}
        >
          {columns.map((c) => (
            <div key={c.key} role="columnheader" className="text-left">
              {c.header}
            </div>
          ))}
        </div>
      </div>
      <div ref={scrollRef} role="rowgroup" className="min-h-0 flex-1 overflow-y-auto">
        <div style={{ height: virtualizer.getTotalSize(), position: "relative" }}>
          {virtualizer.getVirtualItems().map((item) => {
            const row = rows[item.index]!;
            return (
              <div
                key={rowKey(row)}
                role="row"
                aria-rowindex={item.index + 1}
                className="absolute left-0 grid w-full items-center gap-3 border-b border-glass-border px-4 text-[13px] hover:bg-hover"
                style={{
                  top: 0,
                  height: item.size,
                  transform: `translateY(${item.start}px)`,
                  gridTemplateColumns: template,
                }}
              >
                {columns.map((c) => (
                  <div key={c.key} role="cell" className="min-w-0 truncate">
                    {c.render(row)}
                  </div>
                ))}
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
}
