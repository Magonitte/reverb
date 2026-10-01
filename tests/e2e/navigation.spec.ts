import { expect, test } from "@playwright/test";
import { collectErrors, open, settle } from "./helpers";

// T7 — navegação por sidebar e por atalhos.
const SIDEBAR: Array<[string, string, string]> = [
  ["library", "Biblioteca", "/library"],
  ["playlists", "Playlists", "/playlists"],
  ["activity", "Atividade", "/activity"],
  ["review", "Revisar", "/review"],
  ["settings", "Configurações", "/settings/"],
  ["home", "Início", "/"],
];

test("T7: cada item da sidebar leva à rota certa, com título e sem erros no console", async ({
  page,
}) => {
  const errors = collectErrors(page);
  await open(page, "/");

  for (const [nav, title, hash] of SIDEBAR) {
    await page.getByTestId(`nav-${nav}`).click();
    await expect(page.getByRole("heading", { level: 1, name: title })).toBeVisible();
    await expect.poll(() => new URL(page.url()).hash).toContain(`#${hash.replace(/\/$/, "")}`);
    await expect(page.getByTestId(`nav-${nav}`)).toHaveAttribute("aria-current", "page");
    await settle(page);
  }
  expect(errors).toEqual([]);
});

test("T7: cada atalho leva à rota certa", async ({ page }) => {
  const errors = collectErrors(page);
  await open(page, "/review");

  const cases: Array<[string, string, string]> = [
    ["Control+q", "Atividade", "#/activity"],
    ["Control+h", "Biblioteca", "#/library"],
    ["Control+,", "Configurações", "#/settings"],
    ["Control+d", "Início", "#/"],
  ];
  for (const [keys, title, hash] of cases) {
    await page.keyboard.press(keys);
    await expect(page.getByRole("heading", { level: 1, name: title })).toBeVisible();
    await expect.poll(() => new URL(page.url()).hash).toBe(hash);
  }
  expect(errors).toEqual([]);
});

test("T7: Ctrl+K abre a barra de comando focada e Esc fecha", async ({ page }) => {
  await open(page, "/library");
  await page.keyboard.press("Control+k");
  const overlay = page.getByTestId("command-bar-overlay");
  await expect(overlay).toBeVisible();
  await expect(overlay.getByRole("textbox", { name: "Barra de comando" })).toBeFocused();
  await page.keyboard.type("rick astley");
  await expect(overlay.getByRole("textbox")).toHaveValue("rick astley");
  await page.keyboard.press("Escape");
  await expect(overlay).toBeHidden();
});

test("T7: Espaço não pausa a fila dentro do campo de busca", async ({ page }) => {
  await open(page, "/");
  const input = page.getByRole("textbox", { name: "Barra de comando" });
  await input.click();
  await page.keyboard.type("a b");
  await expect(input).toHaveValue("a b");
});

test("T7: o tooltip da sidebar aparece no foco do teclado", async ({ page }) => {
  await open(page, "/");
  await page.getByTestId("nav-library").focus();
  await expect(page.getByRole("tooltip")).toHaveText("Biblioteca");
});

test("T7: Configurações > Aparência troca o idioma sem recarregar", async ({ page }) => {
  await open(page, "/settings");
  await page.getByLabel("Idioma").selectOption("en");
  await expect(page.getByTestId("nav-home")).toHaveAttribute("aria-label", "Home");
  await expect(page.getByRole("heading", { level: 1, name: "Settings" })).toBeVisible();
  await page.getByLabel("Language").selectOption("pt-BR");
  await expect(page.getByTestId("nav-home")).toHaveAttribute("aria-label", "Início");
});
