import { onEvent } from "@/lib/ipc/events";
import { api } from "@/lib/ipc/api";
import { showClipboardNotice } from "./clipboard";
import { useUiStore, type ToastTone } from "./ui";
import i18n from "@/lib/i18n";
export interface NativeNotice {
  message: string;
  title: string;
  tone: ToastTone;
  url: string | null;
}
export function showNativeNotice(notice: NativeNotice) {
  if (notice.url) {
    showClipboardNotice(notice.url);
    return;
  }
  useUiStore
    .getState()
    .pushToast({
      title: notice.title,
      message: notice.message,
      tone: notice.tone,
      sound: true,
      durationMs: 12000,
      actionLabel: i18n.t("notifications.openReverb"),
      onAction: () => api.deeplinkTest("reverb://open"),
    });
}
export async function initNativeNotices(): Promise<() => void> {
  return onEvent<NativeNotice>("notification://notice", showNativeNotice);
}
