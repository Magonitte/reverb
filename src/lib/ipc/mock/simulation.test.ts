import { beforeEach, describe, expect, it } from "vitest";
import type { Job } from "@/bindings/Job";
import { mockBus } from "./bus";
import { SCENARIOS, applyScenario, scenarioFromLocation } from "./scenarios";
import {
  mockEnqueue,
  mockJobsList,
  mockQueuePause,
  mockQueueState,
  mockSimulationStep,
  resetMockQueue,
} from "./queue";

beforeEach(() => {
  resetMockQueue();
  mockBus.clear();
});

const enqueue = (id: string) =>
  mockEnqueue({ url: `https://www.youtube.com/watch?v=${id}` } as never);

describe("simulação da fila (F05 T9)", () => {
  it("promove até `parallelism` jobs e os leva por todos os estágios até concluir", () => {
    ["a", "b", "c"].forEach(enqueue);
    const seen = new Set<string>();
    const updates: Job[] = [];
    mockBus.on("job://updated", (p) => updates.push(structuredClone(p as Job)));

    mockSimulationStep(2);
    expect(mockQueueState()).toMatchObject({ running: 2, queued: 1 });

    for (let i = 0; i < 40 && mockJobsList().some((j) => j.status !== "done"); i += 1) {
      mockSimulationStep(2);
      mockJobsList().forEach((j) => seen.add(j.stage));
      expect(mockQueueState().running).toBeLessThanOrEqual(2);
    }

    expect(mockJobsList().every((j) => j.status === "done" && j.overallProgress === 1)).toBe(true);
    for (const stage of ["analyzing", "downloading", "converting", "metadata", "moving", "done"]) {
      expect(seen).toContain(stage);
    }
    expect(updates.length).toBeGreaterThan(10);
  });

  it("o progresso geral é monotônico", () => {
    enqueue("solo");
    let last = 0;
    for (let i = 0; i < 40; i += 1) {
      mockSimulationStep(1);
      const progress = mockJobsList()[0]!.overallProgress;
      expect(progress).toBeGreaterThanOrEqual(last);
      last = progress;
    }
    expect(last).toBe(1);
  });

  it("não inicia jobs com a fila pausada", () => {
    enqueue("a");
    mockQueuePause();
    mockSimulationStep(2);
    expect(mockQueueState()).toMatchObject({ running: 0, queued: 1, paused: true });
  });
});

describe("cenários (?scenario=)", () => {
  it("lê o cenário da query string e ignora valores desconhecidos", () => {
    expect(scenarioFromLocation("?scenario=busy")).toBe("busy");
    expect(scenarioFromLocation("?x=1&scenario=heal")).toBe("heal");
    expect(scenarioFromLocation("?scenario=outro")).toBeNull();
    expect(scenarioFromLocation("")).toBeNull();
    expect([...SCENARIOS]).toEqual([
      "onboarding",
      "onboarding-error",
      "playlists",
      "empty",
      "big",
      "busy",
      "errors",
      "heal",
      "update-available",
      "update-downloading",
      "update-error",
    ]);
  });

  it("empty não cria nada; busy cria rodando + pendentes + concluído", () => {
    applyScenario("empty");
    expect(mockJobsList()).toHaveLength(0);
    applyScenario("busy");
    const jobs = mockJobsList();
    expect(jobs.filter((j) => j.status === "running")).toHaveLength(2);
    expect(jobs.filter((j) => j.status === "queued")).toHaveLength(3);
    expect(jobs.filter((j) => j.status === "done")).toHaveLength(1);
  });

  it("errors cria falhas com tipo de erro; heal liga healing e emite heal://state", () => {
    applyScenario("errors");
    expect(
      mockJobsList()
        .filter((j) => j.status === "failed")
        .map((j) => j.errorKind),
    ).toEqual(["network", "unavailable"]);

    resetMockQueue();
    const events: unknown[] = [];
    mockBus.on("heal://state", (p) => events.push(p));
    applyScenario("heal");
    expect(mockQueueState().healing).toBe(true);
    expect(events).toEqual([{ stage: "checking" }]);
  });
});
