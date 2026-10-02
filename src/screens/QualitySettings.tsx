import { useState } from "react";
import { useTranslation } from "react-i18next";
import type { CookiesSource } from "@/bindings/CookiesSource";
import type { CookiesTestResult } from "@/bindings/CookiesTestResult";
import { Card, Input, Button, Select } from "@/components/ui";
import { api } from "@/lib/ipc/api";
import { useSettingsStore } from "@/stores/settings";
import { useUiStore } from "@/stores/ui";
import { errorText } from "@/lib/errors";

export function QualitySettings() {
  const { t } = useTranslation();
  const settings = useSettingsStore((s) => s.settings);
  const update = useSettingsStore((s) => s.update);
  const toast = useUiStore((s) => s.pushToast);
  const [busy, setBusy] = useState(false);
  const [result, setResult] = useState<CookiesTestResult | null>(null);
  const [file, setFile] = useState("");
  if (!settings) return null;
  async function test() {
    setBusy(true);
    setResult(null);
    try {
      setResult(await api.cookiesTest());
    } catch (e) {
      toast({ message: errorText(t, e), tone: "error" });
    } finally {
      setBusy(false);
    }
  }
  return (
    <Card className="space-y-4">
      <h2 className="text-sm font-semibold">{t("quality.cookies")}</h2>
      <Select
        label={t("quality.source")}
        value={settings.cookiesSource}
        options={["none", "firefox", "chrome", "edge", "brave", "file"].map((value) => ({
          value,
          label: t(`quality.sources.${value}`),
        }))}
        onChange={(e) => {
          setResult(null);
          const source = e.target.value as CookiesSource;
          if (source === "file" && !settings.cookiesFile) return;
          void update({ cookiesSource: source }).catch((err) =>
            toast({ message: errorText(t, err), tone: "error" }),
          );
        }}
      />
      <Input
        label={t("quality.file")}
        value={file || settings.cookiesFile}
        onChange={(e) => setFile(e.target.value)}
      />
      <Button
        disabled={!file.trim() || busy}
        onClick={() =>
          void update({ cookiesSource: "file", cookiesFile: file.trim() })
            .then(() => setResult(null))
            .catch((e) => toast({ message: errorText(t, e), tone: "error" }))
        }
      >
        {t("quality.useFile")}
      </Button>
      <p className="text-xs text-fg-muted">{t("quality.hint")}</p>
      <Button disabled={busy} onClick={() => void test()}>
        {t(busy ? "quality.testing" : "quality.test")}
      </Button>
      {result && (
        <p role="status" aria-label={t("quality.test")} className="text-sm">
          {result.ok
            ? `${t(result.premium ? "quality.premium" : "quality.standard")} · ${result.bestAudio ?? ""}`
            : errorText(t, { kind: result.errorKind })}
        </p>
      )}
    </Card>
  );
}
