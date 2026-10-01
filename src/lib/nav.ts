import {
  ArrowDownUp,
  House,
  Library,
  ListChecks,
  ListMusic,
  Settings,
  type LucideIcon,
} from "lucide-react";

export type NavId = "home" | "library" | "playlists" | "activity" | "review" | "settings";

export interface NavItem {
  id: NavId;
  to: string;
  icon: LucideIcon;
  /** Chave i18n do rótulo. */
  labelKey: string;
}

/** Itens do trilho (Sidebar). `settings` fica sozinho no fim. */
export const NAV_ITEMS: NavItem[] = [
  { id: "home", to: "/", icon: House, labelKey: "nav.home" },
  { id: "library", to: "/library", icon: Library, labelKey: "nav.library" },
  { id: "playlists", to: "/playlists", icon: ListMusic, labelKey: "nav.playlists" },
  { id: "activity", to: "/activity", icon: ArrowDownUp, labelKey: "nav.activity" },
  { id: "review", to: "/review", icon: ListChecks, labelKey: "nav.review" },
  { id: "settings", to: "/settings", icon: Settings, labelKey: "nav.settings" },
];

export const NAV_BY_ID = Object.fromEntries(NAV_ITEMS.map((i) => [i.id, i])) as Record<
  NavId,
  NavItem
>;

/** Itens do BottomNav; "Mais" abre Revisar e Configurações. */
export const BOTTOM_NAV_IDS: NavId[] = ["home", "library", "playlists", "activity"];
export const MORE_NAV_IDS: NavId[] = ["review", "settings"];
