import { beforeEach, describe, expect, it } from "vitest";
import { resetFlowTests } from "@/testing/flow";
import { mockBus, setMockClipboard } from "@/lib/ipc/mock";
import { api } from "@/lib/ipc/api";
import { initClipboard } from "./clipboard";
import { useSettingsStore } from "./settings";
import { useUiStore } from "./ui";

beforeEach(async () => {
  resetFlowTests();
  await useSettingsStore.getState().load();
});
describe("clipboard URL notices", () => {
  it("offers download and analyze and deduplicates the mock clipboard", async () => {
    const off = await initClipboard();
    try {
      await useSettingsStore.getState().update({ clipboardWatch: true });
      setMockClipboard("https://youtu.be/jNQXAC9IVRw");
      setMockClipboard("https://www.youtube.com/watch?v=jNQXAC9IVRw");
      const toasts = useUiStore.getState().toasts;
      expect(toasts).toHaveLength(1);
      expect(toasts[0]).toMatchObject({ actionLabel: "Baixar", secondaryActionLabel: "Analisar" });
      toasts[0].onSecondaryAction?.();
      expect(useUiStore.getState()).toMatchObject({
        commandBarOpen: true,
        commandBarText: "https://www.youtube.com/watch?v=jNQXAC9IVRw",
        commandBarSubmitTick: 1,
      });
      toasts[0].onAction?.();
      expect(await api.jobsList()).toHaveLength(1);
      mockBus.emit("clipboard://url", { url: "https://youtu.be/dQw4w9WgXcQ", visible: false });
      expect(useUiStore.getState().toasts).toHaveLength(1);
    } finally {
      off();
    }
  });
});
