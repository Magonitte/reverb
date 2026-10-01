import { Settings as SettingsIcon } from "lucide-react";
import { useNavigate, useParams } from "react-router";
import { useTranslation } from "react-i18next";
import type { Language } from "@/bindings/Language";
import type { Theme } from "@/bindings/Theme";
import type { Transparency } from "@/bindings/Transparency";
import { ScreenHeader } from "@/components/ScreenHeader";
import { Card } from "@/components/ui/Card";
import { EmptyState } from "@/components/ui/EmptyState";
import { Select } from "@/components/ui/Select";
import { Skeleton } from "@/components/ui/Skeleton";
import { Tabs } from "@/components/ui/Tabs";
import { UpdatesTab } from "./UpdatesTab";
import { useSettingsStore } from "@/stores/settings";
import { useUiStore } from "@/stores/ui";

export const SETTINGS_TABS = [
  "general",
  "downloads",
  "metadata",
  "integration",
  "updates",
  "advanced",
] as const;
type SettingsTab = (typeof SETTINGS_TABS)[number];

/** Nomes de idioma ficam no próprio idioma (não se traduzem). */
const LANGUAGES: Array<{ value: Language; label: string }> = [
  { value: "pt-BR", label: "Português (Brasil)" },
  { value: "en", label: "English" },
];

function Appearance() {
  const { t } = useTranslation();
  const settings = useSettingsStore((s) => s.settings);
  const update = useSettingsStore((s) => s.update);
  const toast = useUiStore((s) => s.pushToast);

  if (!settings) return <Skeleton className="h-40 w-full" />;

  const save = (patch: Parameters<typeof update>[0]) => {
    update(patch).catch(() => toast({ message: t("settings.saveFailed"), tone: "error" }));
  };

  return (
    <Card aria-labelledby="appearance-title">
      <h2 id="appearance-title" className="mb-4 text-sm font-semibold text-fg">
        {t("settings.appearance.title")}
      </h2>
      <div className="grid gap-4 sm:grid-cols-3">
        <Select
          label={t("settings.appearance.theme")}
          value={settings.theme}
          onChange={(e) => save({ theme: e.target.value as Theme })}
          options={[
            { value: "dark", label: t("settings.appearance.themes.dark") },
            { value: "light", label: t("settings.appearance.themes.light") },
            { value: "system", label: t("settings.appearance.themes.system") },
          ]}
        />
        <Select
          label={t("settings.appearance.language")}
          value={settings.language}
          onChange={(e) => save({ language: e.target.value as Language })}
          options={LANGUAGES}
        />
        <Select
          label={t("settings.appearance.transparency")}
          value={settings.transparency}
          onChange={(e) => save({ transparency: e.target.value as Transparency })}
          options={[
            { value: "auto", label: t("settings.appearance.transparencies.auto") },
            { value: "full", label: t("settings.appearance.transparencies.full") },
            { value: "reduced", label: t("settings.appearance.transparencies.reduced") },
          ]}
        />
      </div>
    </Card>
  );
}

/** `/settings/:tab?` — as abas existem; o conteúdo de cada uma chega nas fases seguintes. */
export default function Settings() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const params = useParams();
  const tab: SettingsTab = (SETTINGS_TABS as readonly string[]).includes(params.tab ?? "")
    ? (params.tab as SettingsTab)
    : "general";

  return (
    <section aria-labelledby="screen-title">
      <ScreenHeader title={t("settings.title")} subtitle={t("settings.subtitle")} />
      <Tabs
        label={t("settings.tabsLabel")}
        value={tab}
        onChange={(id) => navigate(`/settings/${id}`)}
        tabs={SETTINGS_TABS.map((id) => ({ id, label: t(`settings.tabs.${id}`) }))}
      >
        {tab === "general" && <Appearance />}
        {tab === "updates" && <UpdatesTab />}
        {tab !== "general" && tab !== "updates" && (
          <EmptyState
            icon={<SettingsIcon aria-hidden="true" />}
            title={t(`settings.empty.${tab}.title`)}
            description={t(`settings.empty.${tab}.description`)}
          />
        )}
      </Tabs>
    </section>
  );
}
