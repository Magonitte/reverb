import { useMemo, type ReactNode } from "react";
import { ArrowUpRight, ClipboardPaste, FolderOpen, House, ListMusic, Tags } from "lucide-react";
import { Link, useNavigate } from "react-router";
import { useTranslation } from "react-i18next";
import type { LibraryItem } from "@/bindings/LibraryItem";
import { LibraryCover } from "@/components/LibraryCover";
import { useLibraryStore } from "@/stores/library";
import { CommandBar } from "@/components/command-bar/CommandBar";
import { JobMiniCard } from "@/components/JobMiniCard";
import { ScreenHeader } from "@/components/ScreenHeader";
import { EmptyState } from "@/components/ui/EmptyState";
import { errorText } from "@/lib/errors";
import { api } from "@/lib/ipc/api";
import { sortedJobs, useJobsStore } from "@/stores/jobs";
import { useUiStore } from "@/stores/ui";
import { SyncSummary } from "@/components/playlists/SyncSummary";

function QuickAction({
  icon,
  label,
  description,
  onClick,
}: {
  icon: ReactNode;
  label: string;
  description: string;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      aria-label={label}
      className="glass group flex items-start gap-3 rounded-lg p-4 text-left text-sm font-medium text-fg transition-colors duration-[140ms] hover:border-glass-border-hover hover:bg-glass-hover"
    >
      <span className="flex size-10 shrink-0 items-center justify-center rounded-md bg-accent-muted text-accent [&_svg]:size-5">
        {icon}
      </span>
      <span className="min-w-0 flex-1">
        <span className="block">{label}</span>
        <span className="mt-1 block text-xs font-normal leading-relaxed text-fg-muted">
          {description}
        </span>
      </span>
      <ArrowUpRight
        className="size-4 shrink-0 text-fg-muted group-hover:text-accent"
        aria-hidden="true"
      />
    </button>
  );
}

function RecentCard({ job, onOpen }: { job: LibraryItem; onOpen: (job: LibraryItem) => void }) {
  const { t } = useTranslation();
  const title = job.title;
  return (
    <li data-testid="recent-item">
      <button
        type="button"
        onClick={() => onOpen(job)}
        aria-label={t("home.revealRecent", { title })}
        className="glass flex w-full items-center gap-3 rounded-lg p-2 text-left transition-colors duration-[140ms] hover:bg-glass-hover"
      >
        <LibraryCover id={job.id} updatedAt={job.updatedAt} missing={job.missing} />
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
  const recent = useLibraryStore((s) => s.recent);

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
      <div className="glass rounded-xl border border-glass-border p-5 max-sm:p-4">
        <h2 className="mb-3 text-sm font-semibold text-fg">{t("home.startTitle")}</h2>
        <CommandBar autoFocus />
      </div>
      <SyncSummary />

      <div className="mt-8">
        <h2 className="mb-3 text-sm font-semibold text-fg-secondary">{t("home.quickActions")}</h2>
        <div className="grid grid-cols-1 gap-3 sm:grid-cols-2 xl:grid-cols-4">
          <QuickAction
            icon={<ClipboardPaste aria-hidden="true" />}
            label={t("home.actions.pasteAndDownload")}
            description={t("home.actionHints.paste")}
            onClick={() => void pasteAndDownload()}
          />
          <QuickAction
            icon={<ListMusic aria-hidden="true" />}
            label={t("home.actions.analyzePlaylist")}
            description={t("home.actionHints.playlist")}
            onClick={() => {
              useUiStore.getState().requestCommandBarFocus();
              pushToast({ message: t("home.analyzePlaylistHint"), tone: "info" });
            }}
          />
          <QuickAction
            icon={<Tags aria-hidden="true" />}
            label={t("home.actions.editTags")}
            description={t("home.actionHints.tags")}
            onClick={() => navigate("/tag-editor")}
          />
          <QuickAction
            icon={<FolderOpen aria-hidden="true" />}
            label={t("home.actions.openMusicFolder")}
            description={t("home.actionHints.folder")}
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
                  const path = j.filePath;
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
