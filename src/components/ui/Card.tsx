import type { HTMLAttributes } from "react";
import { cn } from "./cn";

export interface CardProps extends HTMLAttributes<HTMLDivElement> {
  interactive?: boolean;
  elevated?: boolean;
}

export function Card({ interactive, elevated, className, ...rest }: CardProps) {
  return (
    <div
      className={cn(
        elevated ? "glass-elevated" : "glass",
        "rounded-lg p-5 transition-all duration-[140ms] ease-out hover:bg-glass-hover hover:border-glass-border-hover hover:shadow-md",
        interactive && "cursor-pointer active:scale-[0.985]",
        className,
      )}
      {...rest}
    />
  );
}
