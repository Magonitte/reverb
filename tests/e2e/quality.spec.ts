import { test, expect } from "@playwright/test";
test("F13 cookie source is saved and tested", async ({ page }) => {
  await page.goto("/#/settings/downloads");
  await page.getByRole("combobox", { name: "Origem dos cookies" }).selectOption("firefox");
  await page.getByRole("button", { name: "Testar cookies", exact: true }).click();
  await expect(page.getByRole("status", { name: "Testar cookies" })).toContainText(
    "Premium disponível · AAC 256 kbps",
  );
});
