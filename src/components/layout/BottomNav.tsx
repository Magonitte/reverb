import { useState } from "react";
import { NavLink } from "react-router";
import { Ellipsis } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Dialog } from "@/components/ui/Dialog";
import { cn } from "@/components/ui/cn";
import { BOTTOM_NAV_IDS, MORE_NAV_IDS, NAV_BY_ID } from "@/lib/nav";
import { useNavIndicators } from "./Sidebar";

/** Navegação inferior (< 640 px): 4 itens + "Mais" (Revisar e Configurações). */
export function BottomNav() {
  const { t } = useTranslation();
  const { counts, dots } = useNavIndicators();
  const [moreOpen, setMoreOpen] = useState(false);

  const itemClass = (active: boolean) =>
    cn(
      "relative flex flex-1 flex-col items-center justify-center gap-0.5 text-[10px] font-medium transition-colors duration-[140ms]",
      active ? "text-accent" : "text-fg-muted hover:text-fg",
    );

  return (
    <>
      <nav
        aria-label={t("nav.main")}
        data-testid="bottom-nav"
        className="acrylic relative z-[5] flex h-[var(--bottomnav-h)] shrink-0 border-t border-glass-border sm:hidden"
      >
        {BOTTOM_NAV_IDS.map((id) => {
          const item = NAV_BY_ID[id];
          const Icon = item.icon;
          const count = counts[id];
          return (
            <NavLink
              key={id}
              to={item.to}
              end={item.to === "/"}
              data-testid={`bottom-nav-${id}`}
              className={({ isActive }) => itemClass(isActive)}
            >
              <Icon className="size-5" aria-hidden="true" />
              <span>{t(item.labelKey)}</span>
              {count !== undefined && (
                <span
                  aria-hidden="true"
                  className="absolute right-[calc(50%-20px)] top-1.5 min-w-4 rounded-full bg-accent px-1 text-center text-[9px] font-bold leading-4 text-accent-on"
                >
                  {count}
                </span>
              )}
            </NavLink>
          );
        })}
        <button
          type="button"
          data-testid="bottom-nav-more"
          onClick={() => setMoreOpen(true)}
          className={itemClass(false)}
        >
          <Ellipsis className="size-5" aria-hidden="true" />
          <span>{t("nav.more")}</span>
          {MORE_NAV_IDS.some((id) => dots.includes(id)) && (
            <span
              aria-hidden="true"
              className="absolute right-[calc(50%-14px)] top-2 size-[7px] rounded-full bg-accent"
            />
          )}
        </button>
      </nav>
      <Dialog
        open={moreOpen}
        onClose={() => setMoreOpen(false)}
        title={t("nav.more")}
        closeLabel={t("common.close")}
      >
        <ul className="flex flex-col gap-1">
          {MORE_NAV_IDS.map((id) => {
            const item = NAV_BY_ID[id];
            const Icon = item.icon;
            return (
              <li key={id}>
                <NavLink
                  to={item.to}
                  onClick={() => setMoreOpen(false)}
                  className="flex items-center gap-3 rounded-md px-3 py-3 text-[13px] text-fg-secondary hover:bg-hover hover:text-fg"
                >
                  <Icon className="size-5 text-accent" aria-hidden="true" />
                  {t(item.labelKey)}
                </NavLink>
              </li>
            );
          })}
        </ul>
      </Dialog>
    </>
  );
}
