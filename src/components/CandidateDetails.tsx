import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Disc3 } from "lucide-react";
import type { Candidate } from "@/bindings/Candidate";
import { Dialog } from "@/components/ui/Dialog";
import { Button } from "@/components/ui/Button";

export function CandidateArtwork({ candidate }: { candidate: Candidate }) {
  const { t } = useTranslation();
  const [failed, setFailed] = useState(false);
  return candidate.coverUrl && !failed ? (
    <img
      src={candidate.coverUrl}
      alt={t("tagEditor.cover")}
      onError={() => setFailed(true)}
      className="aspect-square w-full rounded-lg object-cover"
    />
  ) : (
    <div className="flex aspect-square w-full flex-col items-center justify-center gap-2 rounded-lg bg-field text-fg-muted">
      <Disc3 className="size-10" aria-hidden="true" />
      <span className="text-xs">{t("metadataDetails.noCover")}</span>
    </div>
  );
}

export function CandidateDetails({
  candidate,
  busy,
  onClose,
  onApply,
  saveInEditor = true,
}: {
  candidate: Candidate;
  busy: boolean;
  onClose: () => void;
  onApply: () => void;
  saveInEditor?: boolean;
}) {
  const { t } = useTranslation();
  const fields = [
    [t("metadataDetails.provider"), candidate.provider],
    [t("tagEditor.fields.title"), candidate.title],
    [t("tagEditor.fields.artist"), candidate.artists.join(", ")],
    [t("tagEditor.fields.album"), candidate.album],
    [t("tagEditor.fields.albumArtist"), candidate.albumArtist],
    [t("tagEditor.fields.year"), candidate.year],
    [t("tagEditor.fields.genre"), candidate.genre],
    [t("tagEditor.fields.trackNo"), candidate.trackNo],
    [t("tagEditor.fields.trackTotal"), candidate.trackTotal],
    [t("tagEditor.fields.discNo"), candidate.discNo],
    [
      t("metadataDetails.duration"),
      candidate.durationS == null
        ? null
        : t("review.duration", { seconds: Math.round(candidate.durationS) }),
    ],
    [t("metadataDetails.isrc"), candidate.isrc],
    [t("metadataDetails.providerId"), candidate.providerId],
    [t("metadataDetails.recordingId"), candidate.mbRecordingId],
  ];
  return (
    <Dialog
      open
      title={t("metadataDetails.title")}
      closeLabel={t("common.close")}
      onClose={onClose}
      footer={
        <Button loading={busy} onClick={onApply}>
          {t(saveInEditor ? "tagEditor.apply" : "review.apply")}
        </Button>
      }
    >
      <div className="mx-auto mb-5 w-48">
        <CandidateArtwork
          key={`${candidate.provider}:${candidate.providerId}`}
          candidate={candidate}
        />
      </div>
      <dl className="grid grid-cols-[minmax(0,1fr)_minmax(0,2fr)] gap-x-4 gap-y-3 text-sm">
        {fields.map(([label, value]) => (
          <div key={String(label)} className="contents">
            <dt className="text-fg-muted">{label}</dt>
            <dd className="break-words">{value || t("metadataDetails.unavailable")}</dd>
          </div>
        ))}
      </dl>
      {saveInEditor && (
        <p className="mt-5 text-xs text-fg-muted">{t("metadataDetails.saveHint")}</p>
      )}
    </Dialog>
  );
}
