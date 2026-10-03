import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Card } from "@/components/ui/Card";
import { Toggle } from "@/components/ui/Toggle";
import { useSettingsStore } from "@/stores/settings";
import { useUiStore } from "@/stores/ui";
const OPTIONS = [
  "launchAtStartup",
  "startMinimized",
  "minimizeToTray",
  "closeToTray",
  "completionNotifications",
] as const;

export function DesktopSettings() {
  const { t } = useTranslation();
  const settings = useSettingsStore((state) => state.settings);
  const update = useSettingsStore((state) => state.update);
  const toast = useUiStore((state) => state.pushToast);
  const [busy, setBusy] = useState(false);
  if (!settings) return null;
  const save = async (key: (typeof OPTIONS)[number], value: boolean) => {
    setBusy(true);
    try {
      await update({ [key]: value });
    } catch {
      toast({ message: t("settings.saveFailed"), tone: "error" });
    } finally {
      setBusy(false);
    }
  };
  return (
    <Card className="space-y-4">
      <h2 className="text-sm font-semibold">{t("settings.desktop.title")}</h2>
      {OPTIONS.map((key) => (
        <div key={key} className="flex items-center justify-between gap-4 text-sm">
          <span>{t(`settings.desktop.${key}`)}</span>
          <Toggle
            label={t(`settings.desktop.${key}`)}
            checked={settings[key]}
            disabled={busy}
            onChange={(value) => void save(key, value)}
          />
        </div>
      ))}
    </Card>
  );
}
