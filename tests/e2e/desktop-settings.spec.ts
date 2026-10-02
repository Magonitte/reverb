import { expect, test } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";
import { open, goHash, setTheme } from "./helpers";
test("F12 general desktop toggles persist across tabs", async ({ page }) => {
  await open(page, "/settings");
  const startup = page.getByRole("switch", { name: "Iniciar com o sistema" });
  await startup.click();
  await expect(startup).toHaveAttribute("aria-checked", "true");
  await goHash(page, "/settings/advanced");
  await page.getByLabel("Runtime JavaScript").selectOption("system-node");
  await page.getByRole("switch", { name: "Rodar diagnóstico semanal" }).click();
  await goHash(page, "/settings");
  await expect(startup).toHaveAttribute("aria-checked", "true");
  await goHash(page, "/settings/advanced");
  await expect(page.getByRole("switch", { name: "Rodar diagnóstico semanal" })).toHaveAttribute(
    "aria-checked",
    "false",
  );
  await page.getByRole("button", { name: "Rodar agora" }).click();
  await expect(page.getByRole("list", { name: "Resultados do diagnóstico" })).toBeVisible();
  await expect(page.getByText("Integridade do banco")).toBeVisible();
});
test("F12 restore confirmation cancels; reset opens onboarding", async ({ page }) => {
  await open(page, "/settings/advanced");
  await page.getByRole("button", { name: "Restaurar backup" }).click();
  await expect(page.getByRole("dialog")).toContainText("interromperá downloads e reiniciará");
  await page.getByRole("dialog").getByRole("button", { name: "Cancelar" }).click();
  await expect(page.getByRole("dialog")).toHaveCount(0);
  await page.getByRole("button", { name: "Restaurar padrões" }).click();
  await page.getByRole("dialog").getByRole("button", { name: "Confirmar" }).click();
  await expect(page.getByRole("heading", { level: 1 })).toHaveText("Bem-vindo ao Reverb");
});

for (const theme of ["dark", "light"] as const) {
  test(`F12 integration and advanced accessibility (${theme})`, async ({ page }) => {
    await open(page, "/settings");
    await setTheme(page, theme);
    for (const route of ["/settings/integration", "/settings/advanced"]) {
      await goHash(page, route);
      await page.screenshot({
        path: `test-results/f12-${route.split("/").at(-1)}-${theme}.png`,
        fullPage: true,
      });
      const results = await new AxeBuilder({ page }).analyze();
      expect(
        results.violations.filter((violation) =>
          ["serious", "critical"].includes(violation.impact ?? ""),
        ),
      ).toEqual([]);
    }
    await page.getByRole("button", { name: "Rodar agora" }).click();
    await expect(page.getByRole("list", { name: "Resultados do diagnóstico" })).toBeVisible();
    const results = await new AxeBuilder({ page }).analyze();
    expect(
      results.violations.filter((violation) =>
        ["serious", "critical"].includes(violation.impact ?? ""),
      ),
    ).toEqual([]);
  });
}
