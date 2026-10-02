import { api } from "@/lib/ipc/api";
import { onEvent } from "@/lib/ipc/events";
import i18n from "@/lib/i18n";
import { useUiStore } from "./ui";

export async function initClipboard(): Promise<() => void> {
  return onEvent<{ url: string; visible?: boolean }>("clipboard://url", ({ url, visible }) => {
    if (visible === false) return;
    const ui = useUiStore.getState();
    ui.pushToast({
      message: i18n.t("integration.clipboardDetected"),
      actionLabel: i18n.t("integration.downloadCopied"),
      onAction: () => {
        void api.deeplinkTest(`reverb://add?url=${encodeURIComponent(url)}`).catch(() => {
          ui.pushToast({ message: i18n.t("integration.linkFailed"), tone: "error" });
        });
      },
      secondaryActionLabel: i18n.t("integration.analyzeCopied"),
      onSecondaryAction: () => {
        ui.openCommandBar(url);
        ui.requestCommandBarSubmit();
      },
    });
  });
}
