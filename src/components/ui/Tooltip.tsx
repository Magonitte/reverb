import { cloneElement, useId, useState, type ReactElement } from "react";
import { cn } from "./cn";

export interface TooltipProps {
  content: string;
  /** Lado em que o balão aparece. */
  side?: "right" | "top" | "bottom";
  children: ReactElement<{ "aria-describedby"?: string }>;
}

const SIDES = {
  right: "left-full top-1/2 ml-3 -translate-y-1/2",
  top: "bottom-full left-1/2 mb-2 -translate-x-1/2",
  bottom: "top-full left-1/2 mt-2 -translate-x-1/2",
};

/** Balão que aparece no hover e no foco do teclado. */
export function Tooltip({ content, side = "right", children }: TooltipProps) {
  const [open, setOpen] = useState(false);
  const id = useId();
  return (
    <span
      className="relative flex"
      onMouseEnter={() => setOpen(true)}
      onMouseLeave={() => setOpen(false)}
      onFocus={() => setOpen(true)}
      onBlur={() => setOpen(false)}
    >
      {cloneElement(children, { "aria-describedby": open ? id : undefined })}
      {open && (
        <span
          id={id}
          role="tooltip"
          className={cn(
            "glass-elevated pointer-events-none absolute z-50 whitespace-nowrap rounded-md px-2.5 py-1 text-xs text-fg shadow-md",
            SIDES[side],
          )}
        >
          {content}
        </span>
      )}
    </span>
  );
}
