import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import type { DiagnosticReport } from "@/bindings/DiagnosticReport";
import type { DataPaths } from "@/bindings/DataPaths";
import type { JsRuntime } from "@/bindings/JsRuntime";
import type { YtdlpChannel } from "@/bindings/YtdlpChannel";
import { Button } from "@/components/ui/Button";
import { Card } from "@/components/ui/Card";
import { Select } from "@/components/ui/Select";
import { Toggle } from "@/components/ui/Toggle";
import { ConfirmDialog } from "@/components/ui/ConfirmDialog";
import { api } from "@/lib/ipc/api";
import { useSettingsStore } from "@/stores/settings";
import { useUiStore } from "@/stores/ui";
const CHANNELS = ["stable", "nightly"] as const;
type Confirmation = { kind: "restore"; path: string } | { kind: "reset" };

export function AdvancedTab() {
  const { t, i18n } = useTranslation();
  const settings = useSettingsStore((state) => state.settings);
  const update = useSettingsStore((state) => state.update);
  const reset = useSettingsStore((state) => state.reset);
  const toast = useUiStore((state) => state.pushToast);
  const [report, setReport] = useState<DiagnosticReport | null>(null);
  const [paths, setPaths] = useState<DataPaths | null>(null);
  const [runtimes, setRuntimes] = useState<JsRuntime[]>([]);
  const [busy, setBusy] = useState(false);
  const [confirmation, setConfirmation] = useState<Confirmation | null>(null);
  useEffect(() => {
    let active = true;
    Promise.all([api.diagnosticsLast(), api.dataPaths(), api.runtimeChoices()]).then(
      ([last, directories, choices]) => {
        if (active) {
          setReport(last);
          setPaths(directories);
          setRuntimes(choices);
        }
      },
      () => {
        if (active) toast({ message: t("settings.advanced.failed"), tone: "error" });
      },
    );
    return () => {
      active = false;
    };
  }, [t, toast]);
  const action = async (operation: () => Promise<void>) => {
    setBusy(true);
    try {
      await operation();
    } catch {
      toast({ message: t("settings.advanced.failed"), tone: "error" });
    } finally {
      setBusy(false);
    }
  };
  const backup = () =>
    action(async () => {
      const path = await api.pickBackupPath();
      if (path) {
        await api.dataExport(path);
        toast({ message: t("settings.advanced.exported"), tone: "success" });
      }
    });
  const restore = () =>
    action(async () => {
      const path = await api.pickBackupPath(true);
      if (path) setConfirmation({ kind: "restore", path });
    });
  const confirm = () =>
    action(async () => {
      if (!confirmation) return;
      if (confirmation.kind === "restore") {
        await api.dataImport(confirmation.path);
        toast({ message: t("settings.advanced.restarting"), tone: "success" });
      } else await reset();
      setConfirmation(null);
    });
  const runtimeOptions: JsRuntime[] = [
    ...new Set<JsRuntime>(["auto", "managed-deno", ...runtimes, settings?.jsRuntime ?? "auto"]),
  ];
  return (
    <div className="space-y-4">
      {settings && (
        <Card className="space-y-4">
          <h2 className="text-sm font-semibold">{t("settings.advanced.tools")}</h2>
          <div className="grid gap-4 sm:grid-cols-2">
            <Select
              label={t("onboarding.runtime")}
              value={settings.jsRuntime}
              disabled={busy}
              options={runtimeOptions.map((value) => ({
                value,
                label: t(`onboarding.runtimes.${value}`),
              }))}
              onChange={(event) =>
                void action(() => update({ jsRuntime: event.target.value as JsRuntime }))
              }
            />
            <Select
              label={t("settings.updates.channel")}
              value={settings.ytdlpChannel}
              disabled={busy}
              options={CHANNELS.map((value) => ({
                value,
                label: t(
                  value === "stable"
                    ? "settings.updates.channelStable"
                    : "settings.updates.channelNightly",
                ),
              }))}
              onChange={(event) =>
                void action(() => update({ ytdlpChannel: event.target.value as YtdlpChannel }))
              }
            />
          </div>
          <div className="flex items-center justify-between gap-4 text-sm">
            <span>{t("settings.advanced.weekly")}</span>
            <Toggle
              label={t("settings.advanced.weekly")}
              checked={settings.weeklySelfTest}
              disabled={busy}
              onChange={(weeklySelfTest) => void action(() => update({ weeklySelfTest }))}
            />
          </div>
        </Card>
      )}
      <Card className="space-y-4">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <h2 className="text-sm font-semibold">{t("settings.advanced.diagnostics")}</h2>
          <Button
            loading={busy}
            onClick={() => void action(async () => setReport(await api.diagnosticsRun()))}
          >
            {t("settings.advanced.run")}
          </Button>
        </div>
        <p className="text-xs text-fg-muted">
          {report
            ? t("settings.advanced.last", {
                date: new Date(report.createdAt * 1000).toLocaleString(i18n.language),
              })
            : t("settings.advanced.never")}
        </p>
        {report && (
          <ul aria-label={t("settings.advanced.result")} className="space-y-3">
            {report.items.map((item) => (
              <li key={item.id} className="rounded-lg border border-glass-border p-3">
                <div className="flex flex-wrap justify-between gap-2 text-sm">
                  <span>{t(`settings.advanced.items.${item.id}`)}</span>
                  <span
                    className={
                      item.level === "error"
                        ? "text-error"
                        : item.level === "warning"
                          ? "text-warning"
                          : "text-success"
                    }
                  >
                    {t(`settings.advanced.levels.${item.level}`)}
                  </span>
                </div>
                <p className="mt-1 break-words whitespace-pre-wrap text-xs text-fg-muted">
                  {item.detail}
                </p>
              </li>
            ))}
          </ul>
        )}
      </Card>
      <Card className="space-y-4">
        <h2 className="text-sm font-semibold">{t("settings.advanced.data")}</h2>
        <p className="text-xs text-fg-muted">{t("settings.advanced.backupHelp")}</p>
        {paths && (
          <p className="break-all text-xs text-fg-muted">
            {t(paths.portable ? "settings.advanced.portable" : "settings.advanced.installed")} ·{" "}
            {paths.dataDir}
          </p>
        )}
        <div className="flex flex-wrap gap-3">
          <Button
            variant="secondary"
            disabled={busy}
            onClick={() =>
              void action(async () => {
                if (await api.logsExport())
                  toast({ message: t("settings.advanced.exported"), tone: "success" });
              })
            }
          >
            {t("settings.advanced.logs")}
          </Button>
          <Button variant="secondary" disabled={busy} onClick={() => void backup()}>
            {t("settings.advanced.backup")}
          </Button>
          <Button variant="secondary" disabled={busy} onClick={() => void restore()}>
            {t("settings.advanced.restore")}
          </Button>
          <Button
            variant="secondary"
            disabled={busy}
            onClick={() => void action(() => api.openDataDir())}
          >
            {t("settings.advanced.open")}
          </Button>
          <Button
            variant="danger"
            disabled={busy}
            onClick={() => setConfirmation({ kind: "reset" })}
          >
            {t("settings.advanced.reset")}
          </Button>
        </div>
      </Card>
      <ConfirmDialog
        open={confirmation !== null}
        title={t(
          confirmation?.kind === "restore"
            ? "settings.advanced.restore"
            : "settings.advanced.reset",
        )}
        message={t(
          confirmation?.kind === "restore"
            ? "settings.advanced.restoreConfirm"
            : "settings.advanced.resetConfirm",
        )}
        confirmLabel={t(busy ? "common.loading" : "common.confirm")}
        cancelLabel={t("common.cancel")}
        closeLabel={t("common.close")}
        danger
        onConfirm={() => {
          if (!busy) void confirm();
        }}
        onCancel={() => {
          if (!busy) setConfirmation(null);
        }}
      />
    </div>
  );
}
