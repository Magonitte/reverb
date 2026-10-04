import { useEffect, useState } from "react";
import { ListChecks } from "lucide-react";
import { useNavigate } from "react-router";
import { useTranslation } from "react-i18next";
import { ScreenHeader } from "@/components/ScreenHeader";
import { Button } from "@/components/ui/Button";
import { EmptyState } from "@/components/ui/EmptyState";
import { LibraryCover } from "@/components/LibraryCover";
import { CandidateDetails } from "@/components/CandidateDetails";
import type { Candidate } from "@/bindings/Candidate";
import { api } from "@/lib/ipc/api";
import { errorText } from "@/lib/errors";
import { useLibraryStore } from "@/stores/library";
import { useUiStore } from "@/stores/ui";

export default function Review() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const { items, total, loading, error, load, setQuery } = useLibraryStore();
  const [index, setIndex] = useState(0);
  const [choice, setChoice] = useState(0);
  const [busy, setBusy] = useState(false);
  const [details, setDetails] = useState<Candidate | null>(null);
  const pushToast = useUiStore((s) => s.pushToast);
  useEffect(() => {
    void setQuery({
      needsReview: true,
      text: null,
      artist: null,
      album: null,
      format: null,
      missing: null,
      dateRange: "all",
      offset: 0,
    });
  }, [setQuery]);
  const activeIndex = Math.min(index, Math.max(0, items.length - 1));
  const item = items[activeIndex];
  const candidates = item?.reviewCandidates?.slice(0, 5) ?? [];
  const run = async (action: () => Promise<unknown>) => {
    if (busy) return;
    setBusy(true);
    try {
      await action();
      await load();
      setChoice(0);
    } catch (e) {
      pushToast({ message: errorText(t, e), tone: "error" });
    } finally {
      setBusy(false);
    }
  };
  const apply = () => {
    if (item && candidates[choice])
      void run(() => api.reviewApply(item.id, candidates[choice].candidate));
  };
  const dismiss = () => {
    if (item) void run(() => api.reviewDismiss(item.id));
  };
  const manual = () => {
    if (item) navigate(`/tag-editor?path=${encodeURIComponent(item.filePath)}&reviewId=${item.id}`);
  };
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      const target = event.target as HTMLElement;
      if (
        busy ||
        details ||
        loading ||
        event.ctrlKey ||
        event.metaKey ||
        event.altKey ||
        target.closest(
          "input:not([type=radio]):not([type=checkbox]),textarea,select,[contenteditable=true]",
        )
      )
        return;
      const key = event.key.toLowerCase();
      if (key === "j" || key === "k") {
        event.preventDefault();
        setIndex(Math.max(0, Math.min(items.length - 1, activeIndex + (key === "j" ? 1 : -1))));
        setChoice(0);
      } else if (/^[1-5]$/.test(key) && Number(key) <= candidates.length) {
        event.preventDefault();
        setChoice(Number(key) - 1);
      } else if (key === "enter" && target.tagName !== "BUTTON") {
        event.preventDefault();
        apply();
      } else if (key === "m") {
        event.preventDefault();
        dismiss();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  });
  return (
    <>
      <ScreenHeader title={t("review.title")} subtitle={t("review.subtitle")} />
      {error != null && (
        <p role="alert">
          {errorText(t, error)} <Button onClick={() => void load()}>{t("common.retry")}</Button>
        </p>
      )}
      {item ? (
        <div className="flex flex-col gap-4">
          <p className="text-xs text-fg-muted">{t("review.shortcuts")}</p>
          <div className="glass flex items-center gap-4 rounded-lg p-4">
            <LibraryCover id={item.id} updatedAt={item.updatedAt} missing={item.missing} />
            <div className="min-w-0">
              <p className="text-xs text-fg-muted">{t("review.current")}</p>
              <h2 className="truncate text-base font-semibold">{item.title}</h2>
              <p className="truncate text-fg-muted">
                {item.artist} · {item.album}
              </p>
              <p className="text-xs text-fg-muted">
                {item.year} ·{" "}
                {item.durationS != null
                  ? t("review.duration", { seconds: Math.round(item.durationS) })
                  : ""}
              </p>
              <p className="break-all text-xs text-fg-muted">{item.filePath}</p>
            </div>
          </div>
          <div
            role="radiogroup"
            aria-label={t("review.candidates")}
            className="grid gap-3 md:grid-cols-2"
          >
            {candidates.map(({ candidate, score }, i) => (
              <div
                key={`${candidate.provider}:${candidate.providerId}`}
                className="glass flex gap-3 rounded-lg p-4"
              >
                <input
                  type="radio"
                  name="candidate"
                  checked={choice === i}
                  onChange={() => setChoice(i)}
                  disabled={busy}
                  aria-label={t("review.choose", { number: i + 1, title: candidate.title })}
                />
                <span className="min-w-0">
                  {candidate.coverUrl && (
                    <img
                      src={candidate.coverUrl}
                      alt=""
                      className="mb-2 size-16 rounded-md object-cover"
                    />
                  )}
                  <span className="block font-medium">{candidate.title}</span>
                  <span className="block text-sm text-fg-muted">
                    {candidate.artists.join(", ")} · {candidate.album}
                  </span>
                  <span className="block text-xs text-fg-muted">
                    {candidate.provider} · {Math.round(score * 100)}%
                  </span>
                  <span className="block text-xs text-fg-muted">
                    {candidate.year} ·{" "}
                    {candidate.durationS != null
                      ? t("review.duration", { seconds: Math.round(candidate.durationS) })
                      : ""}
                  </span>
                </span>
                <Button variant="ghost" disabled={busy} onClick={() => setDetails(candidate)}>
                  {t("metadataDetails.view")}
                </Button>
              </div>
            ))}
          </div>
          {details && (
            <CandidateDetails
              saveInEditor={false}
              candidate={details}
              busy={busy}
              onClose={() => setDetails(null)}
              onApply={() =>
                void run(async () => {
                  await api.reviewApply(item.id, details);
                  setDetails(null);
                })
              }
            />
          )}
          {!candidates.length && <p>{t("review.noCandidates")}</p>}
          {item.missing && (
            <p role="alert" className="text-warning">
              {t("library.missing")}
            </p>
          )}
          <div className="flex flex-wrap gap-2">
            <Button disabled={busy || item.missing || !candidates[choice]} onClick={apply}>
              {t("review.apply")}
            </Button>
            <Button variant="ghost" disabled={busy} onClick={dismiss}>
              {t("review.keep")}
            </Button>
            <Button variant="ghost" disabled={busy || item.missing} onClick={manual}>
              {t("review.manual")}
            </Button>
          </div>
          <div className="flex flex-wrap items-center gap-2">
            <Button
              variant="ghost"
              disabled={activeIndex === 0 || busy}
              onClick={() => {
                setIndex(activeIndex - 1);
                setChoice(0);
              }}
            >
              {t("library.previous")}
            </Button>
            <p role="status">{t("review.position", { index: activeIndex + 1, total })}</p>
            <Button
              variant="ghost"
              disabled={busy || activeIndex + 1 >= items.length}
              onClick={() => {
                setIndex(activeIndex + 1);
                setChoice(0);
              }}
            >
              {t("library.next")}
            </Button>
            {total > items.length && (
              <Button
                disabled={busy}
                onClick={() => {
                  setIndex(0);
                  setChoice(0);
                  void setQuery({
                    offset: (useLibraryStore.getState().query.offset ?? 0) + items.length,
                  });
                }}
              >
                {t("review.nextPage")}
              </Button>
            )}
          </div>
        </div>
      ) : loading ? (
        <p role="status">{t("common.loading")}</p>
      ) : (
        <EmptyState
          icon={<ListChecks />}
          title={t("review.empty.title")}
          description={t("review.empty.description")}
        />
      )}
    </>
  );
}
