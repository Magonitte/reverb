import { useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router";
import type { ImportAnalysis } from "@/bindings/ImportAnalysis";
import { ScreenHeader } from "./ScreenHeader";
import { Button, Checkbox, Select, Badge } from "./ui";
import { api } from "@/lib/ipc/api";
import { errorText } from "@/lib/errors";
import { PROFILE_OPTIONS } from "@/lib/profiles";
import { useSettingsStore } from "@/stores/settings";
import { useUiStore } from "@/stores/ui";
import { VirtualTable } from "./ui/VirtualTable";
export function ImportedCollectionView({ analysis }: { analysis: ImportAnalysis }) {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const settings = useSettingsStore((s) => s.settings);
  const toast = useUiStore((s) => s.pushToast);
  const [filter, setFilter] = useState("all");
  const [profile, setProfile] = useState(settings?.defaultProfile ?? "original");
  const [selected, setSelected] = useState(
    new Set(analysis.items.filter((i) => i.matched.bucket === "ok").map((i) => i.track.id)),
  );
  const [busy, setBusy] = useState(false);
  const items = analysis.items.filter((i) => filter === "all" || i.matched.bucket === filter);
  async function download(mode: "once" | "sync") {
    setBusy(true);
    try {
      const jobs = await api.importEnqueue({
        url: analysis.collection.url,
        trackIds: [...selected],
        mode,
        profileId: profile,
        syncOptions:
          mode === "sync"
            ? {
                url: analysis.collection.url,
                title: null,
                profileId: profile,
                outputDir: null,
                intervalHours: 24,
                maxItems: null,
                removeDeleted: false,
                writeM3u: true,
              }
            : null,
      });
      toast({
        message: t(mode === "sync" ? "imports.synced" : "collection.queued", {
          count: jobs.length,
        }),
        tone: "success",
      });
      navigate(mode === "sync" ? "/playlists" : "/activity");
    } catch (e) {
      toast({ message: errorText(t, e), tone: "error" });
    } finally {
      setBusy(false);
    }
  }
  return (
    <section aria-labelledby="screen-title" className="flex h-full flex-col">
      <ScreenHeader
        title={analysis.collection.title}
        subtitle={t("imports.summary", {
          provider: analysis.collection.provider,
          count: analysis.items.length,
        })}
      />
      <p className="mb-3 text-sm text-fg-muted">{t("imports.hint")}</p>
      <div className="mb-4 flex flex-wrap items-end gap-3">
        <Button onClick={() => setSelected(new Set())}>{t("collection.selectNone")}</Button>
        <Select
          label={t("imports.filter")}
          value={filter}
          onChange={(e) => setFilter(e.target.value)}
          options={["all", "ok", "review", "none"].map((value) => ({
            value,
            label: t(`imports.${value}`),
          }))}
        />
        <Select
          label={t("preview.profile")}
          value={profile}
          onChange={(e) => setProfile(e.target.value)}
          options={PROFILE_OPTIONS.map((p) => ({ value: p.id, label: t(p.labelKey) }))}
        />
        <Button
          onClick={() =>
            setSelected(
              new Set(items.filter((i) => i.matched.bucket === "ok").map((i) => i.track.id)),
            )
          }
        >
          {t("imports.selectOk")}
        </Button>
        <Button
          variant="primary"
          disabled={busy || !selected.size}
          onClick={() => void download("once")}
        >
          {t("collection.downloadSelected", { count: selected.size })}
        </Button>
        <Button disabled={busy} onClick={() => void download("sync")}>
          {t("sync.thisCollection")}
        </Button>
      </div>
      <VirtualTable
        className="flex-1"
        rows={items}
        rowKey={(i) => i.track.id}
        label={t("collection.title")}
        columns={[
          {
            key: "select",
            header: t("imports.selection"),
            width: "40px",
            render: (i) => (
              <Checkbox
                label={t("collection.selectTrack", { title: i.track.fields.title })}
                checked={selected.has(i.track.id)}
                disabled={i.matched.bucket === "none"}
                onChange={(on) =>
                  setSelected((prev) => {
                    const n = new Set(prev);
                    if (on) n.add(i.track.id);
                    else n.delete(i.track.id);
                    return n;
                  })
                }
              />
            ),
          },
          {
            key: "title",
            header: t("collection.columns.title"),
            width: "minmax(0,1fr)",
            render: (i) => (
              <span className="block truncate">
                {i.track.fields.title}
                <span className="block text-xs text-fg-muted">
                  {i.track.fields.artist} · {i.track.fields.album}
                </span>
              </span>
            ),
          },
          {
            key: "match",
            header: t("imports.match"),
            width: "170px",
            render: (i) => (
              <Badge
                tone={
                  i.matched.bucket === "ok"
                    ? "success"
                    : i.matched.bucket === "review"
                      ? "warning"
                      : "neutral"
                }
              >
                {t(`imports.${i.matched.bucket}`)}{" "}
                {i.matched.videoId
                  ? `${Math.round(i.matched.confidence * 100)}% · ${i.matched.via.toUpperCase()}`
                  : ""}
              </Badge>
            ),
          },
        ]}
      />
    </section>
  );
}
