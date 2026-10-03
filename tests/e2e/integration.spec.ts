import { test, expect } from "@playwright/test";

test("F12 integration records and disables the global shortcut", async ({ page }) => {
  await page.goto("/#/settings/integration");
  const recorder = page.getByRole("button", { name: "Atalho global", exact: true });
  await recorder.click();
  await recorder.press("Control+Shift+D");
  await expect(recorder).toHaveText("Ctrl+Shift+D");
  await page.getByRole("button", { name: "Desligar atalho" }).click();
  await expect(recorder).toHaveText("Nenhum");
  const watch = page.getByRole("switch", { name: "Observar links copiados" });
  await watch.click();
  await expect(watch).toHaveAttribute("aria-checked", "true");
});

test("F12 integration bookmarklet can be copied from Settings", async ({ page }) => {
  await page.goto("/#/settings/integration");
  const code = page.getByRole("textbox", { name: "Código do bookmarklet" });
  await expect(code).toHaveValue(
    "javascript:location.href='reverb://add?url='+encodeURIComponent(location.href)",
  );
  await page.getByRole("button", { name: "Copiar bookmarklet" }).click();
  await expect(page.getByText("Bookmarklet copiado", { exact: true })).toBeVisible();
});
