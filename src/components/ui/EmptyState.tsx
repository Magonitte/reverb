import type { ReactNode } from "react";
import { cn } from "./cn";

export interface EmptyStateProps {
  icon: ReactNode;
  title: string;
  description?: string;
  action?: ReactNode;
  className?: string;
}

export function EmptyState({ icon, title, description, action, className }: EmptyStateProps) {
  return (
    <div
      data-testid="empty-state"
      className={cn(
        "glass flex flex-col items-center gap-3 rounded-xl px-6 py-12 text-center",
        className,
      )}
    >
      <div className="flex size-14 items-center justify-center rounded-full bg-accent-muted text-accent [&_svg]:size-6">
        {icon}
      </div>
      <h2 className="text-base font-semibold text-fg">{title}</h2>
      {description && <p className="max-w-md text-[13px] text-fg-muted">{description}</p>}
      {action}
    </div>
  );
}
