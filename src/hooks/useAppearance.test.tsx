import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import i18n from "@/lib/i18n";
import { DEFAULT_SETTINGS } from "@/lib/ipc/mock/settings";
import { useAppInfoStore } from "@/stores/appInfo";
import { useSettingsStore } from "@/stores/settings";
import { useAppearance } from "./useAppearance";

/** matchMedia falso, com `change` disparável. */
function installMatchMedia(initialDark: boolean) {
  let dark = initialDark;
  const listeners = new Set<(e: MediaQueryListEvent) => void>();
  window.matchMedia = ((query: string) => ({
    get matches() {
      return dark;
    },
    media: query,
    addEventListener: (_: string, fn: (e: MediaQueryListEvent) => void) => listeners.add(fn),
    removeEventListener: (_: string, fn: (e: MediaQueryListEvent) => void) => listeners.delete(fn),
  })) as unknown as typeof window.matchMedia;
  return {
    setDark(value: boolean) {
      dark = value;
      listeners.forEach((fn) => fn({ matches: value } as MediaQueryListEvent));
    },
  };
}

function setSettings(patch: Partial<typeof DEFAULT_SETTINGS>) {
  useSettingsStore.setState({ settings: { ...DEFAULT_SETTINGS, ...patch } });
}

const root = document.documentElement;

describe("useAppearance", () => {
  beforeEach(() => {
    useAppInfoStore.setState({ info: { version: "0", platform: "windows", arch: "x86_64" } });
  });
  afterEach(() => {
    useSettingsStore.setState({ settings: null });
    useAppInfoStore.setState({ info: null });
    void i18n.changeLanguage("pt-BR");
  });

  it("T3: dark e light aplicam data-theme", () => {
    installMatchMedia(false);
    setSettings({ theme: "dark" });
    renderHook(() => useAppearance());
    expect(root.dataset.theme).toBe("dark");
    act(() => setSettings({ theme: "light" }));
    expect(root.dataset.theme).toBe("light");
  });

  it("T3: system segue o media query e reage à mudança", () => {
    const media = installMatchMedia(true);
    setSettings({ theme: "system" });
    renderHook(() => useAppearance());
    expect(root.dataset.theme).toBe("dark");
    act(() => media.setDark(false));
    expect(root.dataset.theme).toBe("light");
    act(() => media.setDark(true));
    expect(root.dataset.theme).toBe("dark");
  });

  it("T4: auto + linux ⇒ reduced; auto + windows ⇒ full", () => {
    installMatchMedia(true);
    setSettings({ transparency: "auto" });
    useAppInfoStore.setState({ info: { version: "0", platform: "linux", arch: "x86_64" } });
    renderHook(() => useAppearance());
    expect(root.dataset.transparency).toBe("reduced");
    act(() =>
      useAppInfoStore.setState({ info: { version: "0", platform: "windows", arch: "x86_64" } }),
    );
    expect(root.dataset.transparency).toBe("full");
  });

  it("T4: a escolha explícita vence a plataforma", () => {
    installMatchMedia(true);
    setSettings({ transparency: "reduced" });
    renderHook(() => useAppearance());
    expect(root.dataset.transparency).toBe("reduced");
  });

  it("não ativa o Mica fora da janela nativa (navegador/testes)", () => {
    installMatchMedia(true);
    setSettings({ transparency: "full" });
    renderHook(() => useAppearance());
    expect(root.dataset.mica).toBe("false");
  });

  it("aplica o idioma das configurações sem recarregar", () => {
    installMatchMedia(true);
    setSettings({ language: "en" });
    renderHook(() => useAppearance());
    expect(i18n.language).toBe("en");
    expect(root.lang).toBe("en");
    act(() => setSettings({ language: "pt-BR" }));
    expect(i18n.language).toBe("pt-BR");
  });
});
