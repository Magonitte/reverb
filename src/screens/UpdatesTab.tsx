import { useTranslation } from "react-i18next";
import type { Tool } from "@/bindings/Tool";
import type { YtdlpChannel } from "@/bindings/YtdlpChannel";
import { Button } from "@/components/ui/Button";
import { Card } from "@/components/ui/Card";
import { ProgressBar } from "@/components/ui/ProgressBar";
import { Skeleton } from "@/components/ui/Skeleton";
import { Toggle } from "@/components/ui/Toggle";
import { useAppInfoStore } from "@/stores/appInfo";
import { useSettingsStore } from "@/stores/settings";
import { useToolsStore } from "@/stores/tools";
import { useUiStore } from "@/stores/ui";
import { useUpdaterStore, type UpdaterPhase } from "@/stores/updater";
import { api } from "@/lib/ipc/api";

const TOOLS: Tool[] = ["ytdlp", "deno", "ffmpeg"];
const CHANNELS = ["stable", "nightly"] as const satisfies readonly YtdlpChannel[];

const PHASE_KEY: Record<UpdaterPhase, string> = {
  idle: "updater.idle",
  checking: "updater.checking",
  uptodate: "updater.upToDate",
  available: "updater.available",
  downloading: "updater.downloading",
  ready: "updater.ready",
  restarting: "updater.restarting",
  error: "updater.error",
};

export function UpdatesTab() {
  const { t } = useTranslation();
  const settings = useSettingsStore((s) => s.settings);
  const updateSettings = useSettingsStore((s) => s.update);
  const statuses = useToolsStore((s) => s.statuses);
  const appVersion = useAppInfoStore((s) => s.info?.version);
  const phase = useUpdaterStore((s) => s.phase);
  const version = useUpdaterStore((s) => s.version);
  const currentVersion = useUpdaterStore((s) => s.currentVersion);
  const notes = useUpdaterStore((s) => s.notes);
  const downloaded = useUpdaterStore((s) => s.downloaded);
  const total = useUpdaterStore((s) => s.total);
  const error = useUpdaterStore((s) => s.error);
  const check = useUpdaterStore((s) => s.check);
  const install = useUpdaterStore((s) => s.install);
  const toast = useUiStore((s) => s.pushToast);

  if (!settings) return <Skeleton className="h-64 w-full" />;

  const save = (patch: Parameters<typeof updateSettings>[0]) => {
    updateSettings(patch).catch(() => toast({ message: t("settings.saveFailed"), tone: "error" }));
  };

  const current = currentVersion ?? appVersion ?? "";
  const percent = total && total > 0 ? Math.round((downloaded / total) * 100) : null;
  const showOffer =
    phase === "available" || phase === "downloading" || phase === "ready" || phase === "restarting";

  return (
    <div className="flex flex-col gap-4">
      <Card aria-labelledby="updates-app-title">
        <div className="flex flex-wrap items-start justify-between gap-3">
          <div>
            <h2 id="updates-app-title" className="text-sm font-semibold text-fg">
              {t("settings.updates.appTitle")}
            </h2>
            <p data-testid="updater-phase" className="mt-1 text-[13px] text-fg-secondary">
              {t(PHASE_KEY[phase])}
            </p>
            {showOffer && version && (
              <p className="mt-1 text-[13px] text-fg">{t("settings.updates.toVersion", { current, version })}</p>
            )}
            {phase === "downloading" && percent !== null && (
              <p className="mt-1 text-xs text-fg-muted">
                {t("settings.updates.percent", { percent })}
              </p>
            )}
            {notes && showOffer && (
              <p className="mt-2 max-w-prose text-[13px] text-fg-muted">{notes}</p>
            )}
            {(phase === "available" || phase === "ready") && (
              <p className="mt-2 text-[13px] text-fg-muted">{t("updater.windowsCloseNotice")}</p>
            )}
            {phase === "error" && error && (
              <p className="mt-2 text-[13px] text-error">{error}</p>
            )}
          </div>
          {phase === "available" && (
            <Button variant="primary" onClick={() => void install()}>
              {t("settings.updates.installNow")}
            </Button>
          )}
          {phase === "error" && (
            <Button onClick={() => void check()}>{t("settings.updates.retry")}</Button>
          )}
        </div>
        {phase === "downloading" && (
          <div className="mt-4">
            <ProgressBar
              value={percent === null ? undefined : percent / 100}
              label={t("updater.downloading")}
            />
          </div>
        )}
      </Card>

      <Card aria-labelledby="updates-tools-title">
        <h2 id="updates-tools-title" className="mb-3 text-sm font-semibold text-fg">
          {t("settings.updates.toolsTitle")}
        </h2>
        <ul className="flex flex-col gap-2">
          {TOOLS.map((tool) => {
            const status = statuses.find((item) => item.tool === tool);
            const label = !status?.installed
              ? t("tools.missing")
              : status.updateAvailable
                ? t("tools.updateAvailable")
                : t("tools.upToDate");
            return (
              <li key={tool} className="flex flex-wrap items-center gap-3 text-[13px]">
                <span className="w-20 font-medium text-fg">{t(`tools.${tool}`)}</span>
                <span className="w-36 text-fg-muted">{status?.version ?? "—"}</span>
                <span className="text-fg-secondary">{label}</span>
                {status?.previousVersion && (
                  <Button
                    size="sm"
                    className="ml-auto"
                    onClick={() => {
                      api
                        .toolsRollback(tool)
                        .then(() => useToolsStore.getState().load())
                        .catch(() => toast({ message: t("settings.updates.rollbackFailed"), tone: "error" }));
                    }}
                  >
                    {t("settings.updates.rollback")}
                  </Button>
                )}
              </li>
            );
          })}
        </ul>
        <div className="mt-4 flex flex-wrap items-end justify-between gap-4">
          <Button loading={phase === "checking"} onClick={() => void check()}>
            {t("settings.updates.check")}
          </Button>
          <fieldset className="flex flex-col gap-1">
            <legend className="mb-1 text-xs text-fg-muted">{t("settings.updates.channel")}</legend>
            <div className="flex gap-3">
              {CHANNELS.map((value) => (
                <label key={value} className="flex items-center gap-2 text-[13px] text-fg">
                  <input
                    type="radio"
                    name="ytdlp-channel"
                    value={value}
                    checked={settings.ytdlpChannel === value}
                    onChange={() => save({ ytdlpChannel: value })}
                  />
                  {t(value === "stable" ? "settings.updates.channelStable" : "settings.updates.channelNightly")}
                </label>
              ))}
            </div>
          </fieldset>
        </div>
        <div className="mt-4 flex flex-col gap-3">
          <label className="flex items-center justify-between gap-3 text-[13px] text-fg">
            <span>{t("settings.updates.autoApp")}</span>
            <Toggle
              checked={settings.autoCheckAppUpdates}
              onChange={(checked) => save({ autoCheckAppUpdates: checked })}
              label={t("settings.updates.autoApp")}
            />
          </label>
          <label className="flex items-center justify-between gap-3 text-[13px] text-fg">
            <span>{t("settings.updates.autoTools")}</span>
            <Toggle
              checked={settings.autoUpdateTools}
              onChange={(checked) => save({ autoUpdateTools: checked })}
              label={t("settings.updates.autoTools")}
            />
          </label>
        </div>
      </Card>
    </div>
  );
}
