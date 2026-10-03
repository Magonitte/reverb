import { TrimDialog } from "@/components/TrimDialog";
import { CandidateArtwork, CandidateDetails } from "@/components/CandidateDetails";
import { useEffect, useState } from "react";
import { Tags } from "lucide-react";
import { useNavigate, useSearchParams } from "react-router";
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
  const navigate = useNavigate();
  const [params, setParams] = useSearchParams();
  const path = params.get("path") ?? "";
  const [loaded, setLoaded] = useState<{ path: string; tags: TrackTags } | null>(null);
  const [readError, setReadError] = useState<{ path: string; cause: unknown } | null>(null);
  const [readAttempt, setReadAttempt] = useState(0);
  const error = readError?.path === path ? readError.cause : null;
  const [trimDuration, setTrimDuration] = useState<number | null>(null);
  const [busy, setBusy] = useState(false);
  const [url, setUrl] = useState("");
  const [reorganize, setReorganize] = useState(false);
  const [candidates, setCandidates] = useState<Candidate[]>([]);
  const [searchState, setSearchState] = useState<{ path: string; done: boolean; error: unknown }>({
    path: "",
    done: false,
    error: null,
  });
  const [details, setDetails] = useState<Candidate | null>(null);
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
            setReadError(null);
          }
        })
        .catch((e) => {
          if (active) setReadError({ path, cause: e });
        });
    return () => {
      active = false;
    };
  }, [path, readAttempt]);
  const tags = loaded?.path === path ? loaded.tags : null;
  const patch = (changes: Partial<TrackTags>) => {
    if (tags) setLoaded({ path, tags: { ...tags, ...changes } });
  };
  const run = async (action: () => Promise<unknown>) => {
    if (busy) return;
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
      setDetails(null);
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
  const apply = async (candidate: Candidate) => {
    // Metadata remains useful when the provider's artwork is unavailable.
    let cover = tags?.cover;
    if (candidate.coverUrl) {
      try {
        cover = await api.artworkFetch(candidate.coverUrl);
      } catch (e) {
        pushToast({ message: errorText(t, e), tone: "error" });
      }
    }
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
      cover,
    });
    setDetails(null);
    pushToast({ message: t("metadataDetails.applied"), tone: "success" });
  };
  const search = async () => {
    setCandidates([]);
    setDetails(null);
    setSearchState({ path, done: false, error: null });
    try {
      setCandidates(
        await api.metadataSearch([tags?.artist, tags?.title].filter(Boolean).join(" - ")),
      );
      setSearchState({ path, done: true, error: null });
    } catch (error) {
      setSearchState({ path, done: true, error });
    }
  };
  const image = tags ? coverUrl(tags) : undefined;
  return (
    <>
      <ScreenHeader
        title={t("tagEditor.title")}
        subtitle={t("tagEditor.subtitle")}
        actions={
          <>
            <Button variant="ghost" disabled={busy} onClick={() => navigate("/library")}>
              {t("library.back")}
            </Button>
            <Button
              disabled={busy || !path}
              onClick={() => void run(async () => setTrimDuration(await api.audioDuration(path)))}
            >
              {t("quality.trim")}
            </Button>
            <Button disabled={busy} onClick={() => void run(open)}>
              {t("tagEditor.open")}
            </Button>
          </>
        }
      />
      {trimDuration !== null && (
        <TrimDialog
          path={path}
          duration={trimDuration}
          onClose={() => setTrimDuration(null)}
          onDone={() => void api.tagsRead(path).then((tags) => setLoaded({ path, tags }))}
        />
      )}
      {error != null && (
        <p role="alert" className="mb-4 text-error">
          {errorText(t, error)}
          <Button variant="ghost" onClick={() => setReadAttempt((n) => n + 1)}>
            {t("common.retry")}
          </Button>
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
          <Input
            label={t("metadataDetails.isrc")}
            value={tags.isrc ?? ""}
            disabled={busy}
            onChange={(e) => patch({ isrc: e.target.value || null })}
          />
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
          <section
            className="glass space-y-4 rounded-lg p-5"
            aria-label={t("metadataDetails.results")}
          >
            <div>
              <h2 className="font-semibold">{t("metadataDetails.results")}</h2>
              <p className="mt-1 text-sm text-fg-muted">{t("metadataDetails.hint")}</p>
            </div>
            <Button
              disabled={busy}
              loading={busy && searchState.path === path && !searchState.done}
              onClick={() => void run(search)}
            >
              {t("tagEditor.search")}
            </Button>
            {searchState.path === path && searchState.error != null && (
              <p role="alert" className="text-error">
                {errorText(t, searchState.error)}
              </p>
            )}
            {searchState.path === path && searchState.done && !searchState.error && (
              <p role="status" className="text-sm text-fg-muted">
                {t(candidates.length ? "metadataDetails.count" : "metadataDetails.empty", {
                  count: candidates.length,
                })}
              </p>
            )}
            {searchState.path === path && candidates.length > 0 && (
              <ul className="grid gap-3 lg:grid-cols-2">
                {candidates.map((candidate) => (
                  <li
                    key={`${candidate.provider}:${candidate.providerId}`}
                    className="glass flex flex-wrap items-center justify-between gap-3 rounded-md p-3"
                  >
                    <div className="flex min-w-0 items-center gap-3">
                      <div className="w-16 shrink-0">
                        <CandidateArtwork candidate={candidate} />
                      </div>
                      <div className="min-w-0">
                        <p className="font-medium">{candidate.title}</p>
                        <p className="text-sm text-fg-muted">
                          {candidate.artists.join(", ")} · {candidate.album}
                        </p>
                        <p className="text-xs text-fg-muted">
                          {candidate.provider} ·{" "}
                          {candidate.year ?? t("metadataDetails.unavailable")}
                        </p>
                      </div>
                    </div>
                    <div className="flex flex-wrap gap-2">
                      <Button disabled={busy} variant="ghost" onClick={() => setDetails(candidate)}>
                        {t("metadataDetails.view")}
                      </Button>
                      <Button disabled={busy} onClick={() => void run(() => apply(candidate))}>
                        {t("tagEditor.apply")}
                      </Button>
                    </div>
                  </li>
                ))}
              </ul>
            )}
          </section>
          {details && (
            <CandidateDetails
              candidate={details}
              busy={busy}
              onClose={() => setDetails(null)}
              onApply={() => void run(() => apply(details))}
            />
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
