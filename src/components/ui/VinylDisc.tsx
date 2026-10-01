import { cn } from "./cn";

export interface VinylDiscProps {
  /** Gira (download em andamento). Parado com prefers-reduced-motion (base.css). */
  spinning?: boolean;
  size?: number;
  className?: string;
}

const DISC =
  "radial-gradient(circle at 50% 50%, #2a2520 0%, #221e18 18%, #1a1710 18.5%, #2a2520 20%, transparent 20.5%, transparent 21%, rgba(245,158,75,.12) 21%, rgba(245,158,75,.06) 28%, transparent 28.5%, transparent 30%, rgba(245,158,75,.10) 30%, rgba(245,158,75,.04) 40%, transparent 40.5%, transparent 43%, rgba(245,158,75,.08) 43%, rgba(245,158,75,.03) 55%, transparent 55.5%, transparent 58%, rgba(245,158,75,.06) 58%, rgba(245,158,75,.02) 70%, #141210 100%)";

export function VinylDisc({ spinning = false, size = 48, className }: VinylDiscProps) {
  return (
    <div
      aria-hidden="true"
      data-spinning={spinning}
      style={{
        width: size,
        height: size,
        animation: spinning ? "reverb-spin 2.4s linear infinite" : undefined,
        background: DISC,
      }}
      className={cn(
        "relative flex shrink-0 items-center justify-center rounded-full shadow-[0_0_0_1px_var(--glass-border),var(--shadow-md)]",
        className,
      )}
    >
      <span
        className="rounded-full"
        style={{
          width: size * 0.13,
          height: size * 0.13,
          background: "radial-gradient(circle at 40% 40%, #e8943a, #8a4a10)",
          boxShadow: "0 0 12px rgba(245,158,75,.5)",
        }}
      />
    </div>
  );
}
