import AxeBuilder from "@axe-core/playwright";
import { expect, test } from "@playwright/test";
import { ROUTES, goHash, open, setTheme } from "./helpers";

// T9 — axe em todas as rotas, nos dois temas: zero violações serious/critical.
for (const theme of ["dark", "light"] as const) {
  test(`T9: sem violações serious/critical (${theme})`, async ({ page }) => {
    await open(page, "/", "busy");
    await setTheme(page, theme);

    for (const route of ROUTES) {
      await goHash(page, route.path);
      const results = await new AxeBuilder({ page }).analyze();
      const blocking = results.violations
        .filter((v) => v.impact === "serious" || v.impact === "critical")
        .map((v) => ({
          rota: route.path,
          regra: v.id,
          impacto: v.impact,
          alvos: v.nodes.slice(0, 3).map((n) => n.target.join(" ")),
        }));
      expect(blocking, `rota ${route.path} (${theme})`).toEqual([]);
    }
  });
}

test("T9: o Dialog de Mais (BottomNav) é acessível", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await open(page, "/");
  await page.getByTestId("bottom-nav-more").click();
  await expect(page.getByRole("dialog", { name: "Mais" })).toBeVisible();
  const results = await new AxeBuilder({ page }).analyze();
  const blocking = results.violations.filter(
    (v) => v.impact === "serious" || v.impact === "critical",
  );
  expect(blocking.map((v) => v.id)).toEqual([]);
});
