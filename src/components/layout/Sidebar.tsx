import { NavLink } from "react-router";
import { useTranslation } from "react-i18next";
import { Tooltip } from "@/components/ui/Tooltip";
import { cn } from "@/components/ui/cn";
import { NAV_ITEMS, type NavId } from "@/lib/nav";
import { useUpdaterStore } from "@/stores/updater";
import { activeCount, useJobsStore } from "@/stores/jobs";
import { toolsUpdateCount, useToolsStore } from "@/stores/tools";

/** Contadores e pontos de aviso de cada item, vindos das stores. */
export function useNavIndicators(): { counts: Partial<Record<NavId, number>>; dots: NavId[] } {
  const jobs = useJobsStore((s) => s.jobs);
  const statuses = useToolsStore((s) => s.statuses);
  const appUpdate = useUpdaterStore((s) => s.available);
  const counts: Partial<Record<NavId, number>> = {};
  const active = activeCount(jobs);
  if (active > 0) counts.activity = active;
  const dots: NavId[] = toolsUpdateCount(statuses) > 0 || appUpdate ? ["settings"] : [];
  return { counts, dots };
}

/** Trilho de ícones de 58 px (≥ 640 px de largura). */
export function Sidebar() {
  const { t } = useTranslation();
  const { counts, dots } = useNavIndicators();
  const main = NAV_ITEMS.filter((i) => i.id !== "settings");
  const settings = NAV_ITEMS.find((i) => i.id === "settings")!;

  const renderItem = (item: (typeof NAV_ITEMS)[number]) => {
    const Icon = item.icon;
    const count = counts[item.id];
    const dot = dots.includes(item.id);
    const label = t(item.labelKey);
    return (
      <Tooltip key={item.id} content={label}>
        <NavLink
          to={item.to}
          end={item.to === "/"}
          aria-label={label}
          data-testid={`nav-${item.id}`}
          className={({ isActive }) =>
            cn(
              "relative flex aspect-square w-full items-center justify-center rounded-md transition-all duration-[140ms] ease-out",
              isActive
                ? "bg-accent-muted text-accent shadow-[inset_0_0_0_1px_var(--accent-muted)] before:absolute before:-left-2 before:top-1/2 before:h-[22px] before:w-[3px] before:-translate-y-1/2 before:rounded-r-[3px] before:bg-accent before:shadow-[0_0_8px_var(--accent-glow)] before:content-['']"
                : "text-fg-muted hover:bg-hover hover:text-fg",
            )
          }
        >
          <Icon className="size-5" aria-hidden="true" />
          {count !== undefined && (
            <span
              data-testid={`badge-${item.id}`}
              aria-hidden="true"
              className="absolute right-1 top-1 min-w-4 rounded-full bg-accent px-1 text-center text-[9px] font-bold leading-4 text-accent-on"
            >
              {count}
            </span>
          )}
          {dot && (
            <span
              data-testid={`dot-${item.id}`}
              aria-hidden="true"
              className="absolute right-[7px] top-[7px] size-[7px] rounded-full bg-accent shadow-[0_0_6px_var(--accent-glow)]"
            />
          )}
        </NavLink>
      </Tooltip>
    );
  };

  return (
    <nav
      aria-label={t("nav.main")}
      data-testid="sidebar"
      className="acrylic relative z-[5] flex w-[var(--sidebar-w)] shrink-0 flex-col items-center gap-1 border-r border-glass-border px-2 py-3 max-sm:hidden"
    >
      <div className="flex w-full flex-col gap-[3px]">{main.map(renderItem)}</div>
      <div className="mt-auto flex w-full flex-col gap-[3px]">
        <div className="mx-auto mb-2 h-px w-[30px] bg-glass-border" />
        {renderItem(settings)}
      </div>
    </nav>
  );
}
