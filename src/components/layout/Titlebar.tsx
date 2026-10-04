import { Minus, Square, X } from "lucide-react";
import { useTranslation } from "react-i18next";
import { windowControls } from "@/lib/window";
import { cn } from "@/components/ui/cn";
import reverbIcon from "@/assets/reverb.svg";

function ControlButton({
  label,
  onClick,
  danger,
  children,
}: {
  label: string;
  onClick: () => void;
  danger?: boolean;
  children: React.ReactNode;
}) {
  return (
    <button
      type="button"
      aria-label={label}
      onClick={onClick}
      className={cn(
        "flex h-[26px] w-9 items-center justify-center rounded-sm text-fg-muted transition-all duration-[140ms] ease-out [&_svg]:size-3",
        danger ? "hover:bg-[#e8503a] hover:text-white" : "hover:bg-hover hover:text-fg",
      )}
    >
      {children}
    </button>
  );
}

/** Barra de título própria (a janela não tem decoração nativa). Some abaixo de 640 px. */
export function Titlebar() {
  const { t } = useTranslation();
  return (
    <header
      data-testid="titlebar"
      className="acrylic relative z-10 flex h-[var(--titlebar-h)] shrink-0 items-center gap-3 border-b border-glass-border pl-3 pr-1 max-sm:hidden"
    >
      <div data-tauri-drag-region className="flex min-w-0 flex-1 items-center gap-3 self-stretch">
        <img
          src={reverbIcon}
          alt=""
          draggable={false}
          aria-hidden="true"
          data-tauri-drag-region
          className="size-6 shrink-0"
        />
        <span
          data-tauri-drag-region
          className="truncate font-display text-xs font-medium tracking-[0.02em] text-fg-secondary"
        >
          {t("app.name")}
        </span>
      </div>
      <div className="flex gap-0.5">
        <ControlButton
          label={t("titlebar.minimize")}
          onClick={() => void windowControls.minimize()}
        >
          <Minus />
        </ControlButton>
        <ControlButton
          label={t("titlebar.maximize")}
          onClick={() => void windowControls.toggleMaximize()}
        >
          <Square />
        </ControlButton>
        <ControlButton
          label={t("titlebar.close")}
          danger
          onClick={() => void windowControls.close()}
        >
          <X />
        </ControlButton>
      </div>
    </header>
  );
}
