import { NavLink } from "react-router";
import { useState } from "react";
import { PanelLeftClose, PanelLeftOpen } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Tooltip } from "@/components/ui/Tooltip";
import { cn } from "@/components/ui/cn";
import { NAV_ITEMS, type NavId } from "@/lib/nav";
import { useUpdaterStore } from "@/stores/updater";
import { activeCount, useJobsStore } from "@/stores/jobs";
import { useLibraryStore } from "@/stores/library";
import { toolsUpdateCount, useToolsStore } from "@/stores/tools";

/** Contadores e pontos de aviso de cada item, vindos das stores. */
export function useNavIndicators(): { counts: Partial<Record<NavId, number>>; dots: NavId[] } {
  const jobs = useJobsStore((s) => s.jobs);
  const reviewCount = useLibraryStore((s) => s.reviewCount);
  const statuses = useToolsStore((s) => s.statuses);
  const appUpdate = useUpdaterStore((s) => s.available);
  const counts: Partial<Record<NavId, number>> = {};
  const active = activeCount(jobs);
  if (active > 0) counts.activity = active;
  if (reviewCount > 0) counts.review = reviewCount;
  const dots: NavId[] = toolsUpdateCount(statuses) > 0 || appUpdate ? ["settings"] : [];
  return { counts, dots };
}

/** Rótulos no desktop; trilho compacto em janelas menores. */
export function Sidebar() {
  const { t } = useTranslation();
  const { counts, dots } = useNavIndicators();
  const [collapsed, setCollapsed] = useState(() => {
    try {
      return localStorage.getItem("reverb.sidebar.collapsed") === "true";
    } catch {
      return false;
    }
  });
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
              "relative flex h-11 w-full items-center justify-center gap-3 rounded-md transition-all duration-[140ms] ease-out",
              !collapsed && "lg:justify-start lg:px-3",
              isActive
                ? "bg-accent-muted text-accent shadow-[inset_0_0_0_1px_var(--accent-muted)] before:absolute before:-left-2 before:top-1/2 before:h-[22px] before:w-[3px] before:-translate-y-1/2 before:rounded-r-[3px] before:bg-accent before:shadow-[0_0_8px_var(--accent-glow)] before:content-['']"
                : "text-fg-muted hover:bg-hover hover:text-fg",
            )
          }
        >
          <Icon className="size-5 shrink-0" aria-hidden="true" />
          <span
            className={cn(
              "hidden min-w-0 flex-1 truncate text-sm font-medium",
              !collapsed && "lg:block",
            )}
          >
            {label}
          </span>
          {count !== undefined && (
            <span
              data-testid={`badge-${item.id}`}
              aria-hidden="true"
              className={cn(
                "absolute right-1 top-1 min-w-4 rounded-full bg-accent px-1 text-center text-[10px] font-bold leading-4 text-accent-on",
                !collapsed && "lg:static lg:ml-auto",
              )}
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
      data-collapsed={collapsed}
      className={cn(
        "acrylic relative z-[5] flex w-[var(--sidebar-w)] shrink-0 flex-col items-center gap-1 border-r border-glass-border px-2 py-4 max-sm:hidden",
        !collapsed && "lg:w-[196px] lg:px-3",
      )}
    >
      <div className="flex w-full flex-col gap-2">{main.map(renderItem)}</div>
      <div className="mt-auto flex w-full flex-col gap-[3px]">
        <Tooltip content={t(collapsed ? "nav.expand" : "nav.collapse")}>
          <button
            type="button"
            data-testid="sidebar-toggle"
            aria-label={t(collapsed ? "nav.expand" : "nav.collapse")}
            aria-expanded={!collapsed}
            className="hidden h-11 w-full items-center justify-center gap-3 rounded-md text-fg-muted hover:bg-hover hover:text-fg lg:flex"
            onClick={() => {
              const next = !collapsed;
              setCollapsed(next);
              try {
                localStorage.setItem("reverb.sidebar.collapsed", String(next));
              } catch {
                /* The menu remains usable when storage is unavailable. */
              }
            }}
          >
            {collapsed ? (
              <PanelLeftOpen className="size-5" />
            ) : (
              <>
                <PanelLeftClose className="size-5" />
                <span className="text-sm">{t("nav.collapse")}</span>
              </>
            )}
          </button>
        </Tooltip>
        <div className="mx-auto mb-2 h-px w-[30px] bg-glass-border" />
        {renderItem(settings)}
      </div>
    </nav>
  );
}
