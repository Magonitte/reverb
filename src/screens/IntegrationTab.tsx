import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Card } from "@/components/ui/Card";
import { Button } from "@/components/ui/Button";
import { Input } from "@/components/ui/Input";
import { api } from "@/lib/ipc/api";
import { useUiStore } from "@/stores/ui";
import { Toggle } from "@/components/ui/Toggle";
import { useSettingsStore } from "@/stores/settings";

export function IntegrationTab() {
  const { t } = useTranslation();
  const [code, setCode] = useState("");
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
        <div className="flex items-center justify-between gap-4">
          <span className="text-sm">{t("integration.clipboardWatch")}</span>
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
      )}
      <h2 className="text-sm font-semibold">{t("integration.bookmarkletTitle")}</h2>
      <p className="text-sm text-fg-muted">{t("integration.bookmarkletHelp")}</p>
      <Input label={t("integration.bookmarkletCode")} value={code} readOnly />
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
    </Card>
  );
}
