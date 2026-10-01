import { expect, test } from "@playwright/test";

test("página carrega, mostra Reverb e não registra erros no console", async ({ page }) => {
  const errors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") errors.push(msg.text());
  });
  page.on("pageerror", (err) => errors.push(err.message));

  await page.goto("/");
  await expect(page.getByRole("heading", { name: "Reverb" })).toBeVisible();
  await expect(page.getByTestId("app-info")).toContainText("0.1.0");
  expect(errors).toEqual([]);
});
