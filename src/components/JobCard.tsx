import type { ComponentProps, HTMLAttributes, ReactNode, Ref } from "react";
import { FolderOpen, GripVertical, RotateCcw, Trash2, X } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { Job } from "@/bindings/Job";
import { toneOf } from "@/components/JobMiniCard";
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
  /** Alça de arrastar (só pendentes); vem do dnd-kit. */
  handle?: ReactNode;
  rootRef?: Ref<HTMLLIElement>;
  rootProps?: HTMLAttributes<HTMLLIElement>;
}

/** Item completo da Atividade (design §3.4): estágio, progresso, velocidade/ETA, tentativas, erro e ações. */
export function JobCard({
  job,
  maxAttempts,
  onCancel,
  onRetry,
  onRemove,
  onReveal,
  handle,
  rootRef,
  rootProps,
}: JobCardProps) {
  const { t } = useTranslation();
  const running = job.status === "running";
  const finished = job.status === "done" || job.status === "failed" || job.status === "cancelled";
  const title = job.title ?? job.sourceUrl;
  const speed = formatSpeed(job.speedBps);
  const eta = job.etaS !== null ? formatDuration(job.etaS) : "";
  const stats = [speed, eta && t("activity.eta", { eta })].filter(Boolean).join(" · ");
  const label = (key: string) => t(key, { title });

  return (
    <li
      ref={rootRef}
      data-testid="job-card"
      data-status={job.status}
      className="glass flex items-center gap-3 rounded-lg p-3 transition-colors duration-[140ms] hover:bg-glass-hover"
      {...rootProps}
    >
      {handle}
      {job.thumbnail ? (
        <img
          src={job.thumbnail}
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
      </div>
      <div className="flex shrink-0 items-center gap-1">
        {(job.status === "failed" || job.status === "cancelled") && (
          <IconButton size="sm" label={label("activity.retryJob")} onClick={() => onRetry(job)}>
            <RotateCcw />
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
