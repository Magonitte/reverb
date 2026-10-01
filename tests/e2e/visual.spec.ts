import { expect, test } from "@playwright/test";
import { ROUTES, goHash, open, setTheme, type Scenario } from "./helpers";

// T10 — screenshots de base (1366×768, dois temas). Fonte/SO mudam o pixel: só no Windows.
test.skip(process.platform !== "win32", "screenshots de base são geradas e conferidas no Windows");
test.use({ viewport: { width: 1366, height: 768 } });

const SHOT = { animations: "disabled", caret: "hide", maxDiffPixelRatio: 0.002 } as const;

const WITH_DATA: Record<string, Scenario> = { "/": "busy", "/activity": "busy" };

for (const theme of ["dark", "light"] as const) {
  test(`T10: rotas em ${theme}`, async ({ page }) => {
    await open(page, "/", "empty");
    await setTheme(page, theme);
    for (const route of ROUTES) {
      await goHash(page, route.path);
      const name = `${route.path === "/" ? "home" : route.path.slice(1).replace(/\W+/g, "-")}-${theme}`;
      await expect(page).toHaveScreenshot(`${name}.png`, SHOT);
    }
  });

  test(`T10: telas com dados (cenário busy) em ${theme}`, async ({ page }) => {
    for (const [path, scenario] of Object.entries(WITH_DATA)) {
      await open(page, path, scenario);
      await setTheme(page, theme);
      await goHash(page, path);
      const name = `${path === "/" ? "home" : path.slice(1)}-busy-${theme}`;
      await expect(page).toHaveScreenshot(`${name}.png`, SHOT);
    }
  });

  test(`T10: banner de autocura em ${theme}`, async ({ page }) => {
    await open(page, "/", "heal");
    await setTheme(page, theme);
    await goHash(page, "/");
    await expect(page.getByTestId("heal-banner")).toBeVisible();
    await expect(page).toHaveScreenshot(`home-heal-${theme}.png`, SHOT);
  });
}

test("T10: overlay Ctrl+K e Dialog", async ({ page }) => {
  await open(page, "/", "empty");
  await page.keyboard.press("Control+k");
  await expect(page.getByTestId("command-bar-overlay")).toBeVisible();
  await expect(page).toHaveScreenshot("command-bar-overlay-dark.png", SHOT);
});

test("T10: mobile 390×844 (BottomNav)", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await open(page, "/", "busy");
  await expect(page).toHaveScreenshot("home-mobile-dark.png", SHOT);
});
