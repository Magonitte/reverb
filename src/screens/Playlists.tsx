import { useEffect, useState } from "react";
import { ListMusic, RefreshCw } from "lucide-react";
import { Link, useNavigate, useParams } from "react-router";
import { useTranslation } from "react-i18next";
import type { Sync } from "@/bindings/Sync";
import type { SyncItem } from "@/bindings/SyncItem";
import { SyncDialog } from "@/components/playlists/SyncDialog";
import { ScreenHeader } from "@/components/ScreenHeader";
import { Button } from "@/components/ui/Button";
import { Badge } from "@/components/ui/Badge";
import { Dialog } from "@/components/ui/Dialog";
import { Checkbox } from "@/components/ui/Checkbox";
import { Toggle } from "@/components/ui/Toggle";
import { EmptyState } from "@/components/ui/EmptyState";
import { VirtualTable } from "@/components/ui/VirtualTable";
import { VinylDisc } from "@/components/ui/VinylDisc";
import { api } from "@/lib/ipc/api";
import { onEvent } from "@/lib/ipc/events";
import { errorText } from "@/lib/errors";
import { useSyncsStore } from "@/stores/syncs";
import { useUiStore } from "@/stores/ui";

export default function Playlists() {
  const { id } = useParams();
  const navigate = useNavigate();
  const { t, i18n } = useTranslation();
  const { syncs, loading, error, load } = useSyncsStore();
  const [edit, setEdit] = useState<Sync | "new" | null>(null);
  const [deleting, setDeleting] = useState<Sync | null>(null);
  const [deleteFiles, setDeleteFiles] = useState(false);
  const [busy, setBusy] = useState<Set<string>>(new Set());
  const [items, setItems] = useState<SyncItem[]>([]);
  const [itemError, setItemError] = useState<unknown>(null);
  const pushToast = useUiStore((state) => state.pushToast);
  const selected = id ? syncs.find((sync) => sync.id === id) : null;
  useEffect(() => {
    void load();
  }, [load]);
  useEffect(() => {
    if (!id) return;
    let alive = true;
    let revision = 0;
    let off: (() => void) | undefined;
    const refresh = async () => {
      const mine = ++revision;
      try {
        const next = await api.syncItems(id);
        if (alive && mine === revision) {
          setItems(next);
          setItemError(null);
        }
      } catch (error) {
        if (alive && mine === revision) setItemError(error);
      }
    };
    void refresh();
    void onEvent<{ id: string }>("sync://updated", (event) => {
      if (event.id === id) void refresh();
    }).then((remove) => {
      if (alive) off = remove;
      else remove();
    });
    return () => {
      alive = false;
      off?.();
    };
  }, [id]);
  const guard = async (sync: Sync, action: () => Promise<unknown>) => {
    setBusy((current) => new Set(current).add(sync.id));
    try {
      await action();
      await load();
    } catch (error) {
      pushToast({ message: errorText(t, error), tone: "error" });
    } finally {
      setBusy((current) => {
        const next = new Set(current);
        next.delete(sync.id);
        return next;
      });
    }
  };
  const date = (value: number | null) =>
    value === null ? t("sync.never") : new Date(value * 1000).toLocaleDateString(i18n.language);
  const next = (sync: Sync) =>
    !sync.enabled
      ? t("sync.disabled")
      : sync.intervalHours === 0
        ? t("sync.manual")
        : sync.lastSyncAt === null
          ? t("sync.due")
          : date(sync.lastSyncAt + sync.intervalHours * 3600);
  const status = (sync: Sync) =>
    t(
      sync.lastResult?.running
        ? "sync.running"
        : sync.lastResult?.error || sync.lastResult?.failed
          ? "sync.partial"
          : sync.lastSyncAt === null
            ? "sync.pending"
            : "sync.upToDate",
    );
  const itemStatus = (item: SyncItem) =>
    t(
      item.state === "removed"
        ? "sync.itemRemoved"
        : item.missing
          ? "sync.itemMissing"
          : item.libraryId !== null || item.jobStatus === "done"
            ? "sync.itemDownloaded"
            : item.jobStatus === "failed" || item.jobStatus === "cancelled"
              ? "sync.itemFailed"
              : item.jobStatus === "running"
                ? "sync.itemRunning"
                : "sync.itemQueued",
    );
  const actions = (sync: Sync) => (
    <div className="flex flex-wrap gap-2">
      <Button
        variant="secondary"
        disabled={busy.has(sync.id) || !!sync.lastResult?.running}
        onClick={() => void guard(sync, () => api.syncRun(sync.id))}
      >
        <RefreshCw className="size-3.5" aria-hidden="true" />
        {t("sync.run")}
      </Button>
      <Button
        variant="secondary"
        disabled={busy.has(sync.id) || !!sync.lastResult?.running}
        onClick={() => setEdit(sync)}
      >
        {t("sync.edit")}
      </Button>
      <Button
        variant="ghost"
        disabled={busy.has(sync.id)}
        onClick={() => {
          setDeleting(sync);
          setDeleteFiles(false);
        }}
      >
        {t("common.delete")}
      </Button>
    </div>
  );
  return (
    <section aria-labelledby="screen-title">
      <ScreenHeader
        title={selected?.title ?? t(id ? "playlistDetail.title" : "playlists.title")}
        subtitle={t(id ? "playlistDetail.subtitle" : "playlists.subtitle")}
        actions={
          !id && (
            <Button variant="primary" onClick={() => setEdit("new")}>
              {t("sync.create")}
            </Button>
          )
        }
      />
      {error !== null && (
        <p role="alert" className="mb-4 text-sm text-error">
          {errorText(t, error)}{" "}
          <Button variant="ghost" onClick={() => void load()}>
            {t("common.retry")}
          </Button>
        </p>
      )}
      {loading && (
        <p role="status" className="text-xs text-fg-muted">
          {t("common.loading")}
        </p>
      )}
      {!id && syncs.length === 0 && !loading && (
        <EmptyState
          icon={<ListMusic aria-hidden="true" />}
          title={t("playlists.empty.title")}
          description={t("playlists.empty.description")}
          action={
            <Button variant="primary" onClick={() => setEdit("new")}>
              {t("sync.create")}
            </Button>
          }
        />
      )}
      {!id && (
        <ul className="grid grid-cols-[repeat(auto-fit,minmax(280px,1fr))] gap-4">
          {syncs.map((sync) => (
            <li key={sync.id} className="glass rounded-xl p-4" data-testid="sync-card">
              <div className="mb-4 flex items-center gap-3">
                {sync.thumbnail ? (
                  <img src={sync.thumbnail} alt="" className="size-14 rounded-lg object-cover" />
                ) : (
                  <VinylDisc size={56} />
                )}
                <div className="min-w-0 flex-1">
                  <Link
                    to={`/playlists/${sync.id}`}
                    className="block truncate text-sm font-semibold text-fg hover:text-accent"
                  >
                    {sync.title}
                  </Link>
                  <p className="mt-1 text-xs text-fg-muted">
                    {t("sync.trackCount", { count: sync.itemCount })}
                  </p>
                </div>
                <Toggle
                  checked={sync.enabled}
                  label={t("sync.enabledFor", { title: sync.title })}
                  disabled={busy.has(sync.id) || !!sync.lastResult?.running}
                  onChange={(enabled) =>
                    void guard(sync, () =>
                      api.syncUpdate(sync.id, {
                        title: sync.title,
                        profileId: sync.profileId,
                        outputDir: sync.outputDir,
                        intervalHours: sync.intervalHours,
                        maxItems: sync.maxItems,
                        removeDeleted: sync.removeDeleted,
                        writeM3u: sync.writeM3u,
                        enabled,
                      }),
                    )
                  }
                />
              </div>
              <Badge>{status(sync)}</Badge>
              <dl className="my-4 grid grid-cols-2 gap-3 text-xs">
                <div>
                  <dt className="text-fg-muted">{t("sync.last")}</dt>
                  <dd className="mt-1 text-fg-secondary">{date(sync.lastSyncAt)}</dd>
                </div>
                <div>
                  <dt className="text-fg-muted">{t("sync.next")}</dt>
                  <dd className="mt-1 text-fg-secondary">{next(sync)}</dd>
                </div>
              </dl>
              {actions(sync)}
            </li>
          ))}
        </ul>
      )}
      {id && (
        <>
          <Link to="/playlists" className="mb-4 inline-block text-sm text-accent">
            {t("sync.back")}
          </Link>
          {selected && (
            <div className="mb-5 flex flex-wrap items-center justify-between gap-3">
              <Badge>{status(selected)}</Badge>
              {actions(selected)}
            </div>
          )}
          {itemError !== null && (
            <p role="alert" className="text-error">
              {errorText(t, itemError)}
            </p>
          )}
          {selected?.lastResult && (
            <div className="mb-4 space-y-2 text-xs text-fg-muted">
              {selected.lastResult.duplicates.length > 0 && (
                <p>{t("sync.duplicates", { count: selected.lastResult.duplicates.length })}</p>
              )}
              {selected.lastResult.unavailable > 0 && (
                <p>{t("sync.unavailable", { count: selected.lastResult.unavailable })}</p>
              )}
              {selected.lastResult.error && (
                <p role="alert" className="text-error">
                  {selected.lastResult.error}
                </p>
              )}
            </div>
          )}
          <VirtualTable
            rows={items}
            label={t("sync.tableLabel")}
            className="h-[min(60vh,560px)]"
            rowKey={(item) => item.sourceId}
            rowHeight={56}
            columns={[
              {
                key: "position",
                header: t("sync.position"),
                width: "48px",
                hiddenOnMobile: true,
                render: (item) => <span className="text-xs text-fg-muted">{item.position}</span>,
              },
              {
                key: "title",
                header: t("collection.columns.title"),
                width: "minmax(160px,1fr)",
                render: (item) => (
                  <span className="block truncate text-sm">{item.title ?? item.sourceId}</span>
                ),
              },
              {
                key: "state",
                header: t("sync.state"),
                width: "130px",
                render: (item) => <Badge>{itemStatus(item)}</Badge>,
              },
              {
                key: "open",
                header: t("sync.show"),
                width: "88px",
                hiddenOnMobile: true,
                render: (item) =>
                  item.filePath && !item.missing ? (
                    <Button
                      variant="ghost"
                      onClick={() =>
                        void api
                          .libraryReveal(item.filePath!)
                          .catch((error) =>
                            pushToast({ message: errorText(t, error), tone: "error" }),
                          )
                      }
                    >
                      {t("sync.show")}
                    </Button>
                  ) : null,
              },
            ]}
          />
        </>
      )}
      {edit !== null && (
        <SyncDialog
          sync={edit === "new" ? undefined : edit}
          onClose={() => setEdit(null)}
          onSaved={(sync) => {
            setEdit(null);
            navigate(`/playlists/${sync.id}`);
          }}
        />
      )}
      <Dialog
        open={deleting !== null}
        title={t("sync.deleteTitle")}
        closeLabel={t("common.close")}
        onClose={() => setDeleting(null)}
        footer={
          <>
            <Button variant="secondary" onClick={() => setDeleting(null)}>
              {t("common.cancel")}
            </Button>
            <Button
              disabled={!!deleting && busy.has(deleting.id)}
              onClick={() => {
                if (deleting)
                  void guard(deleting, async () => {
                    await api.syncDelete(deleting.id, deleteFiles);
                    setDeleting(null);
                    if (id === deleting.id) navigate("/playlists");
                  });
              }}
            >
              {t("common.delete")}
            </Button>
          </>
        }
      >
        <p className="mb-4 text-sm text-fg-secondary">
          {t("sync.deleteDescription", { title: deleting?.title })}
        </p>
        <Checkbox checked={deleteFiles} onChange={setDeleteFiles}>
          {t("sync.deleteFiles")}
        </Checkbox>
      </Dialog>
    </section>
  );
}
