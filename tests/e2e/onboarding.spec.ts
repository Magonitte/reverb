import { test, expect } from "@playwright/test";
for (const scenario of ["onboarding", "onboarding-error"]) {
  test(`F12 first run finishes setup (${scenario})`, async ({ page }) => {
    await page.goto(`/?scenario=${scenario}#/`);
    await expect(
      page.getByRole("heading", { level: 1, name: "Bem-vindo ao Reverb" }),
    ).toBeVisible();
    await page.getByRole("button", { name: "Continuar" }).click();
    await expect(page.getByRole("button", { name: "Continuar" })).toBeDisabled();
    await page.getByRole("button", { name: "Escolher pasta" }).click();
    await page.getByRole("button", { name: "Continuar" }).click();
    await expect(page.getByRole("button", { name: "Continuar" })).toBeDisabled();
    await page.getByRole("button", { name: "Instalar ferramentas" }).click();
    if (scenario === "onboarding-error") {
      await expect(page.getByRole("alert")).toContainText("A instalação falhou");
      await page.getByRole("button", { name: "Tentar de novo" }).click();
    }
    await expect(page.getByText("Ferramentas prontas")).toBeVisible();
    for (let index = 0; index < 3; index++)
      await page.getByRole("button", { name: "Continuar" }).click();
    await page.getByRole("button", { name: "Começar a usar" }).click();
    await expect(page.getByRole("textbox", { name: "Barra de comando" })).toBeVisible();
  });
}
