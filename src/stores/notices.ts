import { onEvent } from "@/lib/ipc/events";
import i18n from "@/lib/i18n";
import { useUiStore, type ToastTone } from "./ui";

interface NoticePayload {
  level: string;
  i18nKey: string;
  params?: Record<string, unknown>;
}

const TONES: Record<string, ToastTone> = {
  info: "info",
  success: "success",
  warning: "warning",
  error: "error",
};

/** Eventos `notice` do backend viram toasts traduzidos. */
export async function initNotices(): Promise<() => void> {
  return onEvent<NoticePayload>("notice", (n) => {
    useUiStore.getState().pushToast({
      message: i18n.t(n.i18nKey, n.params ?? {}),
      tone: TONES[n.level] ?? "info",
    });
  });
}
