import { ArtistsPanel } from "@/components/ArtistsPanel";
import { useSearchParams } from "react-router";
import { TrimDialog } from "@/components/TrimDialog";
import { LosslessDialog } from "@/components/LosslessDialog";
import { UpgradePanel } from "@/components/UpgradePanel";
import { useEffect, useState } from "react";
import {
  FileAudio,
  AudioLines,
  FolderOpen,
  Tags,
  ArrowUp,
  Scissors,
  Library as LibraryIcon,
  MoreHorizontal,
} from "lucide-react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router";
import type { ImportReport } from "@/bindings/ImportReport";
import type { LibraryItem } from "@/bindings/LibraryItem";
import type { LibraryDateRange } from "@/bindings/LibraryDateRange";
import type { LibrarySort } from "@/bindings/LibrarySort";
import { ScreenHeader } from "@/components/ScreenHeader";
import { LibraryCover } from "@/components/LibraryCover";
import { IconButton } from "@/components/ui/IconButton";
import { Button } from "@/components/ui/Button";
import { Checkbox } from "@/components/ui/Checkbox";
import { ConfirmDialog } from "@/components/ui/ConfirmDialog";
import { Dialog } from "@/components/ui/Dialog";
import { EmptyState } from "@/components/ui/EmptyState";
import { SearchInput } from "@/components/ui/Input";
import { Select } from "@/components/ui/Select";
import { VirtualTable } from "@/components/ui/VirtualTable";
import { api } from "@/lib/ipc/api";
import { errorText } from "@/lib/errors";
import { onEvent } from "@/lib/ipc/events";
import { defaultLibraryQuery, useLibraryStore } from "@/stores/library";
import { useUiStore } from "@/stores/ui";

type Confirmation = { kind: "clear" | "remove" | "trash"; ids: number[] };
const LIBRARY_TABS = ["library", "artists", "missing"];

export default function Library() {
  const { t } = useTranslation();
  const [params, setParams] = useSearchParams();
  const [tab, setTab] = useState(() => (params.has("follow") ? "artists" : "library"));
  const hasFollow = params.has("follow");
  const [previousFollow, setPreviousFollow] = useState(hasFollow);
  if (hasFollow !== previousFollow) {
    setPreviousFollow(hasFollow);
    if (hasFollow) setTab("artists");
  }
  const active = params.has("follow") ? "artists" : tab;
  return (
    <>
      <div className="mb-4 flex flex-wrap gap-2" role="group" aria-label={t("artists.views")}>
        {LIBRARY_TABS.map((value) => (
          <Button
            key={value}
            variant={active === value ? "secondary" : "ghost"}
            aria-pressed={active === value}
            onClick={() => {
              setTab(value);
              if (hasFollow) setParams({});
            }}
          >
            {t(`artists.tabs.${value}`)}
          </Button>
        ))}
      </div>
      {active === "library" ? (
        <LibraryFiles />
      ) : (
        <>
          <ScreenHeader title={t(`artists.tabs.${active}`)} />
          <ArtistsPanel missingOnly={active === "missing"} />
        </>
      )}
    </>
  );
}
function LibraryFiles() {
  const { t, i18n } = useTranslation();
  const navigate = useNavigate();
  const { items, total, artists, albums, query, loading, error, load, setQuery } =
    useLibraryStore();
  const pushToast = useUiStore((s) => s.pushToast);
  const [trim, setTrim] = useState<{ path: string; duration: number } | null>(null);
  const [lossless, setLossless] = useState<number | null>(null);
  const [selected, setSelected] = useState<number[]>([]);
  const [confirmation, setConfirmation] = useState<Confirmation | null>(null);
  const [pending, setPending] = useState(false);
  const [importing, setImporting] = useState(false);
  const [report, setReport] = useState<ImportReport | null>(null);
  const [importError, setImportError] = useState<unknown>(null);
  const [details, setDetails] = useState<LibraryItem | null>(null);
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
    setImporting(true);
    setProgress(null);
    setImportError(null);
    setReport(null);
    try {
      const path = folder ? await api.pickFolder() : await api.pickAudioFile();
      if (!path) return;
      setReport(await api.libraryImport([path]));
      setSelected([]);
      setView("list");
      await setQuery({
        ...defaultLibraryQuery,
        text: null,
        artist: null,
        album: null,
        format: null,
        missing: null,
        needsReview: null,
      });
    } catch (error) {
      setImportError(error);
    } finally {
      await load();
      setImporting(false);
    }
  };
  const rescan = async () => {
    setImporting(true);
    setProgress(null);
    setReport(null);
    setImportError(null);
    try {
      setReport(await api.libraryRescan());
      await load();
    } catch (error) {
      setImportError(error);
    } finally {
      setImporting(false);
    }
  };
  useEffect(() => {
    // Review uses the same store; entering Library must not inherit its review-only query.
    void setQuery({ needsReview: null });
  }, [setQuery]);
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
  const hasFilters = !!(
    query.text ||
    query.artist ||
    query.album ||
    query.format ||
    query.missing ||
    query.needsReview ||
    (query.dateRange && query.dateRange !== "all")
  );
  const resetFilters = () =>
    change({
      ...defaultLibraryQuery,
      text: null,
      artist: null,
      album: null,
      format: null,
      missing: null,
      needsReview: null,
    });
  const pageAlbums = [
    ...new Set(items.map((item) => item.album).filter((album): album is string => !!album)),
  ];
  return (
    <>
      <ScreenHeader
        title={t("library.title")}
        subtitle={t("library.subtitle")}
        actions={
          <div className="flex flex-wrap gap-2">
            <Button
              variant="primary"
              icon={<FileAudio className="size-4" aria-hidden="true" />}
              disabled={importing || pending}
              onClick={() => void run(() => importFiles(false))}
            >
              {t("library.importFile")}
            </Button>
            <Button
              variant="secondary"
              icon={<FolderOpen className="size-4" aria-hidden="true" />}
              disabled={importing || pending}
              onClick={() => void run(() => importFiles(true))}
            >
              {t("library.importFolder")}
            </Button>
            <Button
              variant="ghost"
              disabled={importing || pending}
              onClick={() => void run(rescan)}
            >
              {t("library.rescan")}
            </Button>
            <Button
              variant="ghost"
              disabled={!total || pending || importing}
              onClick={() => setConfirmation({ kind: "clear", ids: [] })}
            >
              {t("library.clear")}
            </Button>
          </div>
        }
      />
      {importing && (
        <div role="status" className="glass mb-4 rounded-lg p-4 text-sm">
          {progress ? t("library.importProgress", progress) : t("library.discovering")}
          {progress && progress.total > 0 && (
            <progress
              aria-label={t("library.importFolder")}
              className="mt-2 w-full accent-accent"
              value={progress.processed}
              max={progress.total}
            />
          )}
        </div>
      )}
      {importError != null && (
        <div role="alert" className="mb-4 rounded-lg border border-error/30 p-4 text-error">
          {errorText(t, importError)}
        </div>
      )}
      {report && (
        <div role="status" className="glass mb-4 rounded-lg p-4 text-sm text-fg-secondary">
          {t("library.importResult", {
            imported: report.imported,
            skipped: report.skipped,
            failed: report.failures.length,
          })}
          {!report.imported && !report.skipped && !report.failures.length && (
            <p className="mt-2">{t("library.noAudio")}</p>
          )}
          {report.failures.length > 0 && (
            <details className="mt-2">
              <summary className="cursor-pointer">{t("library.importFailures")}</summary>
              {report.failures.map((failure, index) => (
                <p key={`${failure.path}:${index}`} className="mt-2 break-all">
                  {failure.path}: {t(failure.message, { defaultValue: failure.message })}
                </p>
              ))}
            </details>
          )}
        </div>
      )}
      <div className="glass mb-4 grid grid-cols-2 gap-3 rounded-lg p-4 md:grid-cols-4">
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
        {hasFilters && (
          <Button variant="ghost" onClick={resetFilters}>
            {t("library.resetFilters")}
          </Button>
        )}
      </div>
      {error != null && (
        <div role="alert" className="mb-4 text-error">
          {errorText(t, error)}{" "}
          <Button variant="ghost" onClick={() => void load()}>
            {t("common.retry")}
          </Button>
        </div>
      )}
      {items.length > 0 && (
        <div className="mb-3 flex flex-wrap items-center gap-2">
          <Checkbox
            checked={items.length > 0 && visibleSelected.length === items.length}
            disabled={!items.length || loading}
            onChange={(checked) => setSelected(checked ? items.map((i) => i.id) : [])}
          >
            {t("library.selectPage")}
          </Checkbox>
          {visibleSelected.length > 0 && (
            <>
              <span className="text-sm text-fg-muted">
                {t("library.selected", { count: visibleSelected.length })}
              </span>
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
            </>
          )}
        </div>
      )}
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
      {view === "albums" && pageAlbums.length > 0 ? (
        <ul className="grid grid-cols-[repeat(auto-fill,minmax(160px,1fr))] gap-3">
          {pageAlbums.map((album) => {
            const representative = items.find((item) => item.album === album);
            return (
              <li key={album}>
                <button
                  type="button"
                  className="glass flex w-full flex-col items-center gap-3 rounded-lg p-4 text-center transition-colors hover:bg-hover"
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
                      size={112}
                    />
                  )}
                  <span className="line-clamp-2 w-full break-words font-medium">{album}</span>
                </button>
              </li>
            );
          })}
        </ul>
      ) : view === "albums" && items.length > 0 ? (
        <EmptyState
          icon={<LibraryIcon />}
          title={t("library.noAlbums")}
          description={t("library.noAlbumsHint")}
        />
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
              width: "minmax(0,2fr)",
              render: (i) => (
                <span title={i.title}>
                  <span>{i.title}</span>
                  {i.provider && i.provider !== "youtube" && (
                    <span className="ml-2 text-xs text-fg-muted">{t(`sources.${i.provider}`)}</span>
                  )}
                  {i.codec === "flac" && (
                    <span className="ml-2 text-xs text-fg-muted">{t("lossless.flac")}</span>
                  )}
                  {i.losslessVerdict && (
                    <span className="ml-2 text-xs text-fg-muted">
                      {t(`lossless.verdicts.${i.losslessVerdict}`)}
                    </span>
                  )}
                  {i.sourceAbrKbps !== null && (
                    <span className="ml-2 text-xs text-fg-muted">
                      {i.sourceAbrKbps >= 200 ? t("quality.premium") : t("quality.sourceQuality")}{" "}
                      {Math.round(i.sourceAbrKbps)} kbps
                    </span>
                  )}
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
              width: "108px",
              render: (i) => (
                <div className="flex gap-1">
                  <IconButton
                    label={t("library.detailsTrack", { title: i.title })}
                    onClick={() => setDetails(i)}
                  >
                    <MoreHorizontal />
                  </IconButton>
                  <IconButton
                    label={t("library.editTrack", { title: i.title })}
                    disabled={i.missing || loading}
                    onClick={() => navigate(`/tag-editor?path=${encodeURIComponent(i.filePath)}`)}
                  >
                    <Tags />
                  </IconButton>
                  <IconButton
                    disabled={i.missing || loading}
                    label={t("library.openTrack", { title: i.title })}
                    onClick={() => void run(() => api.libraryOpenFile(i.filePath))}
                  >
                    <FileAudio />
                  </IconButton>
                </div>
              ),
            },
          ]}
        />
      ) : loading ? (
        <p role="status">{t("common.loading")}</p>
      ) : error == null ? (
        <EmptyState
          icon={<LibraryIcon />}
          title={t(hasFilters ? "library.noResults" : "library.empty.title")}
          description={t(hasFilters ? "library.noResultsHint" : "library.empty.description")}
        />
      ) : null}
      {details && (
        <Dialog
          open
          title={details.title}
          closeLabel={t("common.close")}
          onClose={() => setDetails(null)}
        >
          <div className="space-y-4">
            <LibraryCover
              id={details.id}
              updatedAt={details.updatedAt}
              missing={details.missing}
              size={160}
            />
            <p>
              {details.artist} · {details.album}
            </p>
            <p className="break-all text-sm text-fg-muted">{details.filePath}</p>
            <p className="text-sm text-fg-muted">
              {details.codec?.toUpperCase()} ·{" "}
              {details.durationS == null
                ? t("metadataDetails.unavailable")
                : t("review.duration", { seconds: Math.round(details.durationS) })}
            </p>
            <dl className="grid grid-cols-2 gap-3 text-sm">
              {[
                [t("tagEditor.fields.year"), details.year],
                [t("tagEditor.fields.genre"), details.genre],
                [t("metadataDetails.isrc"), details.isrc],
                [t("metadataDetails.provider"), details.metadataSource ?? details.provider],
              ].map(([label, value]) => (
                <div key={String(label)}>
                  <dt className="text-fg-muted">{label}</dt>
                  <dd className="break-words">{value ?? t("metadataDetails.unavailable")}</dd>
                </div>
              ))}
            </dl>
            {details.missing && (
              <p role="alert" className="text-warning">
                {t("library.missing")}
              </p>
            )}
            <div className="flex flex-wrap gap-2">
              <Button
                aria-label={`${t("lossless.title")} ${details.title}`}
                disabled={details.missing || loading}
                onClick={() => {
                  setDetails(null);
                  setLossless(details.id);
                }}
              >
                <AudioLines className="size-4" aria-hidden="true" />
                {t("lossless.title")}
              </Button>
              <Button
                disabled={details.missing || loading}
                aria-label={t("library.openTrack", { title: details.title })}
                onClick={() => void run(() => api.libraryOpenFile(details.filePath))}
              >
                <FileAudio className="size-4" aria-hidden="true" />
                {t("library.open")}
              </Button>
              <Button
                disabled={details.missing || loading}
                aria-label={t("library.revealTrack", { title: details.title })}
                onClick={() => void run(() => api.libraryReveal(details.filePath))}
              >
                <FolderOpen className="size-4" aria-hidden="true" />
                {t("library.reveal")}
              </Button>
              <Button
                aria-label={`${t("quality.trim")} ${details.title}`}
                disabled={details.missing || !details.durationS}
                onClick={() => {
                  setDetails(null);
                  setTrim({ path: details.filePath, duration: details.durationS ?? 0 });
                }}
              >
                <Scissors className="size-4" aria-hidden="true" />
                {t("quality.trim")}
              </Button>
              <Button
                aria-label={`${t("quality.upgrade")} ${details.title}`}
                disabled={
                  details.missing ||
                  details.provider !== "youtube" ||
                  (details.sourceAbrKbps ?? 200) >= 200
                }
                onClick={() =>
                  void run(async () => {
                    const available = await api.upgradeScan([details.id]);
                    if (available.length) {
                      await api.upgradeEnqueue([details.id]);
                      pushToast({ message: t("quality.queued"), tone: "success" });
                    } else {
                      pushToast({ message: t("quality.found", { count: 0 }), tone: "info" });
                    }
                  })
                }
              >
                <ArrowUp className="size-4" aria-hidden="true" />
                {t("quality.upgrade")}
              </Button>
              <Button
                aria-label={t("library.editTrack", { title: details.title })}
                disabled={details.missing || loading}
                onClick={() => navigate(`/tag-editor?path=${encodeURIComponent(details.filePath)}`)}
              >
                <Tags className="size-4" aria-hidden="true" />
                {t("tagEditor.title")}
              </Button>
            </div>
          </div>
        </Dialog>
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
      {visibleSelected.length > 0 && (
        <UpgradePanel key={visibleSelected.join(",")} ids={visibleSelected} />
      )}
      {trim && (
        <TrimDialog
          path={trim.path}
          duration={trim.duration}
          onClose={() => setTrim(null)}
          onDone={() => void load()}
        />
      )}
      {lossless !== null && <LosslessDialog id={lossless} onClose={() => setLossless(null)} />}
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
