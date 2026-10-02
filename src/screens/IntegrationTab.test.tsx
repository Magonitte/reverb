import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it } from "vitest";
import { renderApp, resetFlowTests } from "@/testing/flow";
import { api } from "@/lib/ipc/api";
import { BOOKMARKLET } from "@/lib/ipc/mock/integration";
import { useSettingsStore } from "@/stores/settings";

beforeEach(async () => {
  resetFlowTests();
  await useSettingsStore.getState().load();
});
describe("F12 bookmarklet", () => {
  it("records an accelerator, clears it, and persists clipboard watching", async () => {
    const user = userEvent.setup();
    renderApp("/settings/integration");
    const field = await screen.findByRole("button", { name: "Atalho global" });
    await user.click(field);
    await user.keyboard("{Control>}{Shift>}D{/Shift}{/Control}");
    expect((await api.settingsGet()).globalShortcut).toBe("Ctrl+Shift+D");
    await user.click(screen.getByRole("button", { name: "Desligar atalho" }));
    expect((await api.settingsGet()).globalShortcut).toBe("");
    await user.click(screen.getByRole("switch", { name: "Observar links copiados" }));
    expect((await api.settingsGet()).clipboardWatch).toBe(true);
  });
  it("shows instructions and copies the exact bookmarklet through Rust IPC", async () => {
    renderApp("/settings/integration");
    const button = await screen.findByRole("button", { name: "Copiar bookmarklet" });
    await userEvent.click(button);
    expect(await api.clipboardReadText()).toBe(BOOKMARKLET);
    expect(screen.getByRole("textbox", { name: "Código do bookmarklet" })).toHaveValue(BOOKMARKLET);
    expect(screen.getByText(/salve como endereço de um favorito/)).toBeVisible();
  });
  it("test command enqueues a validated URL using the specified profile", async () => {
    await api.deeplinkTest("reverb://add?url=https%3A%2F%2Fyoutu.be%2FjNQXAC9IVRw&profile=opus_96");
    const jobs = await api.jobsList();
    expect(jobs).toHaveLength(1);
    expect(jobs[0]).toMatchObject({ sourceId: "jNQXAC9IVRw", profileId: "opus_96" });
    await expect(api.deeplinkTest("reverb://add?url=plain+text")).rejects.toMatchObject({
      kind: "invalid",
    });
    expect(await api.jobsList()).toHaveLength(1);
  });
});
