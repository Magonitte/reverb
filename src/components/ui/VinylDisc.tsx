import { useId } from "react";
import { cn } from "./cn";

export interface VinylDiscProps {
  spinning?: boolean;
  size?: number;
  className?: string;
}

/** Vinyl grooves, reflected light, paper label and a real spindle hole. */
export function VinylDisc({ spinning = false, size = 48, className }: VinylDiscProps) {
  const id = useId();
  return (
    <div
      aria-hidden="true"
      data-spinning={spinning}
      style={{
        width: size,
        height: size,
        animation: spinning ? "reverb-spin 2.4s linear infinite" : undefined,
      }}
      className={cn(
        "relative shrink-0 rounded-full shadow-[0_0_0_1px_var(--glass-border),var(--shadow-md)]",
        className,
      )}
    >
      <svg viewBox="0 0 100 100" className="size-full">
        <defs>
          <linearGradient id={`${id}-vinyl`} x1="0" y1="0" x2="1" y2="1">
            <stop stopColor="#33343a" />
            <stop offset=".35" stopColor="#111216" />
            <stop offset=".55" stopColor="#292a30" />
            <stop offset=".75" stopColor="#101115" />
            <stop offset="1" stopColor="#38393f" />
          </linearGradient>
          <linearGradient id={`${id}-label`} x1="0" y1="0" x2="1" y2="1">
            <stop stopColor="#ffc477" />
            <stop offset="1" stopColor="#cc762d" />
          </linearGradient>
        </defs>
        <circle
          cx="50"
          cy="50"
          r="49"
          fill={`url(#${id}-vinyl)`}
          stroke="#55565b"
          strokeWidth="1"
        />
        {[22, 25, 28, 31, 34, 37, 40, 43, 46].map((r) => (
          <circle
            key={r}
            cx="50"
            cy="50"
            r={r}
            fill="none"
            stroke="#74757c"
            strokeOpacity=".22"
            strokeWidth=".65"
          />
        ))}
        <circle cx="50" cy="50" r="18" fill={`url(#${id}-label)`} />
        <circle
          cx="50"
          cy="50"
          r="13"
          fill="none"
          stroke="#79441d"
          strokeOpacity=".35"
          strokeWidth=".8"
        />
        <path d="M44 40h12M44 60h12" stroke="#79441d" strokeOpacity=".6" strokeWidth="1.8" />
        <circle cx="50" cy="50" r="3.5" fill="#090a0e" stroke="#e0a568" strokeWidth="1" />
      </svg>
    </div>
  );
}
