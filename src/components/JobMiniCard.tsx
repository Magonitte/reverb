import { useTranslation } from "react-i18next";
import type { Job } from "@/bindings/Job";
import type { JobStage } from "@/bindings/JobStage";
import { ProgressBar, type ProgressTone } from "@/components/ui/ProgressBar";
import { VinylDisc } from "@/components/ui/VinylDisc";

const STAGE_TONE: Partial<Record<JobStage, ProgressTone>> = {
  converting: "convert",
  done: "done",
};

/** Cor da barra por estágio (referência: download âmbar, conversão amarela, concluído verde). */
export function toneOf(job: Job): ProgressTone {
  if (job.status === "failed") return "error";
  if (job.status === "done") return "done";
  return STAGE_TONE[job.stage] ?? "download";
}

/** Card compacto de um job: disco de vinil (gira se está rodando), título, estágio e progresso. */
export function JobMiniCard({ job }: { job: Job }) {
  const { t } = useTranslation();
  const running = job.status === "running";
  const title = job.title ?? job.sourceUrl;
  return (
    <li
      data-testid="job-card"
      className="glass flex items-center gap-3 rounded-lg p-3 transition-colors duration-[140ms] hover:bg-glass-hover"
    >
      <VinylDisc spinning={running} size={40} />
      <div className="min-w-0 flex-1">
        <p className="truncate text-[13px] font-medium text-fg">{title}</p>
        <p className="truncate text-xs text-fg-muted">
          {job.artist ? `${job.artist} · ` : ""}
          {t(`stages.${job.stage}`)}
        </p>
        <ProgressBar
          className="mt-2"
          value={running || job.status === "done" ? job.overallProgress : 0}
          tone={toneOf(job)}
          label={t("activity.progressLabel", { title })}
        />
      </div>
    </li>
  );
}
