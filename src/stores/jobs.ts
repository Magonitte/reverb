import { create } from "zustand";
import type { Job } from "@/bindings/Job";
import type { QueueState } from "@/bindings/QueueState";
import { api } from "@/lib/ipc/api";
import { onEvent } from "@/lib/ipc/events";

interface JobsState {
  jobs: Record<string, Job>;
  queue: QueueState;
  load: () => Promise<void>;
  applyUpdated: (job: Job) => void;
  applyRemoved: (id: string) => void;
}

const EMPTY_QUEUE: QueueState = { paused: false, running: 0, queued: 0, healing: false };

export const useJobsStore = create<JobsState>((set) => ({
  jobs: {},
  queue: EMPTY_QUEUE,
  load: async () => {
    const [list, queue] = await Promise.all([api.jobsList(), api.queueState()]);
    set({ jobs: Object.fromEntries(list.map((j) => [j.id, j])), queue });
  },
  applyUpdated: (job) => set((s) => ({ jobs: { ...s.jobs, [job.id]: job } })),
  applyRemoved: (id) =>
    set((s) => {
      const { [id]: _removed, ...rest } = s.jobs;
      void _removed;
      return { jobs: rest };
    }),
}));

/** Jobs em ordem de fila (posição crescente). */
export function sortedJobs(jobs: Record<string, Job>): Job[] {
  return Object.values(jobs).sort((a, b) => a.position - b.position);
}

/** Ativos = rodando + pendentes (badge da sidebar). */
export function activeCount(jobs: Record<string, Job>): number {
  return Object.values(jobs).filter((j) => j.status === "running" || j.status === "queued").length;
}

export async function initJobsStore(): Promise<() => void> {
  const { applyUpdated, applyRemoved, load } = useJobsStore.getState();
  const offs = await Promise.all([
    onEvent<Job>("job://updated", applyUpdated),
    onEvent<{ id: string }>("job://removed", (p) => applyRemoved(p.id)),
    onEvent<QueueState>("queue://state", (queue) => useJobsStore.setState({ queue })),
  ]);
  await load();
  return () => offs.forEach((off) => off());
}
