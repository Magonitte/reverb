import { expect, test } from "@playwright/test";
import { collectErrors } from "./helpers";

test("página carrega, mostra o Início e não registra erros no console", async ({ page }) => {
  const errors = collectErrors(page);

  await page.goto("/");
  await expect(page.getByRole("heading", { level: 1, name: "Início" })).toBeVisible();
  await expect(page.getByRole("textbox", { name: "Barra de comando" })).toBeVisible();
  await expect(page.getByTestId("sidebar")).toBeVisible();
  expect(errors).toEqual([]);
});
