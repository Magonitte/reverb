import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/Button";
import { ShortcutRecorder } from "@/components/ui/ShortcutRecorder";
export function ShortcutSettings({
  value,
  onChange,
}: {
  value: string;
  onChange: (value: string) => Promise<void>;
}) {
  const { t } = useTranslation();
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const save = async (next: string) => {
    setBusy(true);
    setError("");
    try {
      await onChange(next);
    } catch (failure) {
      setError(t((failure as { i18nKey?: string }).i18nKey ?? "integration.shortcutConflict"));
    } finally {
      setBusy(false);
    }
  };
  return (
    <div className="space-y-2">
      <p className="text-sm">{t("integration.shortcut")}</p>
      <fieldset disabled={busy} className="flex gap-2">
        <ShortcutRecorder
          value={value}
          onChange={(next) => void save(next)}
          label={t("integration.shortcut")}
          emptyLabel={t("integration.shortcutEmpty")}
          recordingLabel={t("integration.shortcutRecording")}
        />
        <Button variant="secondary" disabled={!value} onClick={() => void save("")}>
          {t("integration.shortcutDisable")}
        </Button>
      </fieldset>
      <p className="text-xs text-fg-muted">{t("integration.shortcutHelp")}</p>
      <p className="text-xs text-fg-muted">{t("integration.wayland")}</p>
      {error && (
        <p role="alert" className="text-xs text-error">
          {error}
        </p>
      )}
    </div>
  );
}
