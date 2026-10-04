import type { ComponentProps, HTMLAttributes, ReactNode, Ref } from "react";
import { useEffect, useState } from "react";
import { FileAudio, FolderOpen, GripVertical, RotateCcw, Trash2, X } from "lucide-react";
import { api } from "@/lib/ipc/api";
import { useTranslation } from "react-i18next";
import type { Job } from "@/bindings/Job";
import { toneOf } from "@/components/JobMiniCard";
import { Badge, type BadgeTone } from "@/components/ui/Badge";
import { IconButton } from "@/components/ui/IconButton";
import { ProgressBar } from "@/components/ui/ProgressBar";
import { VinylDisc } from "@/components/ui/VinylDisc";
import { jobErrorText } from "@/lib/errors";
import { formatDuration, formatSpeed } from "@/lib/format";

export interface JobCardProps {
  job: Job;
  maxAttempts: number;
  onCancel: (job: Job) => void;
  onRetry: (job: Job) => void;
  onRemove: (job: Job) => void;
  onReveal: (job: Job) => void;
  onOpen?: (job: Job) => void;
  /** Alça de arrastar (só pendentes); vem do dnd-kit. */
  handle?: ReactNode;
  rootRef?: Ref<HTMLLIElement>;
  rootProps?: HTMLAttributes<HTMLLIElement>;
}

const CONFIDENCE_TONE: Record<"auto" | "review" | "none", BadgeTone> = {
  auto: "success",
  review: "warning",
  none: "neutral",
};

/** Item completo da Atividade (design §3.4): estágio, progresso, velocidade/ETA, tentativas, erro e ações. */
export function JobCard({
  job,
  maxAttempts,
  onCancel,
  onRetry,
  onRemove,
  onReveal,
  onOpen,
  handle,
  rootRef,
  rootProps,
}: JobCardProps) {
  const { t } = useTranslation();
  const [embedded, setEmbedded] = useState<{ id: number; cover: string | null } | null>(null);
  useEffect(() => {
    let active = true;
    if (job.status === "done" && job.libraryId !== null) {
      const id = job.libraryId;
      api
        .libraryCover(id)
        .then((cover) => {
          if (active) setEmbedded({ id, cover });
        })
        .catch(() => {});
    }
    return () => {
      active = false;
    };
  }, [job.status, job.libraryId]);
  const cover = embedded?.id === job.libraryId ? embedded?.cover : null;
  const running = job.status === "running";
  const finished = job.status === "done" || job.status === "failed" || job.status === "cancelled";
  const title = job.title ?? job.sourceUrl;
  const speed = formatSpeed(job.speedBps);
  const eta = job.etaS !== null ? formatDuration(job.etaS) : "";
  const stats = [speed, eta && t("activity.eta", { eta })].filter(Boolean).join(" · ");
  const label = (key: string) => t(key, { title });
  const result = job.metadataResult;
  // Selo de confiança só nos concluídos em que a identificação decidiu algo (auto/revisão).
  const confidence =
    job.status === "done" && job.confidence !== null && result && result.bucket !== "none" ? (
      <Badge
        tone={CONFIDENCE_TONE[result.bucket]}
        title={t("activity.confidenceHint", {
          source: t(`preview.metadata.source.${result.source}`, { defaultValue: result.source }),
        })}
        data-testid="job-confidence"
        className="mr-1 shrink-0"
      >
        {t(result.bucket === "review" ? "activity.confidenceReview" : "activity.confidence", {
          percent: Math.round(job.confidence * 100),
        })}
      </Badge>
    ) : null;

  return (
    <li
      ref={rootRef}
      data-testid="job-card"
      data-status={job.status}
      className="glass flex items-center gap-3 rounded-lg p-3 transition-colors duration-[140ms] hover:bg-glass-hover"
      {...rootProps}
    >
      {handle}
      {running ? (
        <VinylDisc spinning size={40} />
      ) : cover || job.thumbnail ? (
        <img
          src={cover ?? job.thumbnail ?? undefined}
          data-testid={cover ? "job-cover" : undefined}
          alt=""
          className="size-10 shrink-0 rounded-md bg-field object-cover"
        />
      ) : (
        <VinylDisc spinning={running} size={40} />
      )}
      <div className="min-w-0 flex-1">
        <p className="truncate text-[13px] font-medium text-fg">{title}</p>
        <p className="truncate text-xs text-fg-muted">
          {[
            job.artist,
            job.status === "failed" || job.status === "cancelled"
              ? t(`activity.status.${job.status}`)
              : t(`stages.${job.stage}`),
            job.attempts > 1 && t("activity.attempt", { attempt: job.attempts, max: maxAttempts }),
            stats,
          ]
            .filter(Boolean)
            .join(" · ")}
        </p>
        {job.status !== "failed" && job.status !== "cancelled" && (
          <ProgressBar
            className="mt-2"
            value={running || job.status === "done" ? job.overallProgress : 0}
            tone={toneOf(job)}
            label={label("activity.progressLabel")}
          />
        )}
        {job.status === "failed" && (
          <p role="alert" className="mt-1 text-xs text-error" data-testid="job-error">
            {jobErrorText(t, job)}
          </p>
        )}
        {job.warnings.length > 0 && (
          <ul data-testid="job-warnings" className="mt-1 space-y-1 text-xs text-warning">
            {job.warnings.map((warning) => (
              <li key={warning}>{t(warning, { defaultValue: t("warnings.other") })}</li>
            ))}
          </ul>
        )}
      </div>
      <div className="flex shrink-0 items-center gap-1">
        {confidence}
        {(job.status === "failed" || job.status === "cancelled") && (
          <IconButton size="sm" label={label("activity.retryJob")} onClick={() => onRetry(job)}>
            <RotateCcw />
          </IconButton>
        )}
        {job.status === "done" && job.outputPath && (
          <IconButton size="sm" label={label("activity.openFile")} onClick={() => onOpen?.(job)}>
            <FileAudio />
          </IconButton>
        )}
        {job.status === "done" && job.outputPath && (
          <IconButton size="sm" label={label("activity.openFolder")} onClick={() => onReveal(job)}>
            <FolderOpen />
          </IconButton>
        )}
        {!finished && (
          <IconButton size="sm" label={label("activity.cancelJob")} onClick={() => onCancel(job)}>
            <X />
          </IconButton>
        )}
        {finished && (
          <IconButton size="sm" label={label("activity.removeJob")} onClick={() => onRemove(job)}>
            <Trash2 />
          </IconButton>
        )}
      </div>
    </li>
  );
}

/** Alça de arrastar (a ligação com o dnd-kit é feita por quem a usa). */
export function DragHandle(props: ComponentProps<"button"> & { label: string }) {
  const { label, ...rest } = props;
  return (
    <button
      type="button"
      aria-label={label}
      className="flex size-7 shrink-0 cursor-grab touch-none items-center justify-center rounded-md text-fg-dim hover:bg-hover hover:text-fg active:cursor-grabbing"
      {...rest}
    >
      <GripVertical className="size-4" aria-hidden="true" />
    </button>
  );
}
