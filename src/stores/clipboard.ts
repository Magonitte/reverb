import { api } from "@/lib/ipc/api";
import { onEvent } from "@/lib/ipc/events";
import i18n from "@/lib/i18n";
import { useNotificationPreferences } from "./notificationPreferences";
import { useSettingsStore } from "./settings";
import { useUiStore } from "./ui";

export function showClipboardNotice(url: string): void {
  const ui = useUiStore.getState();
  const profile = useSettingsStore.getState().settings?.defaultProfile ?? "original";
  ui.pushToast({
    title: i18n.t("notifications.clipboardTitle"),
    message: i18n.t("notifications.clipboardMessage", {
      format: i18n.t(`profiles.${profile}`, { defaultValue: profile }),
    }),
    details: url,
    group: "clipboard",
    durationMs: useNotificationPreferences.getState().preferences.clipboardSeconds * 1000,
    sound: true,
    actionLabel: i18n.t("integration.downloadCopied"),
    onAction: async () => {
      await api.deeplinkTest(`reverb://add?url=${encodeURIComponent(url)}`);
      ui.pushToast({
        group: "queue-feedback",
        title: i18n.t("notifications.queuedTitle"),
        message: i18n.t("notifications.queuedMessage"),
        tone: "success",
        details: url,
      });
    },
    secondaryActionLabel: i18n.t("common.cancel"),
  });
}
export async function initClipboard(): Promise<() => void> {
  return onEvent<{ url: string; visible?: boolean }>("clipboard://url", ({ url, visible }) => {
    if (visible === false) return; // A dedicated native popup owns background notices.
    showClipboardNotice(url);
  });
}
