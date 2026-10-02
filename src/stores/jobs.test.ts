import { act } from "@testing-library/react";
import { beforeEach, describe, expect, it } from "vitest";
import type { Job } from "@/bindings/Job";
import { mockBus, resetMockQueue } from "@/lib/ipc/mock";
import { mockEnqueue, mockJobCancel, mockJobMove, mockQueuePause } from "@/lib/ipc/mock/queue";
import { activeCount, initJobsStore, sortedJobs, useJobsStore } from "./jobs";

beforeEach(() => {
  resetMockQueue();
  mockBus.clear();
  useJobsStore.setState({
    jobs: {},
    queue: { paused: false, running: 0, queued: 0, healing: false },
  });
});

const enqueue = (n: number, extra: Partial<Parameters<typeof mockEnqueue>[0]> = {}) =>
  mockEnqueue({
    url: `https://www.youtube.com/watch?v=track${n}`,
    sourceId: `track${n}`,
    metadataOverride: null,
    priority: false,
    allowDuplicate: false,
    ...extra,
  });

describe("Store jobs (T1)", () => {
  it("aplica job://updated e job://removed vindos do barramento", async () => {
    const off = await initJobsStore();
    const first = enqueue(1);
    const second = enqueue(2);
    expect(Object.keys(useJobsStore.getState().jobs)).toEqual([first.id, second.id]);

    act(() => mockBus.emit("job://updated", { ...first, status: "running", stage: "downloading" }));
    expect(useJobsStore.getState().jobs[first.id]).toMatchObject({
      status: "running",
      stage: "downloading",
    });

    act(() => mockBus.emit("job://removed", { id: first.id }));
    expect(Object.keys(useJobsStore.getState().jobs)).toEqual([second.id]);
    off();
  });

  it("mantém a ordem da fila por posição, inclusive com prioridade e reordenação", async () => {
    await initJobsStore();
    const a = enqueue(1);
    const b = enqueue(2);
    const c = enqueue(3);
    expect(sortedJobs(useJobsStore.getState().jobs).map((j) => j.id)).toEqual([a.id, b.id, c.id]);

    const urgent = enqueue(4, { priority: true });
    expect(sortedJobs(useJobsStore.getState().jobs)[0]?.id).toBe(urgent.id);

    mockJobMove(c.id, "front");
    expect(sortedJobs(useJobsStore.getState().jobs)[0]?.id).toBe(c.id);
  });

  it("queue://state atualiza o estado da fila e os contadores contam só ativos", async () => {
    await initJobsStore();
    const a = enqueue(1);
    enqueue(2);
    const done: Job = { ...a, id: "x", status: "done" };
    act(() => mockBus.emit("job://updated", done));
    expect(activeCount(useJobsStore.getState().jobs)).toBe(2);

    mockJobCancel(a.id);
    expect(activeCount(useJobsStore.getState().jobs)).toBe(1);
    expect(useJobsStore.getState().queue).toMatchObject({ queued: 1, running: 0, paused: false });

    mockQueuePause();
    expect(useJobsStore.getState().queue.paused).toBe(true);
  });

  it("load() carrega jobs e estado da fila de uma vez", async () => {
    enqueue(1);
    enqueue(2);
    await useJobsStore.getState().load();
    expect(Object.keys(useJobsStore.getState().jobs)).toHaveLength(2);
    expect(useJobsStore.getState().queue.queued).toBe(2);
  });
});
