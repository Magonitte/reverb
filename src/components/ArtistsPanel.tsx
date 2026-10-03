import { useCallback, useEffect, useRef, useState } from "react";
import { Check, FolderOpen, Music2, Search, UserRound } from "lucide-react";
import { useTranslation } from "react-i18next";
import { useSearchParams } from "react-router";
import type { ArtistOptions } from "@/bindings/ArtistOptions";
import type { ArtistHit } from "@/bindings/ArtistHit";
import type { FollowedArtist } from "@/bindings/FollowedArtist";
import type { ArtistRelease } from "@/bindings/ArtistRelease";
import { api } from "@/lib/ipc/api";
import { onEvent } from "@/lib/ipc/events";
import { errorText } from "@/lib/errors";
import { Button, Card, Input, Dialog, Select, Checkbox } from "./ui";
import { EmptyState } from "./ui/EmptyState";
import { PROFILE_OPTIONS } from "@/lib/profiles";
import { useUiStore } from "@/stores/ui";
import { useSettingsStore } from "@/stores/settings";

const defaults: ArtistOptions = {
  monitorExisting: "latest",
  monitorNew: "notify",
  types: ["album", "ep", "single"],
  excludeVariants: true,
  profileId: "original",
  outputDir: null,
};
const RELEASE_TYPES = ["album", "ep", "single"];

export function ArtistsPanel({ missingOnly = false }: { missingOnly?: boolean }) {
  const { t, i18n } = useTranslation();
  const toast = useUiStore((s) => s.pushToast);
  const settings = useSettingsStore((s) => s.settings);
  const [params, setParams] = useSearchParams();
  const [artists, setArtists] = useState<FollowedArtist[]>([]);
  const [releases, setReleases] = useState<ArtistRelease[]>([]);
  const [name, setName] = useState("");
  const [hits, setHits] = useState<ArtistHit[]>([]);
  const [searched, setSearched] = useState(false);
  const [searching, setSearching] = useState(false);
  const [open, setOpen] = useState(false);
  const [choice, setChoice] = useState<string | null>(null);
  const [options, setOptions] = useState(defaults);
  const [action, setAction] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [dialogError, setDialogError] = useState<string | null>(null);
  const [editing, setEditing] = useState<FollowedArtist | null>(null);
  const [expanded, setExpanded] = useState<string | null>(null);
  const [artistFilter, setArtistFilter] = useState("");
  const mounted = useRef(true);
  const loadSerial = useRef(0);
  const searchSerial = useRef(0);

  const load = useCallback(async () => {
    const serial = ++loadSerial.current;
    const list = await api.artistsFollowed();
    const items = missingOnly
      ? await api.missingList()
      : (await Promise.all(list.map((a) => api.artistReleases(a.id)))).flat();
    if (mounted.current && serial === loadSerial.current) {
      setArtists(list);
      setReleases(items);
      setLoading(false);
    }
  }, [missingOnly]);
  const run = useCallback(
    async (kind: string, work: () => Promise<unknown>) => {
      setAction(kind);
      try {
        await work();
        await load();
      } catch (e) {
        toast({ message: errorText(t, e), tone: "error" });
      } finally {
        if (mounted.current) setAction(null);
      }
    },
    [load, t, toast],
  );
  useEffect(() => {
    mounted.current = true;
    ++searchSerial.current;
    let active = true;
    let off = () => {};
    const refresh = () =>
      void load().catch((e) => {
        if (active) {
          setLoading(false);
          toast({ message: errorText(t, e), tone: "error" });
        }
      });
    refresh();
    void onEvent("artists://updated", refresh).then((fn) => {
      if (active) off = fn;
      else fn();
    });
    return () => {
      active = false;
      mounted.current = false;
      off();
    };
  }, [load, t, toast]);

  const followId = params.get("follow");
  const [previousFollow, setPreviousFollow] = useState<string | null>(null);
  if (followId !== previousFollow) {
    setPreviousFollow(followId);
    if (followId) {
      setChoice(followId);
      setOptions(defaults);
      setEditing(null);
      setHits([]);
      setName("");
      setSearched(false);
      setDialogError(null);
      setOpen(true);
    }
  }
  const busy = action !== null;
  const selected = hits.find((h) => h.id === choice);
  function close() {
    if (action === "save") return;
    ++searchSerial.current;
    setSearching(false);
    setOpen(false);
    setEditing(null);
    if (followId) {
      const next = new URLSearchParams(params);
      next.delete("follow");
      setParams(next);
    }
  }
  function startFollow() {
    ++searchSerial.current;
    setEditing(null);
    setChoice(null);
    setOptions(defaults);
    setHits([]);
    setName("");
    setSearched(false);
    setSearching(false);
    setDialogError(null);
    setOpen(true);
  }
  async function search() {
    const serial = ++searchSerial.current;
    setSearching(true);
    setChoice(null);
    setHits([]);
    setDialogError(null);
    try {
      const result = await api.artistsSearch(name.trim());
      if (mounted.current && serial === searchSerial.current) {
        setHits(result);
        setSearched(true);
      }
    } catch (e) {
      if (mounted.current && serial === searchSerial.current) setDialogError(errorText(t, e));
    } finally {
      if (mounted.current && serial === searchSerial.current) setSearching(false);
    }
  }
  async function save() {
    setAction("save");
    setDialogError(null);
    const isNew = !editing;
    try {
      if (editing) await api.artistUpdate(editing.id, options);
      else if (choice) await api.artistFollow(choice, options);
      // Persist following before checking. A failed catalog check must not invite duplicate saves.
      setOpen(false);
      setEditing(null);
      if (followId) {
        const next = new URLSearchParams(params);
        next.delete("follow");
        setParams(next);
      }
    } catch (e) {
      setDialogError(errorText(t, e));
      setAction(null);
      return;
    }
    setAction(null);
    toast({ message: t("artists.saved"), tone: "success" });
    if (isNew) void run("check", () => api.artistsCheckNow());
    else void load().catch((e) => toast({ message: errorText(t, e), tone: "error" }));
  }
  const releaseList = (items: ArtistRelease[]) => (
    <ul className="space-y-2">
      {items.map((r) => (
        <li key={`${r.artistId}:${r.id}`}>
          <Card className="flex flex-wrap items-center gap-3 !p-3">
            {r.cover ? (
              <img src={r.cover} alt="" className="size-12 rounded object-cover" />
            ) : (
              <Music2 className="size-12 rounded bg-field p-3 text-fg-muted" aria-hidden="true" />
            )}
            <div className="min-w-0 flex-1">
              <h3 className="text-sm font-medium break-words">{r.title}</h3>
              <p className="text-xs text-fg-muted">
                {artists.find((a) => a.id === r.artistId)?.name} ·{" "}
                {t(`artists.releaseTypes.${r.recordType}`)} · {r.releaseDate}
              </p>
              <p className="mt-1 text-xs text-fg-secondary">
                {t(`artists.states.${r.state}`, { present: r.present, total: r.total })}
              </p>
            </div>
            {r.monitored && r.present < r.total && (
              <Button
                size="sm"
                disabled={busy}
                onClick={() =>
                  void run("download", async () => {
                    const jobs = await api.missingDownload([r.id]);
                    toast({
                      message: t("artists.queued", { count: jobs.length }),
                      tone: "success",
                    });
                  })
                }
              >
                {t("artists.downloadMissing")}
              </Button>
            )}
          </Card>
        </li>
      ))}
    </ul>
  );

  return (
    <div className="space-y-5">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <p className="max-w-prose text-sm text-fg-muted">
          {t(missingOnly ? "artists.missingHint" : "artists.intro")}
        </p>
        <div className="flex gap-2">
          <Button
            variant="primary"
            disabled={busy}
            onClick={startFollow}
            icon={<UserRound className="size-4" />}
          >
            {t("artists.follow")}
          </Button>
          <Button
            disabled={busy || !artists.length}
            loading={action === "check"}
            onClick={() => void run("check", () => api.artistsCheckNow())}
          >
            {t("artists.check")}
          </Button>
        </div>
      </div>
      {action === "check" && (
        <p role="status" className="text-sm text-fg-secondary">
          {t("artists.checking")}
        </p>
      )}
      {loading ? (
        <p role="status" className="text-sm text-fg-muted">
          {t("artists.loading")}
        </p>
      ) : !artists.length ? (
        <EmptyState
          icon={<UserRound />}
          title={t("artists.emptyTitle")}
          description={t("artists.empty")}
        />
      ) : missingOnly ? (
        <>
          <Select
            label={t("artists.filter")}
            value={artistFilter}
            onChange={(e) => setArtistFilter(e.target.value)}
            options={[
              { value: "", label: t("artists.allArtists") },
              ...artists.map((a) => ({ value: a.id, label: a.name })),
            ]}
          />
          {releases.filter((r) => !artistFilter || r.artistId === artistFilter).length ? (
            releaseList(releases.filter((r) => !artistFilter || r.artistId === artistFilter))
          ) : (
            <EmptyState
              icon={<Check />}
              title={t("artists.noMissing")}
              description={t("artists.missingHint")}
            />
          )}
        </>
      ) : (
        artists.map((a) => {
          const items = releases.filter((r) => r.artistId === a.id);
          const show = expanded === a.id;
          return (
            <section key={a.id} aria-label={a.name} className="space-y-3">
              <Card>
                <div className="flex flex-wrap items-center gap-4">
                  {a.picture ? (
                    <img src={a.picture} alt="" className="size-14 rounded-full object-cover" />
                  ) : (
                    <UserRound
                      className="size-14 rounded-full bg-accent-muted p-4 text-accent"
                      aria-hidden="true"
                    />
                  )}
                  <div className="min-w-0 flex-1">
                    <h2 className="font-display text-base font-semibold">{a.name}</h2>
                    <p className="mt-1 text-xs text-fg-secondary">
                      {t("artists.counts", {
                        total: items.length,
                        complete: items.filter((r) => r.state === "complete").length,
                      })}
                    </p>
                    <p className="mt-1 text-xs text-fg-muted">
                      {t("artists.incomplete", {
                        count: items.filter((r) => r.state === "incomplete").length,
                      })}{" "}
                      ·{" "}
                      {t("artists.nextCheck", {
                        date:
                          a.lastCheckAt === null
                            ? t("artists.pendingCheck")
                            : new Date(
                                (a.lastCheckAt +
                                  (settings?.artistCheckIntervalHours ?? 24) * 3600) *
                                  1000,
                              ).toLocaleString(i18n.language),
                      })}
                    </p>
                  </div>
                  <div className="flex flex-wrap gap-2">
                    <Button
                      size="sm"
                      aria-expanded={show}
                      onClick={() => setExpanded(show ? null : a.id)}
                    >
                      {t("artists.viewReleases")}
                    </Button>
                    <Button
                      size="sm"
                      disabled={busy}
                      onClick={() => {
                        setEditing(a);
                        setOptions(a.options);
                        setDialogError(null);
                        setOpen(true);
                      }}
                    >
                      {t("artists.edit")}
                    </Button>
                    <Button
                      size="sm"
                      variant="ghost"
                      disabled={busy}
                      onClick={() => void run("unfollow", () => api.artistUnfollow(a.id, false))}
                    >
                      {t("artists.unfollow")}
                    </Button>
                  </div>
                </div>
              </Card>
              {show &&
                (items.length ? (
                  releaseList(items)
                ) : (
                  <p className="text-sm text-fg-muted">{t("artists.noReleases")}</p>
                ))}
            </section>
          );
        })
      )}
      <Dialog
        open={open}
        onClose={close}
        closeLabel={t("common.close")}
        title={t(editing ? "artists.edit" : "artists.follow")}
        footer={
          <>
            <Button disabled={action === "save"} onClick={close}>
              {t("common.cancel")}
            </Button>
            <Button
              variant="primary"
              loading={action === "save"}
              disabled={busy || searching || (!editing && !choice) || !options.types.length}
              onClick={() => void save()}
            >
              {t("common.save")}
            </Button>
          </>
        }
      >
        <div className="space-y-5">
          {!editing ? (
            <section className="space-y-3">
              <form
                className="flex items-end gap-2"
                onSubmit={(e) => {
                  e.preventDefault();
                  if (name.trim() && !searching && !busy) void search();
                }}
              >
                <div className="min-w-0 flex-1">
                  <Input
                    data-autofocus
                    label={t("artists.search")}
                    placeholder={t("artists.searchPlaceholder")}
                    value={name}
                    disabled={action === "save"}
                    onChange={(e) => {
                      ++searchSerial.current;
                      setSearching(false);
                      setName(e.target.value);
                      setChoice(null);
                      setHits([]);
                      setSearched(false);
                      setDialogError(null);
                    }}
                  />
                </div>
                <Button
                  type="submit"
                  loading={searching}
                  disabled={busy || !name.trim()}
                  icon={<Search className="size-4" />}
                >
                  {t("artists.search")}
                </Button>
              </form>
              {!!hits.length && (
                <div
                  className="max-h-48 space-y-2 overflow-y-auto"
                  role="group"
                  aria-label={t("artists.results")}
                >
                  {hits.map((h) => (
                    <Button
                      key={h.id}
                      disabled={busy}
                      aria-pressed={choice === h.id}
                      className={`h-auto min-h-14 w-full !justify-start !whitespace-normal py-2 text-left ${choice === h.id ? "!border-accent !bg-accent-muted" : ""}`}
                      onClick={() => setChoice(h.id)}
                    >
                      {h.picture ? (
                        <img src={h.picture} alt="" className="size-9 rounded-full object-cover" />
                      ) : (
                        <UserRound
                          className="size-9 shrink-0 rounded-full bg-field p-2"
                          aria-hidden="true"
                        />
                      )}
                      <span className="flex-1">
                        <span className="block font-medium">{h.name}</span>
                        <span className="block text-xs text-fg-muted">
                          {t("artists.fans", { count: h.fans })} ·{" "}
                          {t("artists.providerId", { id: h.id })}
                        </span>
                      </span>
                      {choice === h.id && (
                        <Check className="size-4 text-accent" aria-hidden="true" />
                      )}
                    </Button>
                  ))}
                </div>
              )}
              {searched && !hits.length && !searching && (
                <p role="status" className="text-sm text-fg-muted">
                  {t("artists.noResults")}
                </p>
              )}
              <p
                className={`rounded-md px-3 py-2 text-sm ${choice ? "bg-accent-muted text-fg" : "bg-field text-fg-muted"}`}
              >
                {choice
                  ? t("artists.selected", { name: selected?.name ?? `Deezer #${choice}` })
                  : t("artists.choose")}
              </p>
            </section>
          ) : (
            <p className="rounded-md bg-accent-muted px-3 py-2 font-medium">{editing.name}</p>
          )}
          <fieldset
            disabled={action === "save"}
            className="space-y-4 border-t border-glass-border pt-4"
          >
            <legend className="text-sm font-semibold">{t("artists.monitoring")}</legend>
            <Select
              label={t("artists.existing")}
              value={options.monitorExisting}
              onChange={(e) => setOptions((o) => ({ ...o, monitorExisting: e.target.value }))}
              options={["all", "latest", "none"].map((value) => ({
                value,
                label: t(`artists.existingOptions.${value}`),
              }))}
              hint={t(`artists.existingHints.${options.monitorExisting}`)}
            />
            <Select
              label={t("artists.new")}
              value={options.monitorNew}
              onChange={(e) => setOptions((o) => ({ ...o, monitorNew: e.target.value }))}
              options={["all", "notify", "none"].map((value) => ({
                value,
                label: t(`artists.newOptions.${value}`),
              }))}
              hint={t(`artists.newHints.${options.monitorNew}`)}
            />
            <fieldset>
              <legend className="mb-2 text-xs font-medium text-fg-secondary">
                {t("artists.types")}
              </legend>
              <div className="flex flex-wrap gap-4">
                {RELEASE_TYPES.map((type) => (
                  <Checkbox
                    key={type}
                    checked={options.types.includes(type)}
                    onChange={(on) =>
                      setOptions((o) => ({
                        ...o,
                        types: on ? [...o.types, type] : o.types.filter((x) => x !== type),
                      }))
                    }
                  >
                    {t(`artists.releaseTypes.${type}`)}
                  </Checkbox>
                ))}
              </div>
              {!options.types.length && (
                <p role="alert" className="mt-2 text-xs text-error">
                  {t("artists.needType")}
                </p>
              )}
            </fieldset>
            <Checkbox
              checked={options.excludeVariants}
              onChange={(excludeVariants) => setOptions((o) => ({ ...o, excludeVariants }))}
            >
              {t("artists.excludeVariants")}
            </Checkbox>
          </fieldset>
          <fieldset
            disabled={action === "save"}
            className="space-y-4 border-t border-glass-border pt-4"
          >
            <legend className="text-sm font-semibold">{t("artists.destination")}</legend>
            <Select
              label={t("preview.profile")}
              value={options.profileId}
              onChange={(e) => setOptions((o) => ({ ...o, profileId: e.target.value }))}
              options={PROFILE_OPTIONS.map((p) => ({ value: p.id, label: t(p.labelKey) }))}
            />
            <div className="flex items-start gap-2">
              <div className="min-w-0 flex-1">
                <Input
                  label={t("preview.folder")}
                  value={options.outputDir ?? ""}
                  placeholder={settings?.outputDir ?? ""}
                  hint={t("artists.folderHint")}
                  onChange={(e) => setOptions((o) => ({ ...o, outputDir: e.target.value || null }))}
                />
              </div>
              <Button
                className="mt-5"
                icon={<FolderOpen className="size-4" />}
                onClick={() =>
                  void api
                    .pickFolder()
                    .then((folder) => {
                      if (folder) setOptions((o) => ({ ...o, outputDir: folder }));
                    })
                    .catch((e) => setDialogError(errorText(t, e)))
                }
              >
                {t("artists.chooseFolder")}
              </Button>
            </div>
          </fieldset>
          {dialogError && (
            <p role="alert" className="rounded-md bg-error/10 p-3 text-sm text-error">
              {dialogError}
            </p>
          )}
        </div>
      </Dialog>
    </div>
  );
}
