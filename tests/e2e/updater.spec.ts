import { expect, test } from "@playwright/test";
import { open } from "./helpers";

// T3 — cenário update-available: toast, ponto e instalação até "Reiniciando".
test("T3: atualização disponível mostra toast, ponto e progresso até reiniciar", async ({ page }) => {
  await open(page, "/settings/updates", "update-available");
  await expect(page.getByRole("status")).toContainText("Reverb 0.1.1 está disponível.");
  await expect(page.getByTestId("dot-settings")).toBeVisible();
  await page.getByRole("button", { name: "Atualizar agora" }).click();
  await expect(page.getByRole("progressbar", { name: "Baixando…" })).toBeVisible();
  await expect(page.getByTestId("updater-phase")).toHaveText("Reiniciando…", { timeout: 10_000 });
});
