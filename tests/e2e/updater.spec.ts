import { expect, test } from "@playwright/test";
import { open } from "./helpers";
import AxeBuilder from "@axe-core/playwright";

// T3 — cenário update-available: toast, ponto e instalação até "Reiniciando".
test("T3: atualização disponível mostra toast, ponto e progresso até reiniciar", async ({
  page,
}) => {
  await open(page, "/settings/updates", "update-available");
  await expect(page.getByRole("status")).toContainText("Reverb 0.1.1 está disponível.");
  await expect(page.getByTestId("dot-settings")).toBeVisible();
  const card = page.getByRole("region", { name: "Reverb" });
  await card
    .getByRole("switch", { name: "Verificar o app automaticamente" })
    .uncheck();
  await card.getByRole("button", { name: "Verificar atualização do Reverb" }).click();
  await expect(card.getByText("Versão instalada: 0.1.0")).toBeVisible();
  await page.getByRole("button", { name: "Atualizar agora" }).click();
  await expect(page.getByRole("progressbar", { name: "Baixando…" })).toBeVisible();
  await expect(page.getByTestId("updater-phase")).toHaveText("Reiniciando…", { timeout: 10_000 });
});
test("manual tool updates work with automatic updates disabled", async ({ page }, info) => {
  await open(page, "/settings/updates", "tools-update");
  await page.getByRole("switch", { name: "Atualizar ferramentas automaticamente" }).uncheck();
  await page.getByRole("button", { name: "Verificar atualizações" }).click();
  const ffmpeg = page.getByRole("listitem", { name: "FFmpeg" });
  await expect(ffmpeg.getByRole("button", { name: "Atualizar FFmpeg" })).toBeVisible();
  await page.screenshot({
    path: info.outputPath("tools-manual-update.png"),
    animations: "disabled",
  });
  expect((await new AxeBuilder({ page }).analyze()).violations).toEqual([]);
  await ffmpeg.getByRole("button", { name: "Atualizar FFmpeg" }).click();
  await expect(ffmpeg.getByText("Atualizado", { exact: true })).toBeVisible();
  await expect(ffmpeg.getByText("20260930T190056Z", { exact: true })).toBeVisible();
  await expect(ffmpeg).toBeVisible();
});
