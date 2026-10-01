import { expect, test } from "@playwright/test";
import { ROUTES, goHash, open } from "./helpers";

// T8 — sem rolagem horizontal em nenhuma largura; BottomNav só em telas estreitas.
const VIEWPORTS = [
  { width: 900, height: 600 },
  { width: 1366, height: 768 },
  { width: 1920, height: 1080 },
  { width: 390, height: 844 },
];

for (const viewport of VIEWPORTS) {
  test.describe(`T8: ${viewport.width}×${viewport.height}`, () => {
    test.use({ viewport });

    test("nenhuma rota tem rolagem horizontal (cenário busy)", async ({ page }) => {
      await open(page, "/", "busy");
      for (const route of ROUTES) {
        await goHash(page, route.path);
        const overflow = await page.evaluate(() => {
          const content = document.querySelector<HTMLElement>("[data-testid=content]");
          return {
            html: document.documentElement.scrollWidth - document.documentElement.clientWidth,
            body: document.body.scrollWidth - document.body.clientWidth,
            content: content ? content.scrollWidth - content.clientWidth : 0,
          };
        });
        expect(overflow, `rota ${route.path}`).toEqual({ html: 0, body: 0, content: 0 });
      }
    });

    test("navegação adequada à largura", async ({ page }) => {
      await open(page, "/");
      if (viewport.width < 640) {
        await expect(page.getByTestId("bottom-nav")).toBeVisible();
        await expect(page.getByTestId("sidebar")).toBeHidden();
        await expect(page.getByTestId("titlebar")).toBeHidden();
      } else {
        await expect(page.getByTestId("sidebar")).toBeVisible();
        await expect(page.getByTestId("titlebar")).toBeVisible();
        await expect(page.getByTestId("bottom-nav")).toBeHidden();
      }
    });
  });
}

test.describe("T8: BottomNav em 390×844", () => {
  test.use({ viewport: { width: 390, height: 844 } });

  test("navega pelos itens e pelo menu Mais", async ({ page }) => {
    await open(page, "/");
    await page.getByTestId("bottom-nav-library").click();
    await expect(page.getByRole("heading", { level: 1, name: "Biblioteca" })).toBeVisible();
    await page.getByTestId("bottom-nav-more").click();
    await page.getByRole("dialog", { name: "Mais" }).getByRole("link", { name: "Revisar" }).click();
    await expect(page.getByRole("heading", { level: 1, name: "Revisar" })).toBeVisible();
  });
});
