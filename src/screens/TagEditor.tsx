import { useEffect, useState } from "react";
import { Tags } from "lucide-react";
import { useSearchParams } from "react-router";
import { useTranslation } from "react-i18next";
import type { Candidate } from "@/bindings/Candidate";
import type { TrackTags } from "@/bindings/TrackTags";
import { ScreenHeader } from "@/components/ScreenHeader";
import { Button } from "@/components/ui/Button";
import { Input, Textarea } from "@/components/ui/Input";
import { Checkbox } from "@/components/ui/Checkbox";
import { EmptyState } from "@/components/ui/EmptyState";
import { api } from "@/lib/ipc/api";
import { errorText } from "@/lib/errors";
import { useUiStore } from "@/stores/ui";
import { useSettingsStore } from "@/stores/settings";

const TEXT_FIELDS = ["title", "artist", "album", "albumArtist", "genre"] as const;
const NUMBER_FIELDS = ["year", "trackNo", "trackTotal", "discNo"] as const;

function coverUrl(tags: TrackTags): string | undefined {
  if (!tags.cover) return undefined;
  const bytes = tags.cover.data;
  let binary = "";
  for (let offset = 0; offset < bytes.length; offset += 4096)
    binary += String.fromCharCode(...bytes.slice(offset, offset + 4096));
  return `data:${tags.cover.mimeType};base64,${btoa(binary)}`;
}

export default function TagEditor() {
  const { t } = useTranslation();
  const [params, setParams] = useSearchParams();
  const path = params.get("path") ?? "";
  const [loaded, setLoaded] = useState<{ path: string; tags: TrackTags } | null>(null);
  const [error, setError] = useState<unknown>(null);
  const [busy, setBusy] = useState(false);
  const [url, setUrl] = useState("");
  const [reorganize, setReorganize] = useState(false);
  const [candidates, setCandidates] = useState<Candidate[]>([]);
  const pushToast = useUiStore((s) => s.pushToast);
  const autoOrganize = useSettingsStore((s) => s.settings?.autoOrganize ?? false);
  useEffect(() => {
    let active = true;
    if (path)
      api
        .tagsRead(path)
        .then((tags) => {
          if (active) {
            setLoaded({ path, tags });
            setError(null);
          }
        })
        .catch((e) => {
          if (active) setError(e);
        });
    return () => {
      active = false;
    };
  }, [path]);
  const tags = loaded?.path === path ? loaded.tags : null;
  const patch = (changes: Partial<TrackTags>) => {
    if (tags) setLoaded({ path, tags: { ...tags, ...changes } });
  };
  const run = async (action: () => Promise<unknown>) => {
    setBusy(true);
    try {
      await action();
    } catch (e) {
      pushToast({ message: errorText(t, e), tone: "error" });
    } finally {
      setBusy(false);
    }
  };
  const open = async () => {
    const file = await api.pickAudioFile();
    if (file) {
      setCandidates([]);
      setParams({ path: file });
    }
  };
  const save = async () => {
    if (!tags) return;
    const reviewId = params.get("reviewId");
    const next = reviewId
      ? (await api.reviewApply(Number(reviewId), null, tags)).filePath
      : await api.tagsWrite(path, tags, reorganize);
    setLoaded({ path: next, tags });
    setParams({ path: next });
    pushToast({ message: t("tagEditor.saved"), tone: "success" });
  };
  const apply = async (candidate: Candidate) =>
    patch({
      title: candidate.title,
      artist: candidate.artists.length ? candidate.artists.join(", ") : tags?.artist,
      album: candidate.album ?? tags?.album,
      albumArtist: candidate.albumArtist ?? tags?.albumArtist,
      year: candidate.year ?? tags?.year,
      genre: candidate.genre ?? tags?.genre,
      trackNo: candidate.trackNo ?? tags?.trackNo,
      trackTotal: candidate.trackTotal ?? tags?.trackTotal,
      discNo: candidate.discNo ?? tags?.discNo,
      isrc: candidate.isrc ?? tags?.isrc,
      cover: candidate.coverUrl ? await api.artworkFetch(candidate.coverUrl) : tags?.cover,
    });
  const image = tags ? coverUrl(tags) : undefined;
  return (
    <>
      <ScreenHeader
        title={t("tagEditor.title")}
        subtitle={t("tagEditor.subtitle")}
        actions={
          <Button disabled={busy} onClick={() => void run(open)}>
            {t("tagEditor.open")}
          </Button>
        }
      />
      {error != null && (
        <p role="alert" className="mb-4 text-error">
          {errorText(t, error)}
        </p>
      )}
      {!path ? (
        <EmptyState
          icon={<Tags />}
          title={t("tagEditor.empty.title")}
          description={t("tagEditor.empty.description")}
        />
      ) : !tags && !error ? (
        <p role="status">{t("common.loading")}</p>
      ) : tags ? (
        <div className="flex flex-col gap-4">
          <p className="break-all text-xs text-fg-muted">{path}</p>
          <div className="grid gap-4 md:grid-cols-2">
            {TEXT_FIELDS.map((field) => (
              <Input
                key={field}
                label={t(`tagEditor.fields.${field}`)}
                value={tags[field] ?? ""}
                disabled={busy}
                onChange={(e) =>
                  patch({ [field]: field === "title" ? e.target.value : e.target.value || null })
                }
              />
            ))}
            {NUMBER_FIELDS.map((field) => (
              <Input
                key={field}
                type="number"
                min={1}
                label={t(`tagEditor.fields.${field}`)}
                value={tags[field] ?? ""}
                disabled={busy}
                onChange={(e) => patch({ [field]: e.target.value ? Number(e.target.value) : null })}
              />
            ))}
          </div>
          <label className="flex flex-col gap-2 text-xs text-fg-secondary">
            {t("tagEditor.fields.lyrics")}
            <Textarea
              value={tags.lyrics ?? ""}
              disabled={busy}
              onChange={(e) => patch({ lyrics: e.target.value || null })}
            />
          </label>
          <p className="text-xs text-fg-muted">
            {t(/^\[\d+:\d+/m.test(tags.lyrics ?? "") ? "tagEditor.synced" : "tagEditor.plain")}
          </p>
          <div className="glass flex flex-wrap items-center gap-3 rounded-lg p-4">
            {image && (
              <img
                src={image}
                alt={t("tagEditor.cover")}
                className="size-24 rounded-md object-cover"
              />
            )}
            <Button
              disabled={busy}
              onClick={() =>
                void run(async () => {
                  const file = await api.pickImageFile();
                  if (file) patch({ cover: await api.artworkRead(file) });
                })
              }
            >
              {t("tagEditor.pickCover")}
            </Button>
            <Input
              label={t("tagEditor.coverUrl")}
              type="url"
              value={url}
              onChange={(e) => setUrl(e.target.value)}
              disabled={busy}
            />
            <Button
              disabled={busy || !url}
              onClick={() => void run(async () => patch({ cover: await api.artworkFetch(url) }))}
            >
              {t("tagEditor.fetchCover")}
            </Button>
            <Button
              disabled={busy || !tags.cover}
              variant="ghost"
              onClick={() => patch({ cover: null })}
            >
              {t("tagEditor.removeCover")}
            </Button>
          </div>
          <Button
            disabled={busy}
            onClick={() =>
              void run(async () =>
                setCandidates(
                  await api.metadataSearch([tags.artist, tags.title].filter(Boolean).join(" ")),
                ),
              )
            }
          >
            {t("tagEditor.search")}
          </Button>
          {candidates.length > 0 && (
            <ul className="flex flex-col gap-2">
              {candidates.map((candidate) => (
                <li
                  key={`${candidate.provider}:${candidate.providerId}`}
                  className="glass flex flex-wrap items-center justify-between gap-3 rounded-md p-3"
                >
                  <span>
                    {candidate.title} · {candidate.artists.join(", ")} · {candidate.album}
                  </span>
                  <Button disabled={busy} onClick={() => void run(() => apply(candidate))}>
                    {t("tagEditor.apply")}
                  </Button>
                </li>
              ))}
            </ul>
          )}
          {autoOrganize && (
            <Checkbox checked={reorganize} onChange={setReorganize} disabled={busy}>
              {t("tagEditor.reorganize")}
            </Checkbox>
          )}
          <Button loading={busy} disabled={!tags.title.trim()} onClick={() => void run(save)}>
            {t("common.save")}
          </Button>
        </div>
      ) : null}
    </>
  );
}
