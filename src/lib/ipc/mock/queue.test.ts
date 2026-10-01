import { beforeEach, describe, expect, it } from "vitest";
import type { Job } from "@/bindings/Job";
import { api } from "@/lib/ipc/api";
import { mockBus, resetMockQueue } from "@/lib/ipc/mock";

const video = (n: number, extra = {}) => ({
  url: `https://www.youtube.com/watch?v=vid${n}`,
  sourceId: `vid${n}`,
  ...extra,
});

beforeEach(() => {
  resetMockQueue();
  mockBus.clear();
});

describe("backend mock da fila", () => {
  it("enfileira, emite job://updated e lista na ordem", async () => {
    const updates: Job[] = [];
    mockBus.on("job://updated", (j) => updates.push(j as Job));
    const first = await api.enqueue(video(1));
    const urgent = await api.enqueue(video(2, { priority: true }));

    expect(first.status).toBe("queued");
    expect(updates.map((j) => j.id)).toEqual([first.id, urgent.id]);
    expect((await api.jobsList()).map((j) => j.id)).toEqual([urgent.id, first.id]);
    expect(await api.queueState()).toEqual({
      paused: false,
      running: 0,
      queued: 2,
      healing: false,
    });
  });

  it("recusa duplicata, aceita com allowDuplicate e com outro perfil", async () => {
    await api.enqueue(video(1));
    await expect(api.enqueue(video(1))).rejects.toMatchObject({ kind: "duplicate" });
    await api.enqueue(video(1, { allowDuplicate: true }));
    await api.enqueue(video(1, { profileId: "mp3_v0" }));
    expect(await api.checkDuplicates(["vid1", "vid9"])).toMatchObject([
      { sourceId: "vid1", foundIn: "queue" },
    ]);
  });

  it("cancela, tenta de novo, reordena, remove e limpa", async () => {
    const a = await api.enqueue(video(1));
    const b = await api.enqueue(video(2));
    const c = await api.enqueue(video(3));

    await api.jobMove(c.id, { before: a.id });
    expect((await api.jobsList()).map((j) => j.id)).toEqual([c.id, a.id, b.id]);
    await api.jobMove(c.id, "back");
    expect((await api.jobsList()).map((j) => j.id)).toEqual([a.id, b.id, c.id]);

    await api.jobCancel(a.id);
    expect((await api.jobsList())[0]?.status).toBe("cancelled");
    expect((await api.jobRetry(a.id)).status).toBe("queued");
    await expect(api.jobRetry(b.id)).rejects.toMatchObject({ kind: "invalid" });

    await api.jobsCancelAll();
    expect(await api.jobsClearFinished()).toBe(3);
    expect(await api.jobsList()).toEqual([]);
  });

  it("pausa e retoma emitindo queue://state", async () => {
    const states: boolean[] = [];
    mockBus.on("queue://state", (s) => states.push((s as { paused: boolean }).paused));
    await api.queuePause();
    await api.queueResume();
    expect(states).toEqual([true, false]);
  });
});
