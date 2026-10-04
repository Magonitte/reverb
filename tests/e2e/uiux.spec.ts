import { expect, test } from "@playwright/test";
import { open } from "./helpers";

test("UI/UX: botão explícito pesquisa texto e analisa link", async ({ page }) => {
  await open(page);
  const input = page.getByRole("textbox", { name: "Barra de comando" });
  await expect(page.getByRole("button", { name: "Pesquisar", exact: true })).toBeDisabled();
  await input.fill("rick astley");
  await page.getByRole("button", { name: "Pesquisar", exact: true }).click();
  await expect(page.getByTestId("search-result")).toHaveCount(3);
  await input.fill("https://youtu.be/jNQXAC9IVRw");
  await page.getByRole("button", { name: "Analisar link", exact: true }).click();
  await expect(page.getByTestId("preview")).toBeVisible();
  await expect(page.getByTestId("preview-title")).toHaveText("Me at the zoo");
});

test("UI/UX: menu recolhido persiste e pode ser expandido", async ({ page }) => {
  await open(page, "/", "busy");
  await page.getByRole("button", { name: "Recolher menu" }).click();
  await expect(page.getByTestId("sidebar")).toHaveAttribute("data-collapsed", "true");
  await page.reload();
  await expect(page.getByRole("button", { name: "Expandir menu" })).toBeVisible();
  await page.getByRole("button", { name: "Expandir menu" }).click();
  await expect(page.getByTestId("sidebar")).toHaveAttribute("data-collapsed", "false");
});

test("UI/UX: sugere artistas sem enviar o formulário", async ({ page }) => {
  await open(page, "/library");
  await page.getByRole("button", { name: "Artistas seguidos", exact: true }).click();
  await page.getByRole("button", { name: "Seguir artista", exact: true }).click();
  const dialog = page.getByRole("dialog", { name: "Seguir artista" });
  await dialog.getByLabel("Buscar artista").fill("Rick Astley");
  await expect(dialog.getByRole("button", { name: /Rick Astley.*Deezer/ })).toBeVisible();
  await dialog.getByRole("button", { name: /Rick Astley.*Deezer/ }).click();
  await expect(dialog.getByText("Artista selecionado: Rick Astley", { exact: true })).toBeVisible();
  await expect(dialog.getByRole("button", { name: "Salvar", exact: true })).toBeEnabled();
});

test("UI/UX: ações aparecem com mouse e foco de teclado", async ({ page }) => {
  await open(page, "/library", "busy");
  const actions = page.locator(".library-row-actions").first();
  await page.mouse.move(0, 0);
  await expect(actions).toHaveCSS("opacity", "0");
  await actions.locator("button").first().focus();
  await expect(actions).toHaveCSS("opacity", "1");
  await page.getByRole("heading", { level: 1 }).click();
  await expect(actions).toHaveCSS("opacity", "0");
  await actions.hover();
  await expect(actions).toHaveCSS("opacity", "1");
});
