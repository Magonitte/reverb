import assert from "node:assert/strict";
import { invoke, waitForHome } from "../helpers.mjs";

describe("Notifications in the real Windows app", () => {
  it("fits the entire card in its measured native window", async () => {
    await waitForHome();
    const main = await browser.getWindowHandle();
    const existing = new Set((await invoke("jobs_list")).map((job) => job.id));
    let created;
    await invoke("queue_pause");
    try {
      await invoke("deeplink_test", {
        link: "reverb://add?url=https%3A%2F%2Fyoutu.be%2FjNQXAC9IVRw",
      });
      created = (await invoke("jobs_list")).find((job) => !existing.has(job.id));
      assert.ok(created, "isolated queued fixture must exist");
      await invoke("plugin:window|minimize", { label: "main" });
      await invoke("plugin:event|emit", {
        event: "job://updated",
        payload: { ...created, status: "failed", title: "Teste de notificação no Windows" },
      });
      await browser.waitUntil(async () => (await browser.getWindowHandles()).length > 1, {
        timeout: 30_000,
        timeoutMsg: "native notification window did not open",
      });
      const popup = (await browser.getWindowHandles()).find((handle) => handle !== main);
      await browser.switchToWindow(popup);
      const toast = await $('[data-testid="toast"]');
      await toast.waitForDisplayed();
      await toast.moveTo();
      const layout = await browser.execute(() => {
        const toast = document.querySelector('[data-testid="toast"]');
        const rect = toast.getBoundingClientRect();
        return {
          top: rect.top,
          bottom: rect.bottom,
          height: innerHeight,
          background: getComputedStyle(document.body).backgroundColor,
          title: toast.querySelector("p").textContent,
          position: getComputedStyle(toast.parentElement).position,
        };
      });
      assert.equal(layout.position, "relative");
      assert.ok(layout.top >= 16, JSON.stringify(layout));
      assert.ok(layout.bottom <= layout.height - 15, JSON.stringify(layout));
      assert.equal(layout.background, "rgba(0, 0, 0, 0)");
      assert.equal(layout.title, "Falha no download");
      await browser.switchToWindow(main);
      const position = await invoke("plugin:window|outer_position", { label: "notification" });
      const size = await invoke("plugin:window|outer_size", { label: "notification" });
      return { layout, position, size };
    } finally {
      await browser.switchToWindow(main);
      if (created) await invoke("job_cancel", { id: created.id });
      await invoke("deeplink_test", { link: "reverb://open" });
      await invoke("queue_resume");
    }
  });
});
