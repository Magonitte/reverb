import { useCallback, useEffect, useState } from "react";
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
  const { t } = useTranslation();
  const toast = useUiStore((s) => s.pushToast);
  const interval = useSettingsStore((s) => s.settings?.artistCheckIntervalHours ?? 24);
  const [params, setParams] = useSearchParams();
  const [artists, setArtists] = useState<FollowedArtist[]>([]);
  const [releases, setReleases] = useState<ArtistRelease[]>([]);
  const [name, setName] = useState("");
  const [hits, setHits] = useState<ArtistHit[]>([]);
  const [open, setOpen] = useState(false);
  const [choice, setChoice] = useState<string | null>(null);
  const [options, setOptions] = useState(defaults);
  const [busy, setBusy] = useState(false);
  const [editing, setEditing] = useState<FollowedArtist | null>(null);
  const load = useCallback(async () => {
    const list = await api.artistsFollowed();
    setArtists(list);
    setReleases(
      missingOnly
        ? await api.missingList()
        : (await Promise.all(list.map((a) => api.artistReleases(a.id)))).flat(),
    );
  }, [missingOnly]);
  const run = useCallback(
    async (work: () => Promise<unknown>) => {
      setBusy(true);
      try {
        await work();
        await load();
      } catch (e) {
        toast({ message: errorText(t, e), tone: "error" });
      } finally {
        setBusy(false);
      }
    },
    [load, t, toast],
  );
  useEffect(() => {
    let active = true;
    let off = () => {};
    void api
      .artistsFollowed()
      .then(async (list) => {
        const releases = missingOnly
          ? await api.missingList()
          : (await Promise.all(list.map((a) => api.artistReleases(a.id)))).flat();
        if (active) {
          setArtists(list);
          setReleases(releases);
        }
      })
      .catch((e) => toast({ message: errorText(t, e), tone: "error" }));
    void onEvent("artists://updated", () => {
      if (active) void load();
    }).then((fn) => {
      if (active) off = fn;
      else fn();
    });
    return () => {
      active = false;
      off();
    };
  }, [load, missingOnly, t, toast]);
  const followId = params.get("follow");
  const [previousFollow, setPreviousFollow] = useState<string | null>(null);
  if (followId !== previousFollow) {
    setPreviousFollow(followId);
    if (followId) {
      setChoice(followId);
      setOpen(true);
    }
  }
  function close() {
    setOpen(false);
    setEditing(null);
    if (followId) {
      const next = new URLSearchParams(params);
      next.delete("follow");
      setParams(next);
    }
  }
  return (
    <div className="space-y-4">
      <div className="flex gap-2">
        <Button
          onClick={() => {
            setEditing(null);
            setChoice(null);
            setOptions(defaults);
            setOpen(true);
          }}
        >
          {t("artists.follow")}
        </Button>
        <Button disabled={busy} onClick={() => void run(() => api.artistsCheckNow())}>
          {t("artists.check")}
        </Button>
      </div>
      {!artists.length && <p className="text-sm text-fg-muted">{t("artists.empty")}</p>}
      {!missingOnly &&
        artists.map((a) => (
          <Card key={a.id} className="flex flex-wrap items-center gap-4">
            {a.picture && (
              <img src={a.picture} alt="" className="size-12 rounded-full object-cover" />
            )}
            <div className="flex-1">
              <h3>{a.name}</h3>
              <p className="text-xs text-fg-muted">
                {t("artists.counts", {
                  total: releases.filter((r) => r.artistId === a.id).length,
                  complete: releases.filter((r) => r.artistId === a.id && r.state === "complete")
                    .length,
                })}
              </p>
              <p className="text-xs text-fg-muted">
                {t("artists.incomplete", {
                  count: releases.filter((r) => r.artistId === a.id && r.state === "incomplete").length,
                })} · {t("artists.nextCheck", {
                  date: a.lastCheckAt === null
                    ? t("artists.pendingCheck")
                    : new Date((a.lastCheckAt + interval * 3600) * 1000).toLocaleString(),
                })}
              </p>
            </div>
            <Button
              onClick={() => {
                setEditing(a);
                setOptions(a.options);
                setOpen(true);
              }}
            >
              {t("artists.edit")}
            </Button>
            <Button disabled={busy} onClick={() => void run(() => api.artistUnfollow(a.id, false))}>
              {t("artists.unfollow")}
            </Button>
          </Card>
        ))}
      <ul className="space-y-2">
        {releases.map((r) => (
          <li key={`${r.artistId}:${r.id}`}>
            <Card className="flex flex-wrap items-center gap-3">
              {r.cover && <img src={r.cover} alt="" className="size-10 rounded object-cover" />}
              <div className="flex-1">
                <h3 className="text-sm">{r.title}</h3>
                <p className="text-xs text-fg-muted">
                  {t(`artists.states.${r.state}`, { present: r.present, total: r.total })}
                </p>
              </div>
              {r.present < r.total && (
                <Button disabled={busy} onClick={() => void run(() => api.missingDownload([r.id]))}>
                  {t("artists.downloadMissing")}
                </Button>
              )}
            </Card>
          </li>
        ))}
      </ul>
      <Dialog
        open={open}
        onClose={close}
        closeLabel={t("common.close")}
        title={t(editing ? "artists.edit" : "artists.follow")}
      >
        <div className="space-y-3">
          {!editing && (
            <>
              <Input
                label={t("artists.search")}
                value={name}
                onChange={(e) => setName(e.target.value)}
              />
              <Button
                disabled={busy || !name.trim()}
                onClick={() => void run(async () => setHits(await api.artistsSearch(name)))}
              >
                {t("artists.search")}
              </Button>
              {hits.map((h) => (
                <Button key={h.id} aria-pressed={choice === h.id} onClick={() => setChoice(h.id)}>
                  {h.name} · {t("artists.fans", { count: h.fans })}
                </Button>
              ))}
            </>
          )}
          <Select
            label={t("artists.existing")}
            value={options.monitorExisting}
            onChange={(e) => setOptions((o) => ({ ...o, monitorExisting: e.target.value }))}
            options={["all", "latest", "none"].map((value) => ({
              value,
              label: t(`artists.existingOptions.${value}`),
            }))}
          />
          <Select
            label={t("artists.new")}
            value={options.monitorNew}
            onChange={(e) => setOptions((o) => ({ ...o, monitorNew: e.target.value }))}
            options={["all", "notify", "none"].map((value) => ({
              value,
              label: t(`artists.newOptions.${value}`),
            }))}
          />
          <fieldset>
            <legend className="mb-2 text-sm">{t("artists.types")}</legend>
            {RELEASE_TYPES.map((type) => (
              <Checkbox
                key={type}
                label={t(`artists.releaseTypes.${type}`)}
                checked={options.types.includes(type)}
                onChange={(on) =>
                  setOptions((o) => ({
                    ...o,
                    types: on ? [...o.types, type] : o.types.filter((x) => x !== type),
                  }))
                }
              />
            ))}
          </fieldset>
          <Checkbox
            label={t("artists.excludeVariants")}
            checked={options.excludeVariants}
            onChange={(excludeVariants) => setOptions((o) => ({ ...o, excludeVariants }))}
          />
          <Select
            label={t("preview.profile")}
            value={options.profileId}
            onChange={(e) => setOptions((o) => ({ ...o, profileId: e.target.value }))}
            options={PROFILE_OPTIONS.map((p) => ({ value: p.id, label: t(p.labelKey) }))}
          />
          <Input
            label={t("preview.folder")}
            value={options.outputDir ?? ""}
            onChange={(e) => setOptions((o) => ({ ...o, outputDir: e.target.value || null }))}
          />
          <Button
            variant="primary"
            disabled={busy || (!editing && !choice) || !options.types.length}
            onClick={() =>
              void run(async () => {
                if (editing) await api.artistUpdate(editing.id, options);
                else if (choice) {
                  await api.artistFollow(choice, options);
                  await api.artistsCheckNow();
                }
                close();
              })
            }
          >
            {t("common.save")}
          </Button>
        </div>
      </Dialog>
    </div>
  );
}
