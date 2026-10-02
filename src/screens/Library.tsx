import { useEffect, useState } from "react";
import { FileAudio, FolderOpen, Tags, Library as LibraryIcon } from "lucide-react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router";
import type { ImportReport } from "@/bindings/ImportReport";
import type { LibraryDateRange } from "@/bindings/LibraryDateRange";
import type { LibrarySort } from "@/bindings/LibrarySort";
import { ScreenHeader } from "@/components/ScreenHeader";
import { LibraryCover } from "@/components/LibraryCover";
import { IconButton } from "@/components/ui/IconButton";
import { Button } from "@/components/ui/Button";
import { Checkbox } from "@/components/ui/Checkbox";
import { ConfirmDialog } from "@/components/ui/ConfirmDialog";
import { EmptyState } from "@/components/ui/EmptyState";
import { SearchInput } from "@/components/ui/Input";
import { Select } from "@/components/ui/Select";
import { VirtualTable } from "@/components/ui/VirtualTable";
import { api } from "@/lib/ipc/api";
import { errorText } from "@/lib/errors";
import { onEvent } from "@/lib/ipc/events";
import { useLibraryStore } from "@/stores/library";
import { useUiStore } from "@/stores/ui";

type Confirmation = { kind: "clear" | "remove" | "trash"; ids: number[] };

export default function Library() {
  const { t, i18n } = useTranslation();
  const navigate = useNavigate();
  const { items, total, artists, albums, query, loading, error, load, setQuery } =
    useLibraryStore();
  const pushToast = useUiStore((s) => s.pushToast);
  const [selected, setSelected] = useState<number[]>([]);
  const [confirmation, setConfirmation] = useState<Confirmation | null>(null);
  const [pending, setPending] = useState(false);
  const [importing, setImporting] = useState(false);
  const [report, setReport] = useState<ImportReport | null>(null);
  const [view, setView] = useState<"list" | "albums">("list");
  const [progress, setProgress] = useState<{ processed: number; total: number } | null>(null);
  useEffect(() => {
    let disposed = false;
    let off = () => {};
    void onEvent<{ processed: number; total: number }>(
      "library://import-progress",
      setProgress,
    ).then((unlisten) => {
      if (disposed) unlisten();
      else off = unlisten;
    });
    return () => {
      disposed = true;
      off();
    };
  }, []);
  const importFiles = async (folder: boolean) => {
    const path = folder ? await api.pickFolder() : await api.pickAudioFile();
    if (!path) return;
    setImporting(true);
    try {
      setReport(await api.libraryImport([path]));
      await load();
    } finally {
      setImporting(false);
    }
  };
  const rescan = async () => {
    setImporting(true);
    try {
      setReport(await api.libraryRescan());
      await load();
    } finally {
      setImporting(false);
    }
  };
  useEffect(() => {
    void load();
  }, [load]);
  const offset = query.offset ?? 0;
  const limit = query.limit ?? 100;
  const visibleSelected = selected.filter((id) => items.some((i) => i.id === id));
  const run = async (action: () => Promise<unknown>) => {
    try {
      await action();
    } catch (e) {
      pushToast({ message: errorText(t, e), tone: "error" });
    }
  };
  const confirm = async () => {
    if (!confirmation || pending) return;
    setPending(true);
    try {
      if (confirmation.kind === "clear") await api.libraryClear();
      else await api.libraryDelete(confirmation.ids, confirmation.kind === "trash");
      setSelected([]);
      setConfirmation(null);
    } catch (e) {
      pushToast({ message: errorText(t, e), tone: "error" });
    } finally {
      await load();
      setPending(false);
    }
  };
  const select = (id: number, checked: boolean) =>
    setSelected((ids) => (checked ? [...new Set([...ids, id])] : ids.filter((v) => v !== id)));
  const change = (patch: Parameters<typeof setQuery>[0]) => {
    setSelected([]);
    void setQuery(patch);
  };
  const canRedownload =
    visibleSelected.length > 0 &&
    items.filter((i) => visibleSelected.includes(i.id)).every((i) => i.sourceUrl);
  return (
    <>
      <ScreenHeader
        title={t("library.title")}
        subtitle={t("library.subtitle")}
        actions={
          <div className="flex flex-wrap gap-2">
            <Button
              variant="ghost"
              disabled={importing}
              onClick={() => void run(() => importFiles(false))}
            >
              {t("library.importFile")}
            </Button>
            <Button
              variant="ghost"
              disabled={importing}
              onClick={() => void run(() => importFiles(true))}
            >
              {t("library.importFolder")}
            </Button>
            <Button variant="ghost" disabled={importing} onClick={() => void run(rescan)}>
              {t("library.rescan")}
            </Button>
            <Button
              variant="ghost"
              disabled={!total || pending}
              onClick={() => setConfirmation({ kind: "clear", ids: [] })}
            >
              {t("library.clear")}
            </Button>
          </div>
        }
      />
      {report && (
        <div role="status" className="mb-4 text-sm text-fg-muted">
          {t("library.importResult", {
            imported: report.imported,
            skipped: report.skipped,
            failed: report.failures.length,
          })}
          {report.failures.map((failure) => (
            <p key={failure.path}>
              {failure.path}: {failure.message}
            </p>
          ))}
        </div>
      )}
      <div className="mb-4 grid grid-cols-2 gap-3 md:grid-cols-4">
        <SearchInput
          aria-label={t("library.search")}
          placeholder={t("library.search")}
          value={query.text ?? ""}
          onChange={(e) => change({ text: e.target.value })}
          wrapperClassName="col-span-2"
        />
        <Select
          label={t("library.format")}
          value={query.format ?? ""}
          onChange={(e) => change({ format: e.target.value || null })}
          options={[
            { value: "", label: t("library.all") },
            ...["mp3", "m4a", "opus", "ogg", "flac", "wav"].map((v) => ({
              value: v,
              label: v.toUpperCase(),
            })),
          ]}
        />
        <Select
          label={t("library.date")}
          value={query.dateRange ?? "all"}
          onChange={(e) => change({ dateRange: e.target.value as LibraryDateRange })}
          options={["all", "today", "week", "month"].map((v) => ({
            value: v,
            label: t(`library.dates.${v}`),
          }))}
        />
        <Select
          label={t("library.artist")}
          value={query.artist ?? ""}
          onChange={(e) => change({ artist: e.target.value || null, album: null })}
          options={[
            { value: "", label: t("library.all") },
            ...artists.map((v) => ({ value: v, label: v })),
          ]}
        />
        <Select
          label={t("library.album")}
          value={query.album ?? ""}
          onChange={(e) => change({ album: e.target.value || null })}
          options={[
            { value: "", label: t("library.all") },
            ...albums.map((v) => ({ value: v, label: v })),
          ]}
        />
        <Select
          label={t("library.sort")}
          value={query.sort ?? "added_desc"}
          onChange={(e) => change({ sort: e.target.value as LibrarySort })}
          options={["added_desc", "title", "artist", "album"].map((v) => ({
            value: v,
            label: t(`library.sorts.${v}`),
          }))}
        />
        <div className="flex flex-col justify-end">
          <Checkbox
            checked={query.needsReview === true}
            onChange={(checked) => change({ needsReview: checked ? true : null })}
          >
            {t("library.needsReview")}
          </Checkbox>
          <Checkbox
            checked={query.missing === true}
            onChange={(checked) => change({ missing: checked ? true : null })}
          >
            {t("library.missing")}
          </Checkbox>
        </div>
      </div>
      {error != null && (
        <div role="alert" className="mb-4 text-error">
          {errorText(t, error)}{" "}
          <Button variant="ghost" onClick={() => void load()}>
            {t("common.retry")}
          </Button>
        </div>
      )}
      <div className="mb-3 flex flex-wrap items-center gap-2">
        <Checkbox
          checked={items.length > 0 && visibleSelected.length === items.length}
          disabled={!items.length || loading}
          onChange={(checked) => setSelected(checked ? items.map((i) => i.id) : [])}
        >
          {t("library.selectPage")}
        </Checkbox>
        <Button
          variant="ghost"
          disabled={!visibleSelected.length || loading || pending}
          onClick={() => setConfirmation({ kind: "remove", ids: visibleSelected })}
        >
          {t("library.remove")}
        </Button>
        <Button
          variant="ghost"
          disabled={!visibleSelected.length || loading || pending}
          onClick={() => setConfirmation({ kind: "trash", ids: visibleSelected })}
        >
          {t("library.trash")}
        </Button>
        <Button
          variant="ghost"
          disabled={!canRedownload || loading}
          onClick={() =>
            void run(async () => {
              for (const item of items.filter((i) => visibleSelected.includes(i.id)))
                await api.enqueue({
                  url: item.sourceUrl!,
                  profileId: item.profileId ?? undefined,
                  allowDuplicate: true,
                });
            })
          }
        >
          {t("library.redownload")}
        </Button>
      </div>
      <div className="mb-4 flex gap-2" role="group" aria-label={t("library.view")}>
        <Button
          variant={view === "list" ? "secondary" : "ghost"}
          aria-pressed={view === "list"}
          onClick={() => setView("list")}
        >
          {t("library.listView")}
        </Button>
        <Button
          variant={view === "albums" ? "secondary" : "ghost"}
          aria-pressed={view === "albums"}
          onClick={() => setView("albums")}
        >
          {t("library.albumView")}
        </Button>
      </div>
      {importing && progress && <p role="status">{t("library.importProgress", progress)}</p>}
      {view === "albums" && albums.length > 0 ? (
        <ul className="grid grid-cols-[repeat(auto-fill,minmax(160px,1fr))] gap-3">
          {albums.map((album) => {
            const representative = items.find((item) => item.album === album);
            return (
              <li key={album}>
                <button
                  type="button"
                  className="glass flex w-full items-center gap-3 rounded-lg p-4 text-left"
                  onClick={() => {
                    change({ album });
                    setView("list");
                  }}
                >
                  {representative && (
                    <LibraryCover
                      id={representative.id}
                      updatedAt={representative.updatedAt}
                      missing={representative.missing}
                    />
                  )}
                  <span className="truncate">{album}</span>
                </button>
              </li>
            );
          })}
        </ul>
      ) : items.length > 0 ? (
        <VirtualTable
          key={JSON.stringify(query)}
          label={t("library.title")}
          className="h-[420px]"
          rows={items}
          rowKey={(i) => String(i.id)}
          columns={[
            {
              key: "cover",
              header: t("library.cover"),
              width: "40px",
              render: (i) => <LibraryCover id={i.id} updatedAt={i.updatedAt} missing={i.missing} />,
            },
            {
              key: "select",
              header: t("library.selection"),
              width: "32px",
              render: (i) => (
                <Checkbox
                  checked={visibleSelected.includes(i.id)}
                  onChange={(checked) => select(i.id, checked)}
                  label={t("library.selectTrack", { title: i.title })}
                  disabled={loading}
                />
              ),
            },
            {
              key: "title",
              header: t("library.track"),
              width: "2fr",
              render: (i) => (
                <span title={i.title}>
                  <span>{i.title}</span>
                  {i.missing && <span className="ml-2 text-warning">{t("library.missing")}</span>}
                </span>
              ),
            },
            {
              key: "artist",
              header: t("library.artist"),
              hiddenOnMobile: true,
              render: (i) => i.artist ?? "",
            },
            {
              key: "album",
              header: t("library.album"),
              hiddenOnMobile: true,
              render: (i) => i.album ?? "",
            },
            {
              key: "format",
              hiddenOnMobile: true,
              header: t("library.format"),
              width: "60px",
              render: (i) => i.filePath.split(".").pop()?.toUpperCase(),
            },
            {
              key: "date",
              header: t("library.addedAt"),
              hiddenOnMobile: true,
              width: "100px",
              render: (i) => new Date(i.addedAt * 1000).toLocaleDateString(i18n.language),
            },
            {
              key: "actions",
              header: t("library.actions"),
              width: "116px",
              render: (i) => (
                <div className="flex gap-1">
                  <IconButton
                    disabled={i.missing || loading}
                    label={t("library.openTrack", { title: i.title })}
                    onClick={() => void run(() => api.libraryOpenFile(i.filePath))}
                  >
                    <FileAudio />
                  </IconButton>
                  <IconButton
                    disabled={i.missing || loading}
                    label={t("library.revealTrack", { title: i.title })}
                    onClick={() => void run(() => api.libraryReveal(i.filePath))}
                  >
                    <FolderOpen />
                  </IconButton>
                  <IconButton
                    label={t("library.editTrack", { title: i.title })}
                    disabled={i.missing || loading}
                    onClick={() => navigate(`/tag-editor?path=${encodeURIComponent(i.filePath)}`)}
                  >
                    <Tags />
                  </IconButton>
                </div>
              ),
            },
          ]}
        />
      ) : loading ? (
        <p role="status">{t("common.loading")}</p>
      ) : (
        <EmptyState
          icon={<LibraryIcon />}
          title={t("library.empty.title")}
          description={t("library.empty.description")}
        />
      )}
      <div className="mt-4 flex flex-wrap items-center justify-between gap-2">
        <p role="status" aria-live="polite" className="text-sm text-fg-muted">
          {t("library.page", {
            start: total ? offset + 1 : 0,
            end: Math.min(offset + items.length, total),
            total,
          })}
        </p>
        <div className="flex gap-2">
          <Button
            variant="ghost"
            disabled={offset === 0 || loading}
            onClick={() => change({ offset: Math.max(0, offset - limit) })}
          >
            {t("library.previous")}
          </Button>
          <Button
            variant="ghost"
            disabled={offset + limit >= total || loading}
            onClick={() => change({ offset: offset + limit })}
          >
            {t("library.next")}
          </Button>
        </div>
      </div>
      <ConfirmDialog
        open={confirmation != null}
        title={t(`library.confirm.${confirmation?.kind ?? "clear"}.title`)}
        message={t(`library.confirm.${confirmation?.kind ?? "clear"}.message`)}
        confirmLabel={pending ? t("common.loading") : t("common.confirm")}
        cancelLabel={t("common.cancel")}
        closeLabel={t("common.close")}
        danger
        onConfirm={() => void confirm()}
        onCancel={() => {
          if (!pending) setConfirmation(null);
        }}
      />
    </>
  );
}
