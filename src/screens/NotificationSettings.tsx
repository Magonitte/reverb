import { useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Bell, Play, Upload } from "lucide-react";
import { Button, Card, Select } from "@/components/ui";
import {
  useNotificationPreferences,
  type NotificationSound,
} from "@/stores/notificationPreferences";
import { useUiStore } from "@/stores/ui";
import { playNotificationSound, readNotificationAudio } from "@/lib/notificationSound";
export function NotificationSettings() {
  const { t } = useTranslation();
  const { preferences: p, update } = useNotificationPreferences();
  const toast = useUiStore((s) => s.pushToast);
  const file = useRef<HTMLInputElement>(null);
  const [busy, setBusy] = useState(false);
  function save(patch: Parameters<typeof update>[0]) {
    try {
      update(patch);
    } catch {
      toast({ message: t("notifications.saveFailed"), tone: "error" });
    }
  }
  return (
    <Card className="space-y-4" aria-labelledby="notification-settings-title">
      <h2
        id="notification-settings-title"
        className="flex items-center gap-2 text-sm font-semibold"
      >
        <Bell className="size-4" />
        {t("notifications.settingsTitle")}
      </h2>
      <p className="text-sm text-fg-muted">{t("notifications.settingsHint")}</p>
      <Select
        label={t("notifications.sound")}
        value={p.sound}
        onChange={(e) => save({ sound: e.target.value as NotificationSound })}
        options={["glass", "marimba", "sparkle", ...(p.customData ? ["custom"] : []), "silent"].map(
          (value) => ({
            value,
            label:
              value === "custom"
                ? `${t("notifications.sounds.custom")} · ${p.customName}`
                : t(`notifications.sounds.${value}`),
          }),
        )}
      />
      <label className="flex flex-col gap-2 text-sm">
        {t("notifications.volume")} · {Math.round(p.volume * 100)}%
        <input
          type="range"
          className="accent-accent"
          min="0"
          max="100"
          value={p.volume * 100}
          onChange={(e) => save({ volume: Number(e.target.value) / 100 })}
        />
      </label>
      <div className="flex flex-wrap gap-3">
        <Button
          disabled={p.sound === "silent" || busy}
          icon={<Play className="size-4" />}
          onClick={() =>
            void playNotificationSound(true).catch(() =>
              toast({ message: t("notifications.playFailed"), tone: "error" }),
            )
          }
        >
          {t("notifications.previewSound")}
        </Button>
        <Button
          loading={busy}
          icon={<Upload className="size-4" />}
          onClick={() => file.current?.click()}
        >
          {t("notifications.addSound")}
        </Button>
        {p.customData && (
          <Button
            variant="ghost"
            disabled={busy}
            onClick={() =>
              save({
                customData: "",
                customName: "",
                sound: p.sound === "custom" ? "glass" : p.sound,
              })
            }
          >
            {t("notifications.removeSound")}
          </Button>
        )}
      </div>
      <input
        ref={file}
        type="file"
        accept=".wav,.mp3,.ogg,.m4a,audio/*"
        className="hidden"
        aria-label={t("notifications.addSound")}
        onChange={(e) => {
          const selected = e.target.files?.[0];
          e.target.value = "";
          if (!selected) return;
          setBusy(true);
          void readNotificationAudio(selected)
            .then((customData) => {
              update({ customData, customName: selected.name, sound: "custom" });
              toast({
                message: t("notifications.soundAdded", { name: selected.name }),
                tone: "success",
              });
            })
            .catch(() => toast({ message: t("notifications.invalidSound"), tone: "error" }))
            .finally(() => setBusy(false));
        }}
      />
      <p className="text-xs text-fg-muted">{t("notifications.fileHint")}</p>
      <Select
        label={t("notifications.clipboardDuration")}
        value={String(p.clipboardSeconds)}
        onChange={(e) => save({ clipboardSeconds: Number(e.target.value) })}
        options={[20, 35, 60].map((n) => ({
          value: String(n),
          label: t("notifications.seconds", { count: n }),
        }))}
      />
      <Button
        onClick={() =>
          toast({
            title: t("notifications.demoTitle"),
            message: t("notifications.demoMessage"),
            details: t("notifications.demoDetails"),
            tone: "success",
            sound: true,
          })
        }
      >
        {t("notifications.testNotification")}
      </Button>
    </Card>
  );
}
