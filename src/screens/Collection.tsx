import { useEffect, useMemo, useState } from "react";
import { ListMusic } from "lucide-react";
import { Link } from "react-router";
import { useTranslation } from "react-i18next";
import type { CollectionEntry } from "@/bindings/CollectionEntry";
import { ScreenHeader } from "@/components/ScreenHeader";
import { Badge } from "@/components/ui/Badge";
import { Button } from "@/components/ui/Button";
import { Checkbox } from "@/components/ui/Checkbox";
import { EmptyState } from "@/components/ui/EmptyState";
import { SearchInput } from "@/components/ui/Input";
import { Select } from "@/components/ui/Select";
import { VirtualTable, type VirtualColumn } from "@/components/ui/VirtualTable";
import { errorText } from "@/lib/errors";
import { formatDuration, thumbnailFor } from "@/lib/format";
import { api } from "@/lib/ipc/api";
import { PROFILE_OPTIONS } from "@/lib/profiles";
import { useFlowStore } from "@/stores/flow";
import { activeCount, useJobsStore } from "@/stores/jobs";
import { useSettingsStore } from "@/stores/settings";
import { useUiStore } from "@/stores/ui";
import { SyncDialog } from "@/components/playlists/SyncDialog";
import { useNavigate } from "react-router";
import { ImportedCollectionView } from "@/components/ImportedCollectionView";

/** Faixa da coleção com a posição original (1-based), que vira o `playlistCtx.index`. */
interface Row {
  entry: CollectionEntry;
  index: number;
}

const WARN_RATIO = 0.9;

export default function Collection() {
  const imported = useFlowStore((s) => s.imported);
  return imported ? <ImportedCollectionView analysis={imported} /> : <YoutubeCollection />;
}
function YoutubeCollection() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const collectionUrl = useFlowStore((state) => state.collectionUrl);
  const [syncDialog, setSyncDialog] = useState(false);
  const collection = useFlowStore((s) => s.collection);
  const settings = useSettingsStore((s) => s.settings);
  const jobs = useJobsStore((s) => s.jobs);
  const pushToast = useUiStore((s) => s.pushToast);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [filter, setFilter] = useState("");
  const [profileId, setProfileId] = useState(settings?.defaultProfile ?? "original");
  const [downloaded, setDownloaded] = useState<Set<string>>(new Set());
  const [busy, setBusy] = useState(false);

  const rows = useMemo<Row[]>(
    () => (collection?.entries ?? []).map((entry, i) => ({ entry, index: i + 1 })),
    [collection],
  );
  const visible = useMemo(() => {
    const needle = filter.trim().toLowerCase();
    return needle
      ? rows.filter((r) => (r.entry.title ?? r.entry.id).toLowerCase().includes(needle))
      : rows;
  }, [rows, filter]);

  // Marca as já baixadas (fila ou biblioteca) para o perfil escolhido.
  useEffect(() => {
    if (rows.length === 0) return;
    let alive = true;
    api
      .checkDuplicates(
        rows.map((r) => r.entry.id),
        profileId,
      )
      .then((hits) => alive && setDownloaded(new Set(hits.map((h) => h.sourceId))))
      .catch(() => alive && setDownloaded(new Set()));
    return () => {
      alive = false;
    };
  }, [rows, profileId]);

  if (!collection) {
    return (
      <section aria-labelledby="screen-title">
        <ScreenHeader title={t("collection.title")} subtitle={t("collection.subtitle")} />
        <EmptyState
          icon={<ListMusic aria-hidden="true" />}
          title={t("collection.empty.title")}
          description={t("collection.empty.description")}
          action={
            <Link to="/" className="text-[13px] text-accent hover:text-accent-hover">
              {t("collection.empty.action")}
            </Link>
          }
        />
      </section>
    );
  }

  const totalSeconds = rows.reduce((sum, r) => sum + (r.entry.duration ?? 0), 0);
  const queueLimit = settings?.queueLimit ?? 500;
  const inQueue = activeCount(jobs);
  const nearLimit = inQueue >= queueLimit * WARN_RATIO;

  const toggle = (id: string, on: boolean) =>
    setSelected((current) => {
      const next = new Set(current);
      if (on) next.add(id);
      else next.delete(id);
      return next;
    });

  const selectOnly = (rowsToSelect: Row[]) =>
    setSelected(new Set(rowsToSelect.map((r) => r.entry.id)));

  const download = async () => {
    const chosen = rows.filter((r) => selected.has(r.entry.id));
    if (chosen.length === 0) return;
    setBusy(true);
    let added = 0;
    let existing = 0;
    try {
      for (const { entry, index } of chosen) {
        try {
          await api.enqueue({
            url: entry.url ?? `https://www.youtube.com/watch?v=${entry.id}`,
            sourceId: entry.id,
            title: entry.title ?? undefined,
            thumbnail: thumbnailFor(entry.id),
            durationS: entry.duration ?? undefined,
            profileId,
            metadataOverride: null,
            playlistCtx: {
              playlistTitle: collection.title ?? "",
              playlistId: collection.id ?? "",
              index,
            },
            priority: false,
            allowDuplicate: false,
          });
          added += 1;
        } catch (e) {
          if ((e as { kind?: string }).kind === "duplicate") existing += 1;
          else throw e;
        }
      }
      pushToast({
        message:
          existing > 0
            ? t("collection.queuedWithExisting", { count: added, existing })
            : t("collection.queued", { count: added }),
        tone: "success",
      });
      setSelected(new Set());
    } catch (e) {
      pushToast({ message: errorText(t, e), tone: "error" });
    } finally {
      setBusy(false);
    }
  };

  const allVisibleSelected = visible.length > 0 && visible.every((r) => selected.has(r.entry.id));

  const columns: VirtualColumn<Row>[] = [
    {
      key: "select",
      header: "",
      width: "36px",
      render: ({ entry }) => (
        <Checkbox
          checked={selected.has(entry.id)}
          onChange={(on) => toggle(entry.id, on)}
          label={t("collection.selectTrack", { title: entry.title ?? entry.id })}
        />
      ),
    },
    {
      key: "title",
      header: t("collection.columns.title"),
      width: "minmax(0,1fr)",
      render: ({ entry, index }) => (
        <span className="flex items-center gap-3">
          <span className="w-6 shrink-0 text-right text-xs text-fg-dim">{index}</span>
          <img
            src={thumbnailFor(entry.id)}
            alt=""
            loading="lazy"
            className="size-8 shrink-0 rounded-sm bg-field object-cover"
          />
          <span className="truncate">{entry.title ?? entry.id}</span>
        </span>
      ),
    },
    {
      key: "duration",
      header: t("collection.columns.duration"),
      width: "72px",
      render: ({ entry }) => formatDuration(entry.duration),
    },
    {
      key: "state",
      header: t("collection.columns.state"),
      width: "120px",
      render: ({ entry }) =>
        downloaded.has(entry.id) ? (
          <Badge tone="success">{t("collection.alreadyDownloaded")}</Badge>
        ) : null,
    },
  ];

  return (
    <section aria-labelledby="screen-title" className="flex h-full flex-col">
      <ScreenHeader
        actions={
          !collectionUrl || /youtube\.com|youtu\.be/.test(new URL(collectionUrl).hostname) ? (
            <Button variant="secondary" onClick={() => setSyncDialog(true)}>
              {t("sync.thisCollection")}
            </Button>
          ) : undefined
        }
        title={collection.title ?? t("collection.title")}
        subtitle={t("collection.summary", {
          channel: collection.channel ?? "—",
          count: rows.length,
          duration: formatDuration(totalSeconds),
        })}
      />
      {nearLimit && (
        <p
          role="status"
          data-testid="queue-limit-warning"
          className="mb-4 text-[13px] text-warning"
        >
          {t("collection.queueNearLimit", { count: inQueue, limit: queueLimit })}
        </p>
      )}
      <div className="mb-4 flex flex-wrap items-end gap-3">
        <SearchInput
          wrapperClassName="min-w-[220px] flex-1"
          value={filter}
          onChange={(e) => setFilter(e.target.value)}
          placeholder={t("collection.filter")}
          aria-label={t("collection.filter")}
        />
        <Select
          label={t("preview.profile")}
          value={profileId}
          onChange={(e) => setProfileId(e.target.value)}
          options={PROFILE_OPTIONS.map((p) => ({ value: p.id, label: t(p.labelKey) }))}
        />
      </div>
      <div className="mb-4 flex flex-wrap items-center gap-2">
        <Button size="sm" onClick={() => selectOnly(allVisibleSelected ? [] : visible)}>
          {allVisibleSelected ? t("collection.selectNone") : t("collection.selectAll")}
        </Button>
        <Button
          size="sm"
          onClick={() => selectOnly(visible.filter((r) => !downloaded.has(r.entry.id)))}
        >
          {t("collection.selectNew")}
        </Button>
        <span className="flex-1" />
        <Button variant="primary" loading={busy} disabled={selected.size === 0} onClick={download}>
          {t("collection.downloadSelected", { count: selected.size })}
        </Button>
      </div>
      <VirtualTable
        className="h-[min(60vh,560px)]"
        label={t("collection.tableLabel")}
        rows={visible}
        rowKey={(r) => `${r.index}-${r.entry.id}`}
        columns={columns}
      />
      {syncDialog && (
        <SyncDialog
          url={collectionUrl ?? `https://www.youtube.com/playlist?list=${collection.id ?? ""}`}
          onClose={() => setSyncDialog(false)}
          onSaved={(sync) => {
            setSyncDialog(false);
            navigate(`/playlists/${sync.id}`);
          }}
        />
      )}
    </section>
  );
}
