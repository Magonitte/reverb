import { expect, test } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";
import { open, goHash, setTheme } from "./helpers";

const URL = "https://www.youtube.com/playlist?list=OLAK5uy_nmDUsWOMoEcz0SsVqUwir0oxu-k1oUyXE";
test("F11 creates a limited manual sync, runs it and confirms deletion", async ({ page }) => {
  await open(page, "/playlists");
  await page.getByRole("button", { name: "Criar sincronização", exact: true }).first().click();
  const dialog = page.getByRole("dialog");
  await dialog.getByLabel("URL da playlist, álbum ou canal").fill(URL);
  await dialog.getByLabel("Limitar às primeiras faixas").fill("0");
  await dialog.getByRole("button", { name: "Criar sincronização", exact: true }).click();
  await expect(dialog).toBeVisible();
  await dialog.getByLabel("Limitar às primeiras faixas").fill("2");
  await dialog.getByLabel("Intervalo de sincronização").selectOption("0");
  await dialog.getByRole("button", { name: "Criar sincronização", exact: true }).click();
  await expect(page.getByRole("heading", { name: /Album - Whenever/ })).toBeVisible();
  await page.getByRole("button", { name: "Sincronizar agora" }).click();
  const table = page.getByRole("table", { name: "Faixas sincronizadas" });
  await expect(table.getByRole("row")).toHaveCount(3);
  await expect(table).toContainText("Never Gonna Give You Up");
  expect((await new AxeBuilder({ page }).analyze()).violations).toEqual([]);
  await page.getByRole("button", { name: "Excluir", exact: true }).click();
  await expect(page.getByRole("dialog")).toBeVisible();
  await page.getByRole("dialog").getByRole("button", { name: "Cancelar" }).click();
  await expect(table).toBeVisible();
  await page.getByRole("button", { name: "Excluir", exact: true }).click();
  await page.getByRole("dialog").getByRole("checkbox").check();
  await page.getByRole("dialog").getByRole("button", { name: "Excluir", exact: true }).click();
  await expect(page.getByTestId("sync-card")).toHaveCount(0);
});
for (const theme of ["dark", "light"] as const)
  test(`F11 playlists, detail and dialog visual ${theme}`, async ({ page }) => {
    test.skip(process.platform !== "win32", "Visual baselines use Windows fonts");
    await open(page, "/playlists", "playlists");
    await setTheme(page, theme);
    await goHash(page, "/playlists");
    await expect(page.getByTestId("sync-card")).toBeVisible();
    expect((await new AxeBuilder({ page }).analyze()).violations).toEqual([]);
    await expect(page).toHaveScreenshot(`playlists-${theme}.png`);
    await page.getByRole("link", { name: /Album - Whenever/ }).click();
    await expect(page.getByRole("table")).toContainText("Removida da playlist");
    expect((await new AxeBuilder({ page }).analyze()).violations).toEqual([]);
    await expect(page).toHaveScreenshot(`playlist-detail-${theme}.png`);
    await page.getByRole("button", { name: "Editar sincronização" }).click();
    expect((await new AxeBuilder({ page }).analyze()).violations).toEqual([]);
    await expect(page).toHaveScreenshot(`sync-dialog-${theme}.png`);
    await page.getByRole("button", { name: "Cancelar" }).click();
    await goHash(page, "/");
    await expect(page.getByTestId("sync-summary")).toBeVisible();
    await page.setViewportSize({ width: 390, height: 844 });
    await goHash(page, "/playlists/sync-1");
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(
      true,
    );
  });
