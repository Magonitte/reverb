import { useMemo, useState } from "react";
import { ArrowDownUp } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { Job } from "@/bindings/Job";
import { JobMiniCard } from "@/components/JobMiniCard";
import { ScreenHeader } from "@/components/ScreenHeader";
import { EmptyState } from "@/components/ui/EmptyState";
import { Tabs } from "@/components/ui/Tabs";
import { sortedJobs, useJobsStore } from "@/stores/jobs";

type TabId = "active" | "done" | "failed";

const ACTIVE = new Set<Job["status"]>(["running", "queued"]);

export default function Activity() {
  const { t } = useTranslation();
  const jobs = useJobsStore((s) => s.jobs);
  const [tab, setTab] = useState<TabId>("active");

  const groups = useMemo(() => {
    const all = sortedJobs(jobs);
    return {
      active: all.filter((j) => ACTIVE.has(j.status)),
      done: all.filter((j) => j.status === "done"),
      failed: all.filter((j) => j.status === "failed" || j.status === "cancelled"),
    };
  }, [jobs]);

  const running = groups.active.filter((j) => j.status === "running").length;
  const queued = groups.active.length - running;

  return (
    <section aria-labelledby="screen-title">
      <ScreenHeader
        title={t("activity.title")}
        subtitle={t("activity.counters", { running, queued, done: groups.done.length })}
      />
      <Tabs
        label={t("activity.tabsLabel")}
        value={tab}
        onChange={(id) => setTab(id as TabId)}
        tabs={[
          { id: "active", label: t("activity.tabs.active"), count: groups.active.length },
          { id: "done", label: t("activity.tabs.done"), count: groups.done.length },
          { id: "failed", label: t("activity.tabs.failed"), count: groups.failed.length },
        ]}
      >
        {groups[tab].length > 0 ? (
          <ul className="flex flex-col gap-2">
            {groups[tab].map((job) => (
              <JobMiniCard key={job.id} job={job} />
            ))}
          </ul>
        ) : (
          <EmptyState
            icon={<ArrowDownUp aria-hidden="true" />}
            title={t(`activity.empty.${tab}.title`)}
            description={t(`activity.empty.${tab}.description`)}
          />
        )}
      </Tabs>
    </section>
  );
}
