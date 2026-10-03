import { Select } from "@/components/ui/Select";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import type { SettingsPatch } from "@/bindings/SettingsPatch";
import { Card } from "@/components/ui/Card";
import { Input } from "@/components/ui/Input";
import { Toggle } from "@/components/ui/Toggle";
import { Button } from "@/components/ui/Button";
import { api } from "@/lib/ipc/api";
import { useSettingsStore } from "@/stores/settings";
import { useUiStore } from "@/stores/ui";

type Flag =
  | "verifyLosslessOnImport"
  | "trimSilence"
  | "watchLibrary"
  | "autoOrganize"
  | "fetchMetadata"
  | "preferOfficialAudio"
  | "offlineMode"
  | "fetchArtwork"
  | "writeFolderCover"
  | "fetchLyrics"
  | "writeLrcFile"
  | "normalizeVolume";

export function PostprocessSettings({ tab }: { tab: "downloads" | "metadata" }) {
  const { t } = useTranslation();
  const settings = useSettingsStore((s) => s.settings);
  const update = useSettingsStore((s) => s.update);
  const toast = useUiStore((s) => s.pushToast);
  const [draft, setDraft] = useState<string | null>(null);
  const [result, setResult] = useState<{
    template: string;
    preview: string;
    error: boolean;
  } | null>(null);
  const template = draft ?? settings?.fileTemplate ?? "";
  const hasSettings = settings !== null;
  useEffect(() => {
    if (tab !== "downloads" || !hasSettings) return;
    let active = true;
    const timer = setTimeout(() => {
      api.templatePreview(template).then(
        (preview) => {
          if (active) setResult({ template, preview, error: false });
        },
        () => {
          if (active) setResult({ template, preview: "", error: true });
        },
      );
    }, 180);
    return () => {
      active = false;
      clearTimeout(timer);
    };
  }, [tab, template, settings?.autoOrganize, settings?.language, settings?.outputDir, hasSettings]);
  if (!settings) return null;
  const save = (patch: SettingsPatch) =>
    update(patch).catch(() => toast({ message: t("settings.saveFailed"), tone: "error" }));
  const flags: Flag[] =
    tab === "downloads"
      ? ["autoOrganize", "watchLibrary", "trimSilence", "verifyLosslessOnImport"]
      : [
          "fetchMetadata",
          "preferOfficialAudio",
          "offlineMode",
          "fetchArtwork",
          "writeFolderCover",
          "fetchLyrics",
          "writeLrcFile",
          "normalizeVolume",
        ];
  const valid = result?.template === template && !result.error;
  return (
    <Card className="space-y-4">
      {flags.map((key) => (
        <div key={key} className="flex items-center justify-between gap-4">
          <div>
            <p className="text-sm text-fg">{t(`settings.postprocess.${key}`)}</p>
            {key === "normalizeVolume" && (
              <p className="mt-1 text-xs text-fg-muted">
                {t("settings.postprocess.replayGainHint")}
              </p>
            )}
          </div>
          <Toggle
            checked={settings[key]}
            label={t(`settings.postprocess.${key}`)}
            onChange={(checked) => void save({ [key]: checked })}
          />
        </div>
      ))}
      {tab === "downloads" && (
        <>
          <Select
            label={t("quality.splitMode")}
            value={settings.splitChapters}
            options={["ask", "always", "never"].map((value) => ({
              value,
              label: t(`quality.splitModes.${value}`),
            }))}
            onChange={(e) =>
              void save({ splitChapters: e.target.value as "ask" | "always" | "never" })
            }
          />
          <Input
            label={t("sync.pacing")}
            type="number"
            min={0}
            max={60}
            step={1}
            value={settings.playlistPacingSeconds}
            onChange={(event) => {
              const value = Number(event.target.value);
              if (Number.isInteger(value) && value >= 0 && value <= 60)
                void save({ playlistPacingSeconds: value });
            }}
          />
          <Input
            label={t("settings.postprocess.template")}
            value={template}
            onChange={(e) => setDraft(e.target.value)}
            hint={t("settings.postprocess.variables")}
            error={
              result?.template === template && result.error
                ? t("settings.postprocess.invalidTemplate")
                : undefined
            }
          />
          <p className="text-xs text-fg-muted">{t("settings.postprocess.preview")}</p>
          <output
            data-testid="template-preview"
            aria-live="polite"
            className="block break-all rounded-md bg-field p-3 font-mono text-xs text-fg-secondary"
          >
            {valid ? result.preview : t("settings.postprocess.previewPending")}
          </output>
          <Button
            disabled={!valid || template === settings.fileTemplate}
            onClick={() => void save({ fileTemplate: template })}
          >
            {t("common.save")}
          </Button>
        </>
      )}
    </Card>
  );
}
