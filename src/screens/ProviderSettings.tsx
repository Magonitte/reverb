import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Card, Input, Button } from "@/components/ui";
import { api } from "@/lib/ipc/api";
import { errorText } from "@/lib/errors";
import { useSettingsStore } from "@/stores/settings";
import { useUiStore } from "@/stores/ui";
const GUIDE_STEPS = ["step1", "step2", "step3", "step4"];

export function ProviderSettings() {
  const { t } = useTranslation();
  const settings = useSettingsStore((s) => s.settings);
  const update = useSettingsStore((s) => s.update);
  const toast = useUiStore((s) => s.pushToast);
  const [values, setValues] = useState<Record<string, string>>({});
  const [busy, setBusy] = useState<string | null>(null);
  const [status, setStatus] = useState<Record<string, string>>({});
  if (!settings) return null;
  const fields = [
    { provider: "acoustid", fields: ["acoustidKey"] },
    { provider: "spotify", fields: ["spotifyClientId", "spotifyClientSecret"] },
    { provider: "discogs", fields: ["discogsToken"] },
    { provider: "jamendo", fields: ["jamendoClientId"] },
  ] as const;
  const test = async (provider: string) => {
    setBusy(provider);
    setStatus((old) => ({ ...old, [provider]: "" }));
    try {
      await api.providerTest(provider);
      setStatus((old) => ({ ...old, [provider]: t("quality.apiOk") }));
    } catch (e) {
      setStatus((old) => ({ ...old, [provider]: errorText(t, e) }));
    } finally {
      setBusy(null);
    }
  };
  return (
    <Card className="space-y-5">
      <h2 className="text-sm font-semibold">{t("quality.apis")}</h2>
      {fields.map((group) => (
        <div
          key={group.provider}
          role="group"
          aria-label={t(`quality.providers.${group.provider}`)}
          className="space-y-2"
        >
          <h3 className="text-sm">
            {t(`quality.providers.${group.provider}`)} ·{" "}
            {t(
              settings.secretsStatus[group.provider]
                ? "quality.configured"
                : "quality.notConfigured",
            )}
          </h3>
          {group.provider === "spotify" && (
            <details className="text-sm text-fg-muted">
              <summary>{t("imports.spotifyGuide")}</summary>
              <ol className="mt-2 list-decimal space-y-2 pl-5">
                {GUIDE_STEPS.map((key) => (
                  <li key={key}>{t(`imports.${key}`)}</li>
                ))}
              </ol>
              <Button
                onClick={() =>
                  void api
                    .openSourceUrl("https://developer.spotify.com/dashboard")
                    .catch((e) => toast({ message: errorText(t, e), tone: "error" }))
                }
              >
                {t("imports.spotifyDashboard")}
              </Button>
            </details>
          )}
          {group.fields.map((field) => (
            <Input
              key={field}
              type="password"
              autoComplete="off"
              label={t(`quality.fields.${field}`)}
              value={values[field] ?? ""}
              onChange={(e) => setValues((old) => ({ ...old, [field]: e.target.value }))}
            />
          ))}
          <div className="flex gap-2">
            <Button
              disabled={busy !== null || !group.fields.some((f) => values[f] !== undefined)}
              onClick={() =>
                void update(
                  Object.fromEntries(
                    group.fields.filter((f) => values[f] !== undefined).map((f) => [f, values[f]]),
                  ),
                )
                  .then(() => {
                    setValues((old) => {
                      const next = { ...old };
                      for (const f of group.fields) delete next[f];
                      return next;
                    });
                    setStatus((old) => ({ ...old, [group.provider]: "" }));
                  })
                  .catch((e) => toast({ message: errorText(t, e), tone: "error" }))
              }
            >
              {t("common.save")}
            </Button>
            <Button
              disabled={busy !== null || !settings.secretsStatus[group.provider]}
              onClick={() => void test(group.provider)}
            >
              {t(busy === group.provider ? "quality.testing" : "quality.testApi")}
            </Button>
            <Button
              disabled={busy !== null || !settings.secretsStatus[group.provider]}
              onClick={() =>
                void update(Object.fromEntries(group.fields.map((f) => [f, ""]))).catch((e) =>
                  toast({ message: errorText(t, e), tone: "error" }),
                )
              }
            >
              {t("quality.clearKey")}
            </Button>
          </div>
          {status[group.provider] && (
            <p
              role="status"
              aria-label={t(`quality.providers.${group.provider}`)}
              className="text-xs text-fg-secondary"
            >
              {status[group.provider]}
            </p>
          )}
        </div>
      ))}
    </Card>
  );
}
