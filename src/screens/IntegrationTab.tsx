import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Card } from "@/components/ui/Card";
import { Button } from "@/components/ui/Button";
import { Input } from "@/components/ui/Input";
import { api } from "@/lib/ipc/api";
import { useUiStore } from "@/stores/ui";
import { Toggle } from "@/components/ui/Toggle";
import { ShortcutSettings } from "./ShortcutSettings";
import { useSettingsStore } from "@/stores/settings";

export function IntegrationTab() {
  const { t } = useTranslation();
  const [code, setCode] = useState("");
  const [testLink, setTestLink] = useState("reverb://open");
  const [testing, setTesting] = useState(false);
  const toast = useUiStore((state) => state.pushToast);
  const settings = useSettingsStore((state) => state.settings);
  const update = useSettingsStore((state) => state.update);
  useEffect(() => {
    let active = true;
    api.bookmarkletCode().then(
      (value) => {
        if (active) setCode(value);
      },
      () => {
        if (active) toast({ message: t("settings.saveFailed"), tone: "error" });
      },
    );
    return () => {
      active = false;
    };
  }, [t, toast]);
  const copy = async () => {
    try {
      await api.bookmarkletCopy();
      toast({ message: t("integration.bookmarkletCopied"), tone: "success" });
    } catch {
      toast({ message: t("settings.saveFailed"), tone: "error" });
    }
  };
  return (
    <Card className="space-y-4">
      {settings && (
        <Input
          type="number"
          min={1}
          max={168}
          label={t("artists.interval")}
          value={settings.artistCheckIntervalHours}
          onChange={(e) =>
            void update({ artistCheckIntervalHours: Number(e.target.value) }).catch(() =>
              toast({ message: t("settings.saveFailed"), tone: "error" }),
            )
          }
        />
      )}
      {settings && (
        <ShortcutSettings
          value={settings.globalShortcut}
          onChange={(globalShortcut) => update({ globalShortcut })}
        />
      )}
      {settings && (
        <section className="rounded-lg border border-glass-border bg-field p-4">
          <div className="flex items-center justify-between gap-4">
            <div>
              <h2 className="text-sm font-semibold">{t("integration.easyTitle")}</h2>
              <p className="mt-1 text-sm text-fg-muted">{t("integration.easyHelp")}</p>
            </div>
            <Toggle
              label={t("integration.clipboardWatch")}
              checked={settings.clipboardWatch}
              onChange={(clipboardWatch) => {
                void update({ clipboardWatch }).catch(() =>
                  toast({ message: t("settings.saveFailed"), tone: "error" }),
                );
              }}
            />
          </div>
          <ol className="mt-4 list-decimal space-y-2 pl-5 text-sm text-fg-secondary">
            {[1, 2, 3].map((step) => (
              <li key={step}>{t(`integration.easyStep${step}`)}</li>
            ))}
          </ol>
          <p className="mt-3 text-xs text-fg-muted">{t("integration.easyPrivacy")}</p>
        </section>
      )}
      <details className="rounded-lg border border-glass-border p-4">
        <summary className="cursor-pointer text-sm font-semibold text-fg">
          {t("integration.advancedTitle")}
        </summary>
        <div className="mt-4 space-y-4">
          <h2 className="text-sm font-semibold">{t("integration.bookmarkletTitle")}</h2>
          <p className="text-sm text-fg-muted">{t("integration.bookmarkletHelp")}</p>
          <ol className="list-decimal space-y-2 pl-5 text-sm text-fg-secondary">
            {[1, 2, 3, 4].map((step) => (
              <li key={step}>{t(`integration.bookmarkletStep${step}`)}</li>
            ))}
          </ol>
          <Input label={t("integration.bookmarkletCode")} value={code} readOnly />
          <div className="flex flex-wrap items-center gap-4">
            <a
              href="reverb://open"
              draggable
              className="inline-block text-accent underline"
              onDragStart={(event) => {
                event.dataTransfer.setData("text/uri-list", code);
                event.dataTransfer.setData("text/plain", code);
              }}
            >
              {t("app.name")}
            </a>
            <Button disabled={!code} onClick={() => void copy()}>
              {t("integration.copyBookmarklet")}
            </Button>
          </div>
          <Input
            label={t("integration.testLink")}
            value={testLink}
            onChange={(event) => setTestLink(event.target.value)}
          />
          <Button
            variant="secondary"
            loading={testing}
            disabled={!testLink.trim()}
            onClick={() => {
              setTesting(true);
              void api
                .deeplinkTest(testLink.trim())
                .then(
                  () => toast({ message: t("integration.testSuccess"), tone: "success" }),
                  () => toast({ message: t("integration.testFailed"), tone: "error" }),
                )
                .finally(() => setTesting(false));
            }}
          >
            {t("integration.test")}
          </Button>
        </div>
      </details>
    </Card>
  );
}
