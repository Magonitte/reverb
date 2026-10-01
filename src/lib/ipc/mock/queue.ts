import type { DuplicateHit } from "@/bindings/DuplicateHit";
import type { EnqueueRequest } from "@/bindings/EnqueueRequest";
import type { Job } from "@/bindings/Job";
import type { MoveTarget } from "@/bindings/MoveTarget";
import type { QueueState } from "@/bindings/QueueState";
import { mockBus } from "./bus";

let jobs: Job[] = [];
let paused = false;
let counter = 0;

export function resetMockQueue(): void {
  jobs = [];
  paused = false;
  counter = 0;
}

function emitJob(job: Job): void {
  mockBus.emit("job://updated", job);
}

function emitState(): void {
  mockBus.emit("queue://state", mockQueueState());
}

function invalid(message: string): never {
  throw { kind: "invalid", message };
}

function find(id: string): Job {
  const job = jobs.find((j) => j.id === id);
  if (!job) invalid(`job não encontrado: ${id}`);
  return job;
}

export function mockQueueState(): QueueState {
  return {
    paused,
    running: jobs.filter((j) => j.status === "running").length,
    queued: jobs.filter((j) => j.status === "queued").length,
    healing: false,
  };
}

export function mockEnqueue(request: EnqueueRequest): Job {
  const url = request.url.trim();
  if (!url) invalid("a URL é obrigatória");
  const sourceId = request.sourceId ?? null;
  const profileId = request.profileId ?? "original";
  const duplicate =
    sourceId !== null &&
    jobs.some(
      (j) =>
        j.sourceId === sourceId &&
        j.profileId === profileId &&
        j.status !== "failed" &&
        j.status !== "cancelled",
    );
  if (duplicate && !request.allowDuplicate) {
    throw { kind: "duplicate", message: `já existe: ${sourceId}` };
  }
  counter += 1;
  const positions = jobs.map((j) => j.position);
  const position = request.priority ? Math.min(0, ...positions) - 1 : Math.max(0, ...positions) + 1;
  const job: Job = {
    id: `mock-job-${counter}`,
    kind: request.playlistCtx ? "playlist_item" : "single",
    provider: "youtube",
    sourceUrl: url,
    sourceId,
    title: request.title ?? null,
    artist: null,
    thumbnail: request.thumbnail ?? null,
    durationS: request.durationS ?? null,
    profileId,
    options: request.options ?? {},
    metadataOverride: request.metadataOverride ?? null,
    warnings: [],
    playlistCtx: request.playlistCtx ?? null,
    syncId: request.playlistCtx?.syncId ?? null,
    status: "queued",
    stage: "waiting",
    progress: 0,
    overallProgress: 0,
    speedBps: null,
    etaS: null,
    errorKind: null,
    errorMessage: null,
    attempts: 0,
    outputPath: null,
    libraryId: null,
    position,
    createdAt: 0,
    updatedAt: 0,
    finishedAt: null,
  };
  jobs.push(job);
  emitJob(job);
  emitState();
  return job;
}

export function mockCheckDuplicates(sourceIds: string[], profileId?: string): DuplicateHit[] {
  const profile = profileId ?? "original";
  return sourceIds.flatMap((sourceId) => {
    const hit = jobs.find(
      (j) =>
        j.sourceId === sourceId &&
        j.profileId === profile &&
        j.status !== "failed" &&
        j.status !== "cancelled",
    );
    return hit ? [{ sourceId, foundIn: "queue", jobId: hit.id }] : [];
  });
}

export function mockJobsList(): Job[] {
  return [...jobs].sort((a, b) => a.position - b.position);
}

function finish(job: Job, status: "cancelled" | "queued"): void {
  job.status = status;
  job.stage = "waiting";
  job.updatedAt += 1;
  emitJob(job);
  emitState();
}

export function mockJobCancel(id: string): void {
  const job = find(id);
  if (job.status === "queued" || job.status === "running") finish(job, "cancelled");
}

export function mockJobsCancelAll(): void {
  jobs
    .filter((j) => j.status === "queued" || j.status === "running")
    .forEach((j) => finish(j, "cancelled"));
}

export function mockJobRetry(id: string): Job {
  const job = find(id);
  if (job.status !== "failed" && job.status !== "cancelled") {
    invalid("só é possível tentar de novo um job que falhou ou foi cancelado");
  }
  job.attempts = 0;
  job.errorKind = null;
  job.errorMessage = null;
  job.position = Math.max(0, ...jobs.map((j) => j.position)) + 1;
  finish(job, "queued");
  return job;
}

export function mockJobRemove(id: string): void {
  find(id);
  jobs = jobs.filter((j) => j.id !== id);
  mockBus.emit("job://removed", { id });
  emitState();
}

export function mockJobsClearFinished(): number {
  const finished = jobs.filter((j) => ["done", "failed", "cancelled"].includes(j.status));
  jobs = jobs.filter((j) => !finished.includes(j));
  finished.forEach((j) => mockBus.emit("job://removed", { id: j.id }));
  return finished.length;
}

export function mockJobMove(id: string, target: MoveTarget): void {
  const moving = find(id);
  if (moving.status !== "queued") invalid(`só é possível reordenar um job na fila: ${id}`);
  const queued = mockJobsList().filter((j) => j.status === "queued" && j.id !== id);
  let index = 0;
  if (target === "back") index = queued.length;
  else if (target !== "front") {
    const reference = "before" in target ? target.before : target.after;
    const at = queued.findIndex((j) => j.id === reference);
    if (at < 0) invalid(`job de referência não está na fila: ${reference}`);
    index = "before" in target ? at : at + 1;
  }
  queued.splice(index, 0, moving);
  const slots = queued.map((j) => j.position).sort((a, b) => a - b);
  const base = slots[0] ?? 1;
  queued.forEach((job, offset) => {
    if (job.position !== base + offset) {
      job.position = base + offset;
      emitJob(job);
    }
  });
}

export function mockQueuePause(): void {
  paused = true;
  emitState();
}

export function mockQueueResume(): void {
  paused = false;
  emitState();
}
