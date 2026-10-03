import { useMemo, useState } from "react";
import {
  DndContext,
  KeyboardSensor,
  PointerSensor,
  closestCenter,
  useSensor,
  useSensors,
  type DragEndEvent,
} from "@dnd-kit/core";
import {
  SortableContext,
  sortableKeyboardCoordinates,
  useSortable,
  verticalListSortingStrategy,
} from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import { ArrowDownUp, Pause, Play } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { Job } from "@/bindings/Job";
import { DragHandle, JobCard, type JobCardProps } from "@/components/JobCard";
import { ScreenHeader } from "@/components/ScreenHeader";
import { Button } from "@/components/ui/Button";
import { ConfirmDialog } from "@/components/ui/ConfirmDialog";
import { EmptyState } from "@/components/ui/EmptyState";
import { Tabs } from "@/components/ui/Tabs";
import { errorText } from "@/lib/errors";
import { api } from "@/lib/ipc/api";
import { sortedJobs, useJobsStore } from "@/stores/jobs";
import { useSettingsStore } from "@/stores/settings";
import { useUiStore } from "@/stores/ui";

type TabId = "active" | "done" | "failed";

const ACTIVE = new Set<Job["status"]>(["running", "queued"]);
const WARN_RATIO = 0.9;

type CardActions = Omit<JobCardProps, "job" | "handle" | "rootRef" | "rootProps">;

function SortableJob({ job, actions }: { job: Job; actions: CardActions }) {
  const { t } = useTranslation();
  const {
    attributes,
    listeners,
    setNodeRef,
    setActivatorNodeRef,
    transform,
    transition,
    isDragging,
  } = useSortable({ id: job.id });
  return (
    <JobCard
      job={job}
      {...actions}
      rootRef={setNodeRef}
      rootProps={{
        style: {
          transform: CSS.Transform.toString(transform),
          transition,
          opacity: isDragging ? 0.6 : 1,
        },
      }}
      handle={
        <DragHandle
          ref={setActivatorNodeRef}
          label={t("activity.dragJob", { title: job.title ?? job.sourceUrl })}
          {...attributes}
          {...listeners}
        />
      }
    />
  );
}

export default function Activity() {
  const { t } = useTranslation();
  const jobs = useJobsStore((s) => s.jobs);
  const queue = useJobsStore((s) => s.queue);
  const settings = useSettingsStore((s) => s.settings);
  const pushToast = useUiStore((s) => s.pushToast);
  const [tab, setTab] = useState<TabId>("active");
  const [confirmCancelAll, setConfirmCancelAll] = useState(false);

  const groups = useMemo(() => {
    const all = sortedJobs(jobs);
    return {
      active: all.filter((j) => ACTIVE.has(j.status)),
      done: all.filter((j) => j.status === "done"),
      failed: all.filter((j) => j.status === "failed" || j.status === "cancelled"),
    };
  }, [jobs]);

  const running = groups.active.filter((j) => j.status === "running");
  const queued = groups.active.filter((j) => j.status === "queued");
  const queueLimit = settings?.queueLimit ?? 500;
  const nearLimit = groups.active.length >= queueLimit * WARN_RATIO;

  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 6 } }),
    useSensor(KeyboardSensor, { coordinateGetter: sortableKeyboardCoordinates }),
  );

  const run = async (action: () => Promise<unknown>) => {
    try {
      await action();
    } catch (e) {
      pushToast({ message: errorText(t, e), tone: "error" });
    }
  };

  const actions: CardActions = {
    maxAttempts: settings?.maxAttempts ?? 3,
    onCancel: (job) => void run(() => api.jobCancel(job.id)),
    onRetry: (job) => void run(() => api.jobRetry(job.id)),
    onRemove: (job) => void run(() => api.jobRemove(job.id)),
    onReveal: (job) => {
      const path = job.outputPath;
      if (path) void run(() => api.libraryReveal(path));
    },
    onOpen: (job) => { const path = job.outputPath; if (path) void run(() => api.libraryOpenFile(path)); },
  };

  const onDragEnd = ({ active, over }: DragEndEvent) => {
    if (!over || active.id === over.id) return;
    const from = queued.findIndex((j) => j.id === active.id);
    const to = queued.findIndex((j) => j.id === over.id);
    if (from < 0 || to < 0) return;
    // Descendo, o item fica depois do alvo; subindo, antes.
    const target = from < to ? { after: String(over.id) } : { before: String(over.id) };
    void run(() => api.jobMove(String(active.id), target));
  };

  // O DndContext fica fora da lista: ele injeta elementos de acessibilidade ao lado dos filhos
  // e um <ul> só pode ter <li>.
  const renderActive = () => (
    <DndContext sensors={sensors} collisionDetection={closestCenter} onDragEnd={onDragEnd}>
      <SortableContext items={queued.map((j) => j.id)} strategy={verticalListSortingStrategy}>
        <ul className="flex flex-col gap-2">
          {running.map((job) => (
            <JobCard key={job.id} job={job} {...actions} />
          ))}
          {queued.map((job) => (
            <SortableJob key={job.id} job={job} actions={actions} />
          ))}
        </ul>
      </SortableContext>
    </DndContext>
  );

  return (
    <section aria-labelledby="screen-title">
      <ScreenHeader
        title={t("activity.title")}
        subtitle={t("activity.counters", {
          running: running.length,
          queued: queued.length,
          done: groups.done.length,
        })}
        actions={
          <div className="flex flex-wrap gap-2">
            <Button
              size="sm"
              icon={
                queue.paused ? (
                  <Play className="size-3.5" aria-hidden="true" />
                ) : (
                  <Pause className="size-3.5" aria-hidden="true" />
                )
              }
              onClick={() => void run(() => (queue.paused ? api.queueResume() : api.queuePause()))}
            >
              {queue.paused ? t("activity.resumeQueue") : t("activity.pauseQueue")}
            </Button>
            <Button
              size="sm"
              variant="danger"
              disabled={groups.active.length === 0}
              onClick={() => setConfirmCancelAll(true)}
            >
              {t("activity.cancelAll")}
            </Button>
            <Button
              size="sm"
              disabled={groups.done.length + groups.failed.length === 0}
              onClick={() => void run(() => api.jobsClearFinished())}
            >
              {t("activity.clearFinished")}
            </Button>
          </div>
        }
      />
      {nearLimit && (
        <p
          role="status"
          data-testid="queue-limit-warning"
          className="mb-4 text-[13px] text-warning"
        >
          {t("activity.queueNearLimit", { count: groups.active.length, limit: queueLimit })}
        </p>
      )}
      {queue.paused && (
        <p role="status" data-testid="queue-paused" className="mb-4 text-[13px] text-fg-muted">
          {t("activity.paused")}
        </p>
      )}
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
        {groups[tab].length === 0 ? (
          <EmptyState
            icon={<ArrowDownUp aria-hidden="true" />}
            title={t(`activity.empty.${tab}.title`)}
            description={t(`activity.empty.${tab}.description`)}
          />
        ) : tab === "active" ? (
          renderActive()
        ) : (
          <ul className="flex flex-col gap-2">
            {groups[tab].map((job) => (
              <JobCard key={job.id} job={job} {...actions} />
            ))}
          </ul>
        )}
      </Tabs>
      <ConfirmDialog
        open={confirmCancelAll}
        title={t("activity.cancelAllConfirm.title")}
        message={t("activity.cancelAllConfirm.message")}
        confirmLabel={t("activity.cancelAllConfirm.confirm")}
        cancelLabel={t("common.cancel")}
        closeLabel={t("common.close")}
        danger
        onCancel={() => setConfirmCancelAll(false)}
        onConfirm={() => {
          setConfirmCancelAll(false);
          void run(() => api.jobsCancelAll());
        }}
      />
    </section>
  );
}
