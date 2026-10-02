// F11 T9: create through the actual dialog, run and observe a downloaded detail row.
import { byText, invoke, waitForHome } from "../helpers.mjs";
import { existsSync } from "node:fs";

describe("F11 playlist synchronization in the real app", () => {
  it("creates a one-track manual sync and downloads it into the detail view", async () => {
    await waitForHome();
    await $('[data-testid="nav-playlists"]').click();
    const buttons = await $$('//button[normalize-space()="Criar sincronização"]');
    await buttons[0].click();
    const field = async (label) => {
      const element = await $(`//label[normalize-space()="${label}"]`);
      const id = await element.getAttribute("for");
      return $(`[id="${id}"]`);
    };
    await (
      await field("URL da playlist, álbum ou canal")
    ).setValue("https://www.youtube.com/playlist?list=OLAK5uy_nmDUsWOMoEcz0SsVqUwir0oxu-k1oUyXE");
    await (await field("Nome da playlist")).setValue("F11 App playlist");
    await (await field("Perfil de áudio")).selectByAttribute("value", "opus_96");
    await (await field("Limitar às primeiras faixas")).setValue("1");
    await (await field("Intervalo de sincronização")).selectByAttribute("value", "0");
    await $('//div[@role="dialog"]//button[normalize-space()="Criar sincronização"]').click();
    const run = await byText("button", "Sincronizar agora");
    await run.waitForClickable({ timeout: 180000 });
    await run.click();
    let synced;
    await browser.waitUntil(
      async () => {
        const syncs = await invoke("syncs_list");
        synced = syncs.find((sync) => sync.title === "F11 App playlist");
        if (!synced?.lastResult) return false;
        if (!synced.lastResult.running && synced.lastResult.failed)
          throw new Error(JSON.stringify(synced.lastResult));
        return !synced.lastResult.running;
      },
      { timeout: 360000, interval: 1000, timeoutMsg: "playlist did not finish downloading" },
    );
    const items = await invoke("sync_items", { id: synced.id });
    expect(items).toHaveLength(1);
    expect(items[0].libraryId).toBeGreaterThan(0);
    expect(existsSync(items[0].filePath)).toBe(true);
    const table = await $('[role="table"][aria-label="Faixas sincronizadas"]');
    await table.waitForDisplayed();
    await expect(table).toHaveText(expect.stringContaining("Never Gonna Give You Up"));
    await expect(table).toHaveText(expect.stringContaining("Baixada"));
  });
});
