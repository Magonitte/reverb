import { useEffect, useRef } from "react";
import { ToastHost } from "@/components/ui/Toast";
import { useAppearance } from "@/hooks/useAppearance";
import { api } from "@/lib/ipc/api";
import { onEvent } from "@/lib/ipc/events";
import { showNativeNotice } from "@/stores/nativeNotices";
import { useUiStore } from "@/stores/ui";
import { useSettingsStore } from "@/stores/settings";
import { useTranslation } from "react-i18next";
export function NotificationWindow() {
  const { t } = useTranslation();
  useAppearance();
  const container = useRef<HTMLDivElement>(null);
  const toasts = useUiStore((s) => s.toasts);
  const ready = useSettingsStore((s) => !!s.settings);
  useEffect(() => {
    document.documentElement.dataset.notificationWindow = "true";
    if (!ready) return;
    let active = true;
    let off = () => {};
    const drain = () =>
      void api.notificationsPending().then((items) => {
        if (active) items.forEach(showNativeNotice);
      });
    void onEvent("notification://pending", drain).then((unlisten) => {
      if (!active) unlisten();
      else {
        off = unlisten;
        drain();
      }
    });
    return () => {
      active = false;
      off();
    };
  }, [ready]);
  useEffect(() => {
    if (!toasts.length) {
      void api.notificationsHide();
      return;
    }
    const root = container.current;
    if (!root) return;
    const resize = () => {
      const host = root.firstElementChild;
      if (host) void api.notificationsResize(Math.ceil(host.scrollHeight) + 32);
    };
    const observer = new ResizeObserver(resize);
    if (root.firstElementChild) observer.observe(root.firstElementChild);
    resize();
    return () => observer.disconnect();
  }, [toasts.length]);
  return (
    <main className="p-4">
      <h1 className="sr-only">{t("notifications.regionTitle")}</h1>
      <div ref={container}>
        <ToastHost floatingWindow />
      </div>
    </main>
  );
}
