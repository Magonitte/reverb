import type { DuplicateHit } from "@/bindings/DuplicateHit";
import type { EnqueueRequest } from "@/bindings/EnqueueRequest";
import type { Job } from "@/bindings/Job";
import type { MoveTarget } from "@/bindings/MoveTarget";
import type { QueueState } from "@/bindings/QueueState";
import { mockBus } from "./bus";

let jobs: Job[] = [];
let paused = false;
let healing = false;
let counter = 0;

export function resetMockQueue(): void {
  stopMockSimulation();
  jobs = [];
  paused = false;
  healing = false;
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
    healing,
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

/** Autocura em andamento (cenário `heal`): reflete em `queue://state.healing`. */
export function setMockHealing(value: boolean): void {
  healing = value;
  emitState();
}

/** Insere jobs prontos (cenários); campos omitidos recebem valores de um job pendente. */
export function seedMockJobs(seeds: Array<Partial<Job> & { sourceUrl: string }>): Job[] {
  const created = seeds.map((seed) => {
    counter += 1;
    const job: Job = {
      id: `mock-job-${counter}`,
      kind: "single",
      provider: "youtube",
      sourceId: null,
      title: null,
      artist: null,
      thumbnail: null,
      durationS: null,
      profileId: "original",
      options: {},
      metadataOverride: null,
      warnings: [],
      playlistCtx: null,
      syncId: null,
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
      position: counter,
      createdAt: counter,
      updatedAt: counter,
      finishedAt: null,
      ...seed,
    };
    jobs.push(job);
    return job;
  });
  created.forEach(emitJob);
  emitState();
  return created;
}

/** Estágios simulados, na ordem do pipeline (arquitetura §10). */
const SIM_STAGES: Job["stage"][] = ["analyzing", "downloading", "converting", "metadata", "moving"];
const SIM_STEP = 0.34;

/**
 * Um passo da simulação: promove pendentes (até `parallelism`) e avança os que rodam por estágios.
 * Determinístico — os testes chamam direto; a página usa `startMockSimulation`.
 */
export function mockSimulationStep(parallelism = 2): void {
  if (!paused) {
    const free = parallelism - jobs.filter((j) => j.status === "running").length;
    mockJobsList()
      .filter((j) => j.status === "queued")
      .slice(0, Math.max(0, free))
      .forEach((job) => {
        job.status = "running";
        job.stage = SIM_STAGES[0]!;
        job.attempts += 1;
        emitJob(job);
      });
  }
  for (const job of jobs.filter((j) => j.status === "running")) {
    job.progress = Math.min(1, job.progress + SIM_STEP);
    const index = SIM_STAGES.indexOf(job.stage);
    if (job.progress >= 1) {
      if (index + 1 >= SIM_STAGES.length) {
        job.status = "done";
        job.stage = "done";
        job.progress = 1;
        job.overallProgress = 1;
        job.speedBps = null;
        job.etaS = null;
        job.finishedAt = job.updatedAt + 1;
        job.outputPath = `C:/Musicas/Reverb/${job.title ?? job.sourceId ?? job.id}.opus`;
      } else {
        job.stage = SIM_STAGES[index + 1]!;
        job.progress = 0;
      }
    }
    if (job.status === "running") {
      job.overallProgress = (SIM_STAGES.indexOf(job.stage) + job.progress) / SIM_STAGES.length;
      const downloading = job.stage === "downloading";
      job.speedBps = downloading ? 2_400_000 : null;
      job.etaS = downloading ? Math.round((1 - job.progress) * 6) : null;
    }
    job.updatedAt += 1;
    emitJob(job);
  }
  emitState();
}

let simulation: ReturnType<typeof setInterval> | null = null;

/** Liga a simulação temporizada (progresso por estágios) para o desenvolvimento no navegador. */
export function startMockSimulation(intervalMs = 700): void {
  if (simulation !== null) return;
  simulation = setInterval(() => mockSimulationStep(), intervalMs);
}

export function stopMockSimulation(): void {
  if (simulation !== null) clearInterval(simulation);
  simulation = null;
}
