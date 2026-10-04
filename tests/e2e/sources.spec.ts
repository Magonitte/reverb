import { test, expect } from "@playwright/test";
test("F14 source tabs open the Archive collection", async ({ page }) => {
  await page.goto("/#/");
  const input = page.locator("[data-command-bar] input");
  await input.fill("goldberg");
  await input.press("Enter");
  await page.getByRole("tab", { name: "Internet Archive" }).click();
  await expect(page.getByText("The Open Goldberg Variations", { exact: true })).toBeVisible();
  await page
    .getByRole("button", { name: "Abrir a pré-visualização de The Open Goldberg Variations" })
    .click();
  await expect(page.getByText("Aria", { exact: true })).toBeVisible();
});
test("F14 library verifier displays the spectrum and probable verdict", async ({ page }) => {
  await page.goto("/?scenario=big#/library");
  await page
    .getByRole("button", { name: /^Detalhes e ações de/ })
    .first()
    .click();
  await page
    .getByRole("button", { name: /Verificar lossless/ })
    .first()
    .click();
  await expect(page.getByRole("dialog", { name: "Verificar lossless" })).toBeVisible();
  await expect(page.getByText("Provável lossless", { exact: true })).toBeVisible();
  await expect(page.getByRole("img", { name: "Espectrograma do áudio" })).toBeVisible();
});
