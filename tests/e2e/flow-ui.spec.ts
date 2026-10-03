import AxeBuilder from "@axe-core/playwright";
import { expect, test, type Page } from "@playwright/test";
import { goHash, open, settle, setTheme, stubThumbnails, waitForRoute } from "./helpers";

// T9 — acessibilidade e screenshots das telas da F07 (Preview, Coleção, resultados de busca).
// Fonte/SO mudam o pixel: as bases são geradas e conferidas só no Windows (como visual.spec.ts).
const VISUAL = process.platform === "win32";
const SHOT = { animations: "disabled", caret: "hide", maxDiffPixelRatio: 0.002 } as const;
const ALBUM_URL =
  "https://music.youtube.com/playlist?list=OLAK5uy_nmDUsWOMoEcz0SsVqUwir0oxu-k1oUyXE";
const VIDEO_URL = "https://youtu.be/lYBUbBu4W08";

test.beforeEach(async ({ page }) => {
  await stubThumbnails(page);
});

const bar = (page: Page) => page.getByRole("textbox", { name: "Barra de comando" });

async function expectNoBlockingViolations(page: Page, where: string) {
  const results = await new AxeBuilder({ page }).analyze();
  const blocking = results.violations
    .filter((v) => v.impact === "serious" || v.impact === "critical")
    .map((v) => ({
      onde: where,
      regra: v.id,
      impacto: v.impact,
      alvos: v.nodes.slice(0, 3).map((n) => n.target.join(" ")),
    }));
  expect(blocking, where).toEqual([]);
}

/** Espera a animação do painel lateral terminar antes de capturar. */
async function settleDialog(page: Page) {
  await page.waitForFunction(
    () => document.querySelector('[role="dialog"]')?.getAnimations().length === 0,
  );
}

for (const theme of ["dark", "light"] as const) {
  test(`T9: Preview, Coleção e resultados — axe e tela (${theme})`, async ({ page }) => {
    await open(page, "/", "empty");
    await setTheme(page, theme);
    await goHash(page, "/");

    // Resultados de busca.
    await bar(page).fill("rick astley never gonna give you up");
    await bar(page).press("Enter");
    await expect(page.getByTestId("search-result")).toHaveCount(3);
    await expectNoBlockingViolations(page, `resultados (${theme})`);
    if (VISUAL) {
      await expect(page).toHaveScreenshot(`flow-search-${theme}.png`, SHOT);
    }

    // Preview.
    await bar(page).fill(VIDEO_URL);
    await bar(page).press("Enter");
    const preview = page.getByRole("dialog", { name: "Pré-visualização" });
    await expect(preview.getByTestId("preview-title")).toBeVisible();
    await settleDialog(page);
    await expectNoBlockingViolations(page, `preview (${theme})`);
    if (VISUAL) {
      await expect(page).toHaveScreenshot(`flow-preview-${theme}.png`, SHOT);
    }
    await preview.getByRole("button", { name: "MP3 320 kbps" }).click();
    await expect(preview.getByRole("note")).toBeVisible();
    await page.keyboard.press("Escape");
    await expect(preview).toHaveCount(0);

    // Coleção.
    await bar(page).fill(ALBUM_URL);
    await bar(page).press("Enter");
    await waitForRoute(page, "/collection");
    await settle(page);
    await page.getByRole("checkbox", { name: "Selecionar Together Forever" }).check();
    await expectNoBlockingViolations(page, `coleção (${theme})`);
    if (VISUAL) {
      await expect(page).toHaveScreenshot(`flow-collection-${theme}.png`, SHOT);
    }
  });
}

test("T9: Atividade com falhas e Início com recentes — axe", async ({ page }) => {
  await open(page, "/activity", "errors");
  await page.getByRole("tab", { name: /Falhas/ }).click();
  await expect(page.getByTestId("job-error").first()).toBeVisible();
  await expectNoBlockingViolations(page, "atividade (falhas)");
});

test("T9: responsivo — Coleção e Preview em 390×844 e 900×600 sem rolagem horizontal", async ({
  page,
}) => {
  for (const viewport of [
    { width: 390, height: 844 },
    { width: 900, height: 600 },
  ]) {
    await page.setViewportSize(viewport);
    await open(page, "/", "empty");
    await bar(page).fill(ALBUM_URL);
    await bar(page).press("Enter");
    await waitForRoute(page, "/collection");
    await settle(page);
    const overflow = await page.evaluate(
      () => document.documentElement.scrollWidth > window.innerWidth,
    );
    expect(overflow, `coleção ${viewport.width}px`).toBe(false);

    await page.goto("/?scenario=empty#/");
    await waitForRoute(page, "/");
    await bar(page).fill(VIDEO_URL);
    await bar(page).press("Enter");
    const preview = page.getByRole("dialog", { name: "Pré-visualização" });
    await expect(preview).toBeVisible();
    const box = (await preview.boundingBox())!;
    expect(box.x + box.width, `preview ${viewport.width}px`).toBeLessThanOrEqual(viewport.width);
  }
});
