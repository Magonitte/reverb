import { test, expect } from "@playwright/test";

test("F12 integration bookmarklet can be copied from Settings", async ({ page }) => {
  await page.goto("/#/settings/integration");
  const code = page.getByRole("textbox", { name: "Código do bookmarklet" });
  await expect(code).toHaveValue(
    "javascript:location.href='reverb://add?url='+encodeURIComponent(location.href)",
  );
  await page.getByRole("button", { name: "Copiar bookmarklet" }).click();
  await expect(page.getByText("Bookmarklet copiado", { exact: true })).toBeVisible();
});
