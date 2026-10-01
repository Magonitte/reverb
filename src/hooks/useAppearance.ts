import { useEffect, useSyncExternalStore } from "react";
import i18n from "@/lib/i18n";
import { resolveTheme, resolveTransparency } from "@/lib/appearance";
import { isTauri } from "@/lib/ipc/isTauri";
import { useAppInfoStore } from "@/stores/appInfo";
import { useSettingsStore } from "@/stores/settings";

const QUERY = "(prefers-color-scheme: dark)";

function systemPrefersDark(): boolean {
  return typeof window !== "undefined" && window.matchMedia?.(QUERY).matches === true;
}

function subscribeSystemTheme(onChange: () => void): () => void {
  if (typeof window === "undefined" || !window.matchMedia) return () => {};
  const media = window.matchMedia(QUERY);
  media.addEventListener("change", onChange);
  return () => media.removeEventListener("change", onChange);
}

/**
 * Aplica `data-theme`, `data-transparency` e o idioma conforme as configurações e a plataforma.
 * `system` acompanha o `prefers-color-scheme` e reage a mudanças.
 */
export function useAppearance(): void {
  const theme = useSettingsStore((s) => s.settings?.theme ?? "dark");
  const transparency = useSettingsStore((s) => s.settings?.transparency ?? "auto");
  const language = useSettingsStore((s) => s.settings?.language);
  const platform = useAppInfoStore((s) => s.info?.platform ?? "windows");
  const systemDark = useSyncExternalStore(subscribeSystemTheme, systemPrefersDark, () => true);

  useEffect(() => {
    const root = document.documentElement;
    const resolvedTransparency = resolveTransparency(transparency, platform);
    root.dataset.theme = resolveTheme(theme, systemDark);
    root.dataset.transparency = resolvedTransparency;
    root.dataset.platform = platform;
    // Mica: só na janela nativa do Windows com transparência; no navegador o fundo precisa existir.
    root.dataset.mica = String(
      isTauri() && platform === "windows" && resolvedTransparency === "full",
    );
  }, [theme, transparency, platform, systemDark]);

  useEffect(() => {
    if (!language) return;
    if (i18n.language !== language) void i18n.changeLanguage(language);
    document.documentElement.lang = language;
  }, [language]);
}
