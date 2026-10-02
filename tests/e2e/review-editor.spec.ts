import { expect, test } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";
import { open, goHash, setTheme } from "./helpers";

test("F10 review keyboard and badge", async ({ page }) => {
  await open(page, "/review", "big");
  await expect(page.getByText("1 de 500 para revisar")).toBeVisible();
  await page.keyboard.press("j");
  await expect(page.getByText("2 de 500 para revisar")).toBeVisible();
  await page.keyboard.press("k");
  await expect(page.getByText("1 de 500 para revisar")).toBeVisible();
  await page.getByRole("radio").click();
  await page.keyboard.press("1");
  await expect(page.getByRole("radio")).toBeChecked();
  await page.keyboard.press("Enter");
  await expect(page.getByText("1 de 499 para revisar")).toBeVisible();
  await expect(page.getByRole("button", { name: "Manter metadados atuais" })).toBeEnabled();
  await page.keyboard.press("m");
  await expect(page.getByText("1 de 498 para revisar")).toBeVisible();
  await expect(page.getByTestId("badge-review")).toHaveText("498");
  expect((await new AxeBuilder({ page }).analyze()).violations).toEqual([]);
});

test("F10 editor opens, changes artwork, searches and saves external tags", async ({ page }) => {
  await open(page, "/tag-editor");
  await page.getByRole("button", { name: /Abrir arquivo de áudio/ }).click();
  await expect(page.getByLabel("Título", { exact: true })).toHaveValue("example");
  await page.getByLabel("Título", { exact: true }).fill("Rick Astley");
  await page.getByRole("button", { name: /Escolher arquivo de capa/ }).click();
  await expect(page.getByRole("img", { name: "Capa do álbum" })).toBeVisible();
  await page.getByRole("button", { name: "Buscar metadados" }).click();
  await page.getByRole("button", { name: "Usar metadados" }).click();
  await expect(page.getByLabel("Título", { exact: true })).toHaveValue("Never Gonna Give You Up");
  await page.getByRole("button", { name: "Salvar", exact: true }).click();
  await expect(page.getByText("Tags salvas.")).toBeVisible();
  expect((await new AxeBuilder({ page }).analyze()).violations).toEqual([]);
});

for (const theme of ["dark", "light"] as const) {
  test(`F10 populated screens in ${theme}`, async ({ page }) => {
    await open(page, "/library", "big");
    await setTheme(page, theme);
    await goHash(page, "/library");
    await expect(page).toHaveScreenshot(`library-populated-${theme}.png`, {
      animations: "disabled",
    });
    await page.getByRole("button", { name: "Álbuns", exact: true }).click();
    await expect(
      page.getByRole("button", { name: "Whenever You Need Somebody", exact: true }),
    ).toBeVisible();
    await expect(page).toHaveScreenshot(`library-albums-${theme}.png`, { animations: "disabled" });
    await goHash(page, "/review");
    await expect(page.getByRole("radio")).toBeVisible();
    expect((await new AxeBuilder({ page }).analyze()).violations).toEqual([]);
    await expect(page).toHaveScreenshot(`review-populated-${theme}.png`, {
      animations: "disabled",
    });
    await goHash(page, "/tag-editor");
    await page.getByRole("button", { name: /Abrir arquivo de áudio/ }).click();
    await expect(page.getByLabel("Título", { exact: true })).toHaveValue("example");
    expect((await new AxeBuilder({ page }).analyze()).violations).toEqual([]);
    await expect(page).toHaveScreenshot(`editor-populated-${theme}.png`, {
      animations: "disabled",
    });
    await page.setViewportSize({ width: 390, height: 844 });
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(
      true,
    );
  });
}
