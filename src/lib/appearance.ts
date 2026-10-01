import type { Theme } from "@/bindings/Theme";
import type { Transparency } from "@/bindings/Transparency";

export type ResolvedTheme = "dark" | "light";
export type ResolvedTransparency = "full" | "reduced";

/** `system` segue o sistema; os demais valem como estão. */
export function resolveTheme(theme: Theme, systemDark: boolean): ResolvedTheme {
  if (theme === "system") return systemDark ? "dark" : "light";
  return theme;
}

/** `auto` ⇒ `reduced` no Linux (WebKitGTK é lento com backdrop-filter), `full` nos demais (§1). */
export function resolveTransparency(
  transparency: Transparency,
  platform: string,
): ResolvedTransparency {
  if (transparency === "auto") return platform === "linux" ? "reduced" : "full";
  return transparency;
}
