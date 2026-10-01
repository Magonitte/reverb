import { expect, test } from "@playwright/test";
import { open } from "./helpers";

// T11 — cenários do backend falso (?scenario=).
test("T11: cenário busy mostra o badge de contagem na sidebar", async ({ page }) => {
  await open(page, "/", "busy");
  await expect(page.getByTestId("badge-activity")).toHaveText("5");
  await expect(page.getByTestId("job-card")).toHaveCount(3);
});

test("T11: cenário empty não mostra badge", async ({ page }) => {
  await open(page, "/", "empty");
  await expect(page.getByTestId("badge-activity")).toHaveCount(0);
  await expect(page.getByTestId("empty-state")).toBeVisible();
});

test("T11: cenário heal mostra o banner de autocura", async ({ page }) => {
  await open(page, "/", "heal");
  await expect(page.getByTestId("heal-banner")).toContainText(
    "O YouTube mudou algo — atualizando o yt-dlp…",
  );
});

test("T11: cenário errors lista as falhas na Atividade", async ({ page }) => {
  await open(page, "/activity", "errors");
  await page.getByRole("tab", { name: /Falhas/ }).click();
  await expect(page.getByTestId("job-card")).toHaveCount(2);
});

test("T11: sem cenário, a simulação temporizada anda (enfileirar pelo console do mock)", async ({
  page,
}) => {
  await page.goto("/#/activity");
  await expect(page.getByRole("heading", { level: 1, name: "Atividade" })).toBeVisible();
  await page.evaluate(async () => {
    const modulePath = "/src/lib/ipc/mock/index.ts";
    const { mockCall } = await import(/* @vite-ignore */ modulePath);
    await mockCall("enqueue", { request: { url: "https://www.youtube.com/watch?v=jNQXAC9IVRw" } });
  });
  await expect(page.getByTestId("badge-activity")).toHaveText("1");
  await expect(page.getByTestId("job-card")).toHaveCount(1);
  await page.getByRole("tab", { name: /Concluídos/ }).click();
  await expect(page.getByTestId("job-card")).toHaveCount(1, { timeout: 15_000 });
});
