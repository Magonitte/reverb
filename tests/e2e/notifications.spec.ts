import { expect, test, type Page } from "@playwright/test";
import { resolve } from "node:path";
import AxeBuilder from "@axe-core/playwright";
import { open, goHash } from "./helpers";
async function copyLink(page: Page, url = "https://youtu.be/jNQXAC9IVRw") {
  await page.evaluate(async (url) => {
    const path = "/src/lib/ipc/mock/index.ts";
    const { mockCall, setMockClipboard } = await import(/* @vite-ignore */ path);
    await mockCall("settings_update", { patch: { clipboardWatch: true } });
    setMockClipboard(url);
  }, url);
}
async function jobCount(page: Page) {
  return page.evaluate(async () => {
    const path = "/src/lib/ipc/mock/index.ts";
    const { mockCall } = await import(/* @vite-ignore */ path);
    return (await mockCall("jobs_list")).length;
  });
}

test("clipboard confirmation, cancellation, timer pause and expiry", async ({ page }) => {
  await open(page);
  await copyLink(page);
  const toast = page.getByTestId("toast").filter({ hasText: "Link de música copiado" });
  await expect(toast.getByRole("button", { name: "Baixar", exact: true })).toBeVisible();
  await expect(toast.getByRole("button", { name: "Cancelar", exact: true })).toBeVisible();
  await expect(toast).toContainText("Original (sem recodificar)");
  expect(await jobCount(page)).toBe(0);
  await toast.getByRole("button", { name: "Cancelar" }).click();
  expect(await jobCount(page)).toBe(0);
  await copyLink(page, "https://youtu.be/dQw4w9WgXcQ");
  await expect(toast).toBeVisible();
  await page.clock.install();
  await toast.hover();
  await expect(toast).toContainText("Tempo pausado");
  const scale = await toast.getByTestId("toast-timer").getAttribute("style");
  await page.clock.runFor(60000);
  await expect(toast.getByTestId("toast-timer")).toHaveAttribute("style", scale!);
  await page.mouse.move(0, 0);
  await expect(toast).not.toContainText("Tempo pausado");
  await page.clock.runFor(36000);
  await expect(page.getByText("Link de música copiado", { exact: true })).toHaveCount(0);
  expect(await jobCount(page)).toBe(0);
});

test("clipboard downloads only after confirmation", async ({ page }) => {
  await open(page);
  await copyLink(page);
  expect(await jobCount(page)).toBe(0);
  await page.getByTestId("toast").getByRole("button", { name: "Baixar", exact: true }).click();
  await expect(page.getByText("Adicionado à fila", { exact: true })).toBeVisible();
  expect(await jobCount(page)).toBe(1);
});

test("custom sound loads, plays and persists after reload", async ({ page }) => {
  await open(page, "/settings");
  await page
    .getByLabel("Carregar som personalizado", { exact: true })
    .setInputFiles(resolve("src/assets/sounds/glass.wav"));
  await expect(page.getByLabel("Som da notificação")).toHaveValue("custom");
  await page.getByRole("button", { name: "Ouvir som", exact: true }).click();
  await page.evaluate(async () => {
    const path = "/src/lib/notificationSound.ts";
    const { playNotificationSound } = await import(/* @vite-ignore */ path);
    await playNotificationSound(true);
  });
  await expect(page.getByText("Não foi possível tocar esse som", { exact: false })).toHaveCount(0);
  await page.getByLabel("Tempo para decidir sobre um link copiado").selectOption("60");
  await page.reload();
  await expect(page.getByLabel("Som da notificação")).toHaveValue("custom");
  await expect(page.getByLabel("Tempo para decidir sobre um link copiado")).toHaveValue("60");
  await page.getByRole("button", { name: "Remover som personalizado" }).click();
  await expect(page.getByLabel("Som da notificação")).toHaveValue("glass");
  await page
    .getByLabel("Carregar som personalizado", { exact: true })
    .setInputFiles({ name: "invalid.wav", mimeType: "audio/wav", buffer: Buffer.from("invalid") });
  await expect(page.getByText(/Escolha um arquivo de áudio válido/)).toBeVisible();
  await expect(page.getByLabel("Som da notificação")).toHaveValue("glass");
});

for (const theme of ["dark", "light"] as const)
  test(`notifications and sound settings accessible in ${theme}`, async ({ page }) => {
    await open(page, "/settings");
    await page.getByLabel("Tema", { exact: true }).selectOption(theme);
    await page.getByRole("button", { name: "Testar notificação" }).click();
    await expect(page.getByTestId("toast")).toContainText("Sua música está pronta");
    await page.getByTestId("toast").hover();
    await expect(page.getByTestId("toast")).toContainText("Tempo pausado");
    expect((await new AxeBuilder({ page }).analyze()).violations).toEqual([]);
    await expect(page).toHaveScreenshot(`notification-settings-${theme}.png`, {
      animations: "disabled",
      maxDiffPixelRatio: 0.002,
      mask: [page.getByTestId("toast-timer")],
    });
    await goHash(page, "/activity");
  });

test("Activity uses the same spinning disc as Home", async ({ page }) => {
  await open(page, "/activity", "busy");
  await expect(page.locator('[data-status="running"] [data-spinning="true"]')).toHaveCount(2);
  await expect(page.locator('[data-status="queued"] [data-spinning="false"]')).toHaveCount(3);
});

for (const theme of ["dark", "light"] as const)
  test(`notification popup follows the ${theme} theme and presents the clipboard decision`, async ({
    page,
  }) => {
    await page.setViewportSize({ width: 452, height: 420 });
    await page.goto("/?scenario=empty#/notification");
    await page.evaluate(async (theme) => {
      const path = "/src/stores/settings.ts";
      const { useSettingsStore } = await import(/* @vite-ignore */ path);
      await useSettingsStore.getState().load();
      await useSettingsStore.getState().update({ theme });
    }, theme);
    await expect(page.locator("html")).toHaveAttribute("data-theme", theme);
    await page.evaluate(async () => {
      const path = "/src/stores/nativeNotices.ts";
      const { showNativeNotice } = await import(/* @vite-ignore */ path);
      showNativeNotice({
        title: "Reverb",
        message: "",
        tone: "info",
        url: "https://www.youtube.com/watch?v=jNQXAC9IVRw",
      });
    });
    const toast = page.getByTestId("toast");
    await expect(toast.getByRole("button", { name: "Baixar", exact: true })).toBeVisible();
    await expect(toast.getByRole("button", { name: "Cancelar", exact: true })).toBeVisible();
    await toast.hover();
    await expect(toast).toContainText("Tempo pausado");
    const colors = await toast.evaluate((element) => {
      const root = getComputedStyle(document.documentElement);
      const timer = element.querySelector('[data-testid="toast-timer"]')!;
      const normalize = (value: string) => {
        const probe = document.createElement("span");
        probe.style.color = value;
        document.body.append(probe);
        const color = getComputedStyle(probe).color;
        probe.remove();
        return color;
      };
      return {
        background: getComputedStyle(element).backgroundColor,
        expectedBackground: normalize(root.getPropertyValue("--bg-base")),
        accent: getComputedStyle(timer).backgroundColor,
        expectedAccent: normalize(root.getPropertyValue("--accent")),
      };
    });
    expect(colors.background).toBe(colors.expectedBackground);
    expect(colors.accent).toBe(colors.expectedAccent);
    expect((await new AxeBuilder({ page }).analyze()).violations).toEqual([]);
    await page.screenshot({ path: `docs/notificacao-link-${theme}.png`, animations: "disabled" });
    await toast.getByRole("button", { name: "Cancelar" }).click();
    await expect(toast).toHaveCount(0);
  });

test("clipboard notification stays paused behind a modal and cannot cover its buttons", async ({
  page,
}) => {
  await open(page);
  await copyLink(page);
  const input = page.getByRole("textbox", { name: "Barra de comando" });
  await input.fill("https://youtu.be/dQw4w9WgXcQ");
  await input.press("Enter");
  await expect(page.getByRole("dialog", { name: "Pré-visualização" })).toBeVisible();
  await expect(page.getByTestId("toast")).toBeHidden();
  await page.clock.install();
  await page.clock.fastForward(60000);
  await page
    .getByRole("dialog", { name: "Pré-visualização" })
    .getByRole("button", { name: "Fechar" })
    .click();
  await expect(page.getByTestId("toast")).toBeVisible();
  expect(await jobCount(page)).toBe(0);
});
