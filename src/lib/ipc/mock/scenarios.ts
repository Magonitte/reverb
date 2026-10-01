import type { Job } from "@/bindings/Job";
import { initialUpdaterState, useUpdaterStore } from "@/stores/updater";
import { mockBus } from "./bus";
import { FX1_VIDEO, FX2_MUSIC, FX3_CLIP, FX4_ALBUM } from "./fixtures";
import { seedMockJobs, setMockHealing, startMockSimulation } from "./queue";
import { seedMockTool } from "./tools";
import { MOCK_UPDATE, setMockUpdaterMode } from "./updater";

export const SCENARIOS = [
  "empty",
  "busy",
  "errors",
  "heal",
  "update-available",
  "update-downloading",
  "update-error",
] as const;
export type Scenario = (typeof SCENARIOS)[number];

/** Lê `?scenario=` da URL (o HashRouter deixa a query string antes do `#`). */
export function scenarioFromLocation(search = globalThis.location?.search ?? ""): Scenario | null {
  const value = new URLSearchParams(search).get("scenario");
  return (SCENARIOS as readonly string[]).includes(value ?? "") ? (value as Scenario) : null;
}

const fromVideo = (v: typeof FX1_VIDEO): Partial<Job> & { sourceUrl: string } => ({
  sourceUrl: v.webpageUrl ?? "",
  sourceId: v.id,
  title: v.title,
  artist: v.artist ?? v.channel,
  thumbnail: v.thumbnail,
  durationS: v.duration,
});

/** Estado estático e determinístico para os testes visuais e E2E. */
export function applyScenario(name: Scenario): void {
  switch (name) {
    case "empty":
      return;
    case "busy": {
      const album = FX4_ALBUM.entries.map((e) => ({
        sourceUrl: `https://www.youtube.com/watch?v=${e.id}`,
        sourceId: e.id,
        title: e.title,
        artist: FX4_ALBUM.channel,
        durationS: e.duration,
      }));
      seedMockJobs([
        {
          ...fromVideo(FX2_MUSIC),
          status: "running",
          stage: "downloading",
          progress: 0.42,
          overallProgress: 0.28,
          speedBps: 2_400_000,
          etaS: 4,
          attempts: 1,
        },
        {
          ...fromVideo(FX3_CLIP),
          status: "running",
          stage: "converting",
          progress: 0.6,
          overallProgress: 0.52,
          attempts: 1,
        },
        ...album.map((a) => ({ ...a, status: "queued" as const })),
        { ...fromVideo(FX1_VIDEO), status: "done", stage: "done", progress: 1, overallProgress: 1 },
      ]);
      return;
    }
    case "errors":
      seedMockJobs([
        {
          ...fromVideo(FX1_VIDEO),
          status: "failed",
          errorKind: "network",
          errorMessage: "Falha de rede ao baixar",
          attempts: 3,
        },
        {
          ...fromVideo(FX3_CLIP),
          status: "failed",
          errorKind: "unavailable",
          errorMessage: "Vídeo indisponível",
          attempts: 1,
        },
      ]);
      return;
    case "heal":
      seedMockJobs([{ ...fromVideo(FX2_MUSIC), status: "queued" }]);
      setMockHealing(true);
      mockBus.emit("heal://state", { stage: "checking" });
      return;
    case "update-available":
      for (const tool of ["ytdlp", "deno", "ffmpeg"] as const) seedMockTool(tool, false);
      mockBus.emit("tools://changed", { tool: "ytdlp", version: "2026.10.01" });
      setMockUpdaterMode("available");
      mockBus.emit("updater://available", MOCK_UPDATE);
      return;
    case "update-downloading":
      for (const tool of ["ytdlp", "deno", "ffmpeg"] as const) seedMockTool(tool, false);
      mockBus.emit("tools://changed", { tool: "ytdlp", version: "2026.10.01" });
      setMockUpdaterMode("available");
      useUpdaterStore.setState({
        ...initialUpdaterState,
        phase: "downloading",
        available: true,
        version: MOCK_UPDATE.version,
        currentVersion: MOCK_UPDATE.currentVersion,
        notes: MOCK_UPDATE.notes,
        date: MOCK_UPDATE.date,
        downloaded: 40,
        total: 100,
      });
      return;
    case "update-error":
      setMockUpdaterMode("error");
      useUpdaterStore.setState({
        ...initialUpdaterState,
        phase: "error",
        error: "Sem conexão com o GitHub",
      });
      return;
  }
}

/** Ponto de entrada do backend falso no navegador: cenário da URL ou simulação contínua. */
export function startMock(search?: string): void {
  const scenario = scenarioFromLocation(search);
  if (scenario) applyScenario(scenario);
  else startMockSimulation();
}
