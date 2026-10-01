import { describe, expect, it } from "vitest";
import { resolveTheme, resolveTransparency } from "./appearance";

describe("resolveTheme (T3)", () => {
  it("dark e light valem como estão, independente do sistema", () => {
    expect(resolveTheme("dark", false)).toBe("dark");
    expect(resolveTheme("dark", true)).toBe("dark");
    expect(resolveTheme("light", true)).toBe("light");
    expect(resolveTheme("light", false)).toBe("light");
  });

  it("system segue o sistema", () => {
    expect(resolveTheme("system", true)).toBe("dark");
    expect(resolveTheme("system", false)).toBe("light");
  });
});

describe("resolveTransparency (T4)", () => {
  it("auto: Linux ⇒ reduced; Windows ⇒ full", () => {
    expect(resolveTransparency("auto", "linux")).toBe("reduced");
    expect(resolveTransparency("auto", "windows")).toBe("full");
    expect(resolveTransparency("auto", "android")).toBe("full");
  });

  it("full e reduced valem como estão em qualquer plataforma", () => {
    expect(resolveTransparency("full", "linux")).toBe("full");
    expect(resolveTransparency("reduced", "windows")).toBe("reduced");
  });
});
