import { useEffect, useState } from "react";
import { useNavigate } from "react-router";
import { useTranslation } from "react-i18next";
import type { JsRuntime } from "@/bindings/JsRuntime";
import type { ToolStatus } from "@/bindings/ToolStatus";
import type { ToolProgress } from "@/bindings/ToolProgress";
import { VinylDisc } from "@/components/ui/VinylDisc";
import { Button } from "@/components/ui/Button";
import { Card } from "@/components/ui/Card";
import { Input } from "@/components/ui/Input";
import { Select } from "@/components/ui/Select";
import { Toggle } from "@/components/ui/Toggle";
import { ProgressBar } from "@/components/ui/ProgressBar";
import { api } from "@/lib/ipc/api";
import { onEvent } from "@/lib/ipc/events";
import { useSettingsStore } from "@/stores/settings";

const STEPS = ["welcome", "folder", "tools", "metadata", "integration", "ready"] as const;
const TOOLS = ["ytdlp", "ffmpeg", "deno"] as const;
const INTEGRATION_OPTIONS = ["clipboardWatch", "launchAtStartup"] as const;
const folderValid = (path: string) => /^([a-zA-Z]:[\\/]|\/)/.test(path.trim());
export default function Onboarding() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const settings = useSettingsStore((state) => state.settings);
  const update = useSettingsStore((state) => state.update);
  const [step, setStep] = useState(0);
  const [folder, setFolder] = useState<string | null>(null);
  const [statuses, setStatuses] = useState<ToolStatus[]>([]);
  const [runtimes, setRuntimes] = useState<JsRuntime[]>([]);
  const [progress, setProgress] = useState<Record<string, number>>({});
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const destination = folder ?? settings?.outputDir ?? "";
  const runtimeReady = (choices: JsRuntime[]) =>
    settings?.jsRuntime === "auto"
      ? choices.length > 0
      : choices.includes(settings?.jsRuntime ?? "auto");
  const ready =
    ["ytdlp", "ffmpeg"].every((tool) =>
      statuses.some((status) => status.tool === tool && status.installed),
    ) && runtimeReady(runtimes);
  useEffect(() => {
    let active = true;
    let off: (() => void) | undefined;
    void onEvent<ToolProgress>("tools://progress", (value) => {
      if (active) setProgress((current) => ({ ...current, [value.tool]: value.percent }));
    }).then((unlisten) => {
      if (active) off = unlisten;
      else unlisten();
    });
    Promise.all([api.toolsStatus(), api.runtimeChoices()]).then(
      ([tools, choices]) => {
        if (active) {
          setStatuses(tools);
          setRuntimes(choices);
        }
      },
      () => {
        if (active) setError(t("onboarding.installFailed"));
      },
    );
    return () => {
      active = false;
      off?.();
    };
  }, [t]);
  const action = async (operation: () => Promise<void>) => {
    setBusy(true);
    setError("");
    try {
      await operation();
    } catch {
      setError(t(step === 2 ? "onboarding.installFailed" : "settings.saveFailed"));
    } finally {
      setBusy(false);
    }
  };
  const install = () =>
    action(async () => {
      await api.toolsInstallMissing();
      const [tools, choices] = await Promise.all([api.toolsStatus(), api.runtimeChoices()]);
      setStatuses(tools);
      setRuntimes(choices);
    });
  const next = () =>
    action(async () => {
      if (step === 1) {
        if (!folderValid(destination)) return;
        await update({ outputDir: destination.trim() });
      }
      if (step === 2 && !ready) return;
      if (step === 5) {
        const [tools, choices] = await Promise.all([api.toolsStatus(), api.runtimeChoices()]);
        if (
          !["ytdlp", "ffmpeg"].every((tool) =>
            tools.some((status) => status.tool === tool && status.installed),
          ) ||
          !runtimeReady(choices)
        ) {
          setStep(2);
          setError(t("onboarding.toolsRequired"));
          return;
        }
        await update({ onboardingCompleted: true });
        navigate("/", { replace: true });
        return;
      }
      setStep((value) => value + 1);
    });
  return (
    <section
      aria-labelledby="screen-title"
      className="mx-auto flex min-h-full max-w-[680px] flex-col justify-center gap-5 p-8"
    >
      <div className="flex items-center gap-4">
        <VinylDisc size={72} />
        <div>
          <h1 id="screen-title" className="font-display text-[26px] font-semibold">
            {t("onboarding.title")}
          </h1>
          <p className="text-sm text-fg-muted">{t("onboarding.subtitle")}</p>
        </div>
      </div>
      <ol
        className="flex flex-wrap gap-3 text-xs text-fg-muted"
        aria-label={t("onboarding.stepsLabel")}
      >
        {STEPS.map((name, index) => (
          <li
            key={name}
            aria-current={index === step ? "step" : undefined}
            className={index === step ? "text-accent" : ""}
          >
            {t(`onboarding.steps.${name}`)}
          </li>
        ))}
      </ol>
      <Card className="space-y-4">
        <h2 className="font-semibold">{t(`onboarding.steps.${STEPS[step]}`)}</h2>
        {step === 0 && <p className="text-sm text-fg-muted">{t("onboarding.welcomeHelp")}</p>}
        {step === 1 && (
          <>
            <Input
              label={t("onboarding.folder")}
              value={destination}
              onChange={(event) => setFolder(event.target.value)}
              error={
                destination && !folderValid(destination) ? t("onboarding.folderInvalid") : undefined
              }
            />
            <Button
              variant="secondary"
              onClick={() =>
                void action(async () => {
                  const picked = await api.pickFolder();
                  if (picked) setFolder(picked);
                })
              }
            >
              {t("onboarding.chooseFolder")}
            </Button>
          </>
        )}
        {step === 2 && (
          <>
            <p className="text-sm text-fg-muted">{t("onboarding.toolsHelp")}</p>
            <Select
              label={t("onboarding.runtime")}
              value={settings?.jsRuntime ?? "auto"}
              onChange={(event) =>
                void action(async () => {
                  await update({ jsRuntime: event.target.value as JsRuntime });
                  setRuntimes(await api.runtimeChoices());
                })
              }
              options={[
                "auto",
                "managed-deno",
                ...runtimes.filter((value) => value.startsWith("system-")),
              ].map((value) => ({ value, label: t(`onboarding.runtimes.${value}`) }))}
            />
            {TOOLS.map((tool) => (
              <div key={tool} className="space-y-1">
                <p className="text-xs text-fg-muted">
                  {t(`tools.${tool}`)} ·{" "}
                  {t(
                    statuses.some((status) => status.tool === tool && status.installed)
                      ? "onboarding.installed"
                      : "onboarding.missing",
                  )}
                </p>
                {progress[tool] !== undefined && (
                  <ProgressBar value={progress[tool] / 100} label={t(`tools.${tool}`)} />
                )}
              </div>
            ))}
            <Button loading={busy} onClick={() => void install()}>
              {t(error ? "common.retry" : "onboarding.install")}
            </Button>
            {ready && (
              <p role="status" className="text-sm text-success">
                {t("onboarding.toolsReady")}
              </p>
            )}
          </>
        )}
        {step === 3 && <p className="text-sm text-fg-muted">{t("onboarding.metadataHelp")}</p>}
        {step === 4 && settings && (
          <>
            {INTEGRATION_OPTIONS.map((key) => (
              <div key={key} className="flex items-center justify-between gap-4">
                <span className="text-sm">{t(`onboarding.${key}`)}</span>
                <Toggle
                  label={t(`onboarding.${key}`)}
                  checked={settings[key]}
                  disabled={busy}
                  onChange={(value) => void action(() => update({ [key]: value }))}
                />
              </div>
            ))}
          </>
        )}
        {step === 5 && <p className="text-sm text-fg-muted">{t("onboarding.readyHelp")}</p>}
        {error && (
          <p role="alert" className="text-sm text-error">
            {error}
          </p>
        )}
      </Card>
      <div className="flex justify-between gap-3">
        <Button
          variant="secondary"
          disabled={step === 0 || busy}
          onClick={() => {
            setError("");
            setStep((value) => value - 1);
          }}
        >
          {t("onboarding.back")}
        </Button>
        <Button
          loading={busy}
          disabled={busy || (step === 1 && !folderValid(destination)) || (step === 2 && !ready)}
          onClick={() => void next()}
        >
          {t(step === 5 ? "onboarding.finish" : "onboarding.next")}
        </Button>
      </div>
    </section>
  );
}
