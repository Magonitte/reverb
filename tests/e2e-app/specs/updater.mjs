import { mkdirSync, readFileSync } from "node:fs";
import { byText, invoke, openTab, waitForHome } from "../helpers.mjs";
const { version } = JSON.parse(readFileSync(new URL("../../../package.json", import.meta.url), "utf8"));

describe("Reverb updater in the real app", () => {
  it("checks manually from Settings when automatic checking is disabled", async () => {
    await waitForHome();
    const previous = await invoke("settings_get");
    try {
      await invoke("settings_update", { patch: { autoCheckAppUpdates: false } });
      await $('[data-testid="nav-settings"]').click();
      await openTab("Atualizações");
      const card = await $('[role="region"][aria-labelledby="updates-app-title"]');
      await expect(card).toHaveText(expect.stringContaining(`Versão instalada: ${version}`));
      await (await byText("button", "Verificar atualização do Reverb")).click();
      await browser.waitUntil(
        async () => {
          const text = await $('[data-testid="updater-phase"]').getText();
          if (text === "Erro ao atualizar") throw new Error(await card.getText());
          return text === "Atualizado" || text === "Atualização disponível";
        },
        { timeout: 60000 },
      );
      mkdirSync("test-results/native", { recursive: true });
      await browser.saveScreenshot("test-results/native/reverb-updater.png");
    } finally {
      await invoke("settings_update", {
        patch: { autoCheckAppUpdates: previous.autoCheckAppUpdates },
      });
    }
  });
});
