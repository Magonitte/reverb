import { useMemo } from "react";
import { House } from "lucide-react";
import { Link } from "react-router";
import { useTranslation } from "react-i18next";
import { CommandBar } from "@/components/CommandBar";
import { JobMiniCard } from "@/components/JobMiniCard";
import { ScreenHeader } from "@/components/ScreenHeader";
import { EmptyState } from "@/components/ui/EmptyState";
import { sortedJobs, useJobsStore } from "@/stores/jobs";

export default function Home() {
  const { t } = useTranslation();
  const jobs = useJobsStore((s) => s.jobs);
  const running = useMemo(
    () => sortedJobs(jobs).filter((j) => j.status === "running" || j.status === "queued"),
    [jobs],
  );

  return (
    <section aria-labelledby="screen-title">
      <ScreenHeader title={t("home.title")} subtitle={t("home.subtitle")} />
      <CommandBar autoFocus />
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
        <EmptyState
          className="mt-8"
          icon={<House aria-hidden="true" />}
          title={t("home.empty.title")}
          description={t("home.empty.description")}
        />
      )}
    </section>
  );
}
