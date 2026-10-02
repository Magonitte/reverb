import { useMemo, type ReactNode } from "react";
import { ClipboardPaste, FolderOpen, House, ListMusic, Tags } from "lucide-react";
import { Link, useNavigate } from "react-router";
import { useTranslation } from "react-i18next";
import type { Job } from "@/bindings/Job";
import { CommandBar } from "@/components/command-bar/CommandBar";
import { JobMiniCard } from "@/components/JobMiniCard";
import { ScreenHeader } from "@/components/ScreenHeader";
import { EmptyState } from "@/components/ui/EmptyState";
import { VinylDisc } from "@/components/ui/VinylDisc";
import { errorText } from "@/lib/errors";
import { api } from "@/lib/ipc/api";
import { sortedJobs, useJobsStore } from "@/stores/jobs";
import { useUiStore } from "@/stores/ui";

const RECENT_COUNT = 8;

function QuickAction({
  icon,
  label,
  onClick,
}: {
  icon: ReactNode;
  label: string;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className="glass flex items-center gap-3 rounded-lg p-4 text-left text-[13px] font-medium text-fg transition-colors duration-[140ms] hover:bg-glass-hover [&_svg]:size-5 [&_svg]:text-accent"
    >
      {icon}
      {label}
    </button>
  );
}

function RecentCard({ job, onOpen }: { job: Job; onOpen: (job: Job) => void }) {
  const { t } = useTranslation();
  const title = job.title ?? job.sourceUrl;
  return (
    <li data-testid="recent-item">
      <button
        type="button"
        onClick={() => onOpen(job)}
        aria-label={t("home.revealRecent", { title })}
        className="glass flex w-full items-center gap-3 rounded-lg p-2 text-left transition-colors duration-[140ms] hover:bg-glass-hover"
      >
        {job.thumbnail ? (
          <img
            src={job.thumbnail}
            alt=""
            className="size-12 shrink-0 rounded-md bg-field object-cover"
          />
        ) : (
          <VinylDisc size={48} />
        )}
        <span className="min-w-0">
          <span className="block truncate text-[13px] font-medium text-fg">{title}</span>
          {job.artist && <span className="block truncate text-xs text-fg-muted">{job.artist}</span>}
        </span>
      </button>
    </li>
  );
}

export default function Home() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const jobs = useJobsStore((s) => s.jobs);
  const pushToast = useUiStore((s) => s.pushToast);
  const all = useMemo(() => sortedJobs(jobs), [jobs]);
  const running = useMemo(
    () => all.filter((j) => j.status === "running" || j.status === "queued"),
    [all],
  );
  // Provisório: os últimos concluídos; na F10 vêm da biblioteca.
  const recent = useMemo(
    () =>
      all
        .filter((j) => j.status === "done")
        .sort((a, b) => (b.finishedAt ?? 0) - (a.finishedAt ?? 0))
        .slice(0, RECENT_COUNT),
    [all],
  );

  const guard = async (action: () => Promise<unknown>) => {
    try {
      await action();
    } catch (e) {
      pushToast({ message: errorText(t, e), tone: "error" });
    }
  };

  const pasteAndDownload = () =>
    guard(async () => {
      const text = (await api.clipboardReadText()).trim();
      if (!text) {
        pushToast({ message: t("home.clipboardEmpty"), tone: "warning" });
        return;
      }
      const ui = useUiStore.getState();
      ui.setCommandBarText(text);
      ui.requestCommandBarSubmit();
    });

  return (
    <section aria-labelledby="screen-title">
      <ScreenHeader title={t("home.title")} subtitle={t("home.subtitle")} />
      <CommandBar autoFocus />

      <div className="mt-8">
        <h2 className="mb-3 text-sm font-semibold text-fg-secondary">{t("home.quickActions")}</h2>
        <div className="grid grid-cols-[repeat(auto-fit,minmax(200px,1fr))] gap-3">
          <QuickAction
            icon={<ClipboardPaste aria-hidden="true" />}
            label={t("home.actions.pasteAndDownload")}
            onClick={() => void pasteAndDownload()}
          />
          <QuickAction
            icon={<ListMusic aria-hidden="true" />}
            label={t("home.actions.analyzePlaylist")}
            onClick={() => {
              useUiStore.getState().requestCommandBarFocus();
              pushToast({ message: t("home.analyzePlaylistHint"), tone: "info" });
            }}
          />
          <QuickAction
            icon={<Tags aria-hidden="true" />}
            label={t("home.actions.editTags")}
            onClick={() => navigate("/tag-editor")}
          />
          <QuickAction
            icon={<FolderOpen aria-hidden="true" />}
            label={t("home.actions.openMusicFolder")}
            onClick={() => void guard(() => api.openOutputDir())}
          />
        </div>
      </div>

      {running.length > 0 ? (
        <div className="mt-8">
          <div className="mb-3 flex items-center justify-between">
            <h2 className="text-sm font-semibold text-fg-secondary">{t("home.inProgress")}</h2>
            <Link to="/activity" className="text-xs text-accent hover:text-accent-hover">
              {t("home.seeAll")}
            </Link>
          </div>
          <ul className="flex flex-col gap-2">
            {running.slice(0, 3).map((job) => (
              <JobMiniCard key={job.id} job={job} />
            ))}
          </ul>
        </div>
      ) : (
        recent.length === 0 && (
          <EmptyState
            className="mt-8"
            icon={<House aria-hidden="true" />}
            title={t("home.empty.title")}
            description={t("home.empty.description")}
          />
        )
      )}

      {recent.length > 0 && (
        <div className="mt-8">
          <h2 className="mb-3 text-sm font-semibold text-fg-secondary">{t("home.recent")}</h2>
          <ul className="grid grid-cols-[repeat(auto-fill,minmax(220px,1fr))] gap-3">
            {recent.map((job) => (
              <RecentCard
                key={job.id}
                job={job}
                onOpen={(j) => {
                  const path = j.outputPath;
                  if (path) void guard(() => api.libraryReveal(path));
                }}
              />
            ))}
          </ul>
        </div>
      )}
    </section>
  );
}
