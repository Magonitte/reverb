import { expect, test } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";
import { open } from "./helpers";

test("F10: library search, pagination and virtualization", async ({ page }) => {
  await open(page, "/library", "big");
  await expect(page.getByText("1–100 de 5000 faixas")).toBeVisible();
  expect(await page.getByRole("row").count()).toBeLessThan(200);
  await page.getByRole("button", { name: "Próxima" }).click();
  await expect(page.getByText("101–200 de 5000 faixas")).toBeVisible();
  await page.getByRole("textbox", { name: "Buscar na biblioteca" }).fill("musica nev");
  await expect(page.getByText("1–1 de 1 faixas")).toBeVisible();
  await expect(page.getByText("Música Never Gonna Give You Up", { exact: true })).toBeVisible();
});

test("F10: clear library requires confirmation", async ({ page }) => {
  await open(page, "/library", "big");
  await page.getByRole("button", { name: "Limpar biblioteca" }).click();
  await page.getByRole("dialog").getByRole("button", { name: "Cancelar" }).click();
  await expect(page.getByText("1–100 de 5000 faixas")).toBeVisible();
  await page.getByRole("button", { name: "Limpar biblioteca" }).click();
  await page.getByRole("dialog").getByRole("button", { name: "Confirmar" }).click();
  await expect(page.getByText("Sua biblioteca está vazia")).toBeVisible();
});

test("F10: populated library is accessible", async ({ page }) => {
  await open(page, "/library", "big");
  const results = await new AxeBuilder({ page }).analyze();
  expect(results.violations).toEqual([]);
});

test("F10: populated library fits a mobile viewport", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await open(page, "/library", "big");
  await expect(page.getByRole("table", { name: "Biblioteca" })).toBeVisible();
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(
    true,
  );
  const results = await new AxeBuilder({ page }).analyze();
  expect(results.violations).toEqual([]);
});
