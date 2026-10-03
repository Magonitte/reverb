import { test, expect } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";
import { setTheme, goHash } from "./helpers";
test("F15 import shows match states, filters, selection and download", async ({ page }) => {
  await page.goto("/#/");
  const input = page.locator("[data-command-bar] input");
  await input.fill("https://deezer.com/br/playlist/5207214368");
  await input.press("Enter");
  await expect(page.getByRole("heading", { name: "Imported collection" })).toBeVisible();
  await expect(page.getByText("Encontrada 100% · ISRC")).toBeVisible();
  await expect(page.getByText("Revisar 82% · TEXT")).toBeVisible();
  const none = page.getByRole("checkbox", { name: "Selecionar Track 3" });
  await expect(none).toBeDisabled();
  await page.getByLabel("Filtrar casamento").selectOption("review");
  await expect(page.getByRole("checkbox", { name: "Selecionar Track 2" })).toBeVisible();
  await page.getByLabel("Filtrar casamento").selectOption("all");
  expect((await new AxeBuilder({ page }).analyze()).violations).toEqual([]);
  await page.getByRole("button", { name: /Baixar selecionadas/ }).click();
  await expect(page.getByRole("heading", { name: "Atividade" })).toBeVisible();
});
test("F15 artist dialog, followed releases and missing downloads", async ({ page }) => {
  await page.goto("/#/library");
  await page.getByRole("button", { name: "Artistas seguidos", exact: true }).click();
  await page.getByRole("button", { name: "Seguir artista", exact: true }).click();
  const dialog = page.getByRole("dialog", { name: "Seguir artista" });
  await dialog.getByLabel("Buscar artista").fill("Rick Astley");
  await dialog.getByRole("button", { name: "Buscar artista" }).click();
  await dialog.getByRole("button", { name: /Rick Astley/ }).click();
  expect((await new AxeBuilder({ page }).analyze()).violations).toEqual([]);
  await dialog.getByRole("button", { name: "Salvar", exact: true }).click();
  await expect(dialog).not.toBeVisible();
  await expect(page.getByText("10 lançamentos · 3 completos")).toBeVisible();
  await page.getByRole("button", { name: "Faltando", exact: true }).click();
  await expect(page.getByText("Release 4", { exact: true })).toBeVisible();
  await expect(page.getByText("Release 1", { exact: true })).not.toBeVisible();
  await page.getByRole("button", { name: "Baixar faltantes" }).first().click();
});
test("F15 Spotify guide and quality target settings are accessible", async ({ page }) => {
  await page.goto("/#/settings");
  await page.getByRole("tab", { name: "Metadados", exact: true }).click();
  await page.getByText("Como configurar o Spotify", { exact: true }).click();
  await expect(page.getByText(/Entre no painel de desenvolvedores/)).toBeVisible();
  await page.getByRole("tab", { name: "Downloads", exact: true }).click();
  await page.getByLabel("Qualidade-alvo da fonte").selectOption("256");
  await expect(page.getByLabel("Qualidade-alvo da fonte")).toHaveValue("256");
  expect((await new AxeBuilder({ page }).analyze()).violations).toEqual([]);
});
test("F15 collection and followed artist screenshots in both themes", async ({ page }, info) => {
  await page.goto("/#/");
  const input = page.locator("[data-command-bar] input");
  await input.fill("https://deezer.com/playlist/5207214368");
  await input.press("Enter");
  await expect(page.getByRole("heading", { name: "Imported collection" })).toBeVisible();
  for (const theme of ["dark", "light"] as const) {
    await setTheme(page, theme);
    await goHash(page, "/collection");
    await page.screenshot({
      path: info.outputPath(`f15-import-${theme}.png`),
      animations: "disabled",
    });
    expect((await new AxeBuilder({ page }).analyze()).violations).toEqual([]);
  }
});
