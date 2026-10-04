import { beforeEach, describe, expect, it } from "vitest";
import { resetFlowTests } from "@/testing/flow";
import { mockBus, setMockClipboard } from "@/lib/ipc/mock";
import { api } from "@/lib/ipc/api";
import { initClipboard } from "./clipboard";
import { useSettingsStore } from "./settings";
import { useUiStore } from "./ui";
import { DEFAULT_NOTIFICATIONS, useNotificationPreferences } from "./notificationPreferences";

beforeEach(async () => {
  resetFlowTests();
  useNotificationPreferences.setState({ preferences: DEFAULT_NOTIFICATIONS });
  await useSettingsStore.getState().load();
});
describe("clipboard URL notices", () => {
  it("requires explicit download confirmation and uses the chosen reading time", async () => {
    const off = await initClipboard();
    try {
      await useSettingsStore.getState().update({ clipboardWatch: true });
      setMockClipboard("https://youtu.be/jNQXAC9IVRw");
      setMockClipboard("https://www.youtube.com/watch?v=jNQXAC9IVRw");
      const [toast] = useUiStore.getState().toasts;
      expect(useUiStore.getState().toasts).toHaveLength(1);
      expect(toast).toMatchObject({
        actionLabel: "Baixar",
        secondaryActionLabel: "Cancelar",
        durationMs: 35000,
        details: "https://www.youtube.com/watch?v=jNQXAC9IVRw",
      });
      expect(await api.jobsList()).toHaveLength(0);
      toast.onSecondaryAction?.();
      expect(await api.jobsList()).toHaveLength(0);
      await toast.onAction?.();
      expect(await api.jobsList()).toHaveLength(1);
      mockBus.emit("clipboard://url", { url: "https://youtu.be/dQw4w9WgXcQ", visible: false });
      expect(useUiStore.getState().toasts.filter((t) => t.group === "clipboard")).toHaveLength(1);
    } finally {
      off();
    }
  });
});
