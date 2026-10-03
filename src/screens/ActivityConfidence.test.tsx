import { screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { FX3_CLIP } from "@/lib/ipc/mock/fixtures";
import { mockMetadataFor } from "@/lib/ipc/mock/metadata";
import { seedMockJobs } from "@/lib/ipc/mock/queue";
import { initHealStore } from "@/stores/heal";
import { initJobsStore } from "@/stores/jobs";
import { useSettingsStore } from "@/stores/settings";
import { renderApp, resetFlowTests } from "@/testing/flow";

beforeEach(async () => {
  resetFlowTests();
  vi.restoreAllMocks();
  await useSettingsStore.getState().load();
  await initJobsStore();
  await initHealStore();
});

const done = (title: string, over: object) => ({
  sourceUrl: `https://www.youtube.com/watch?v=${title}`,
  title,
  status: "done" as const,
  stage: "done" as const,
  progress: 1,
  overallProgress: 1,
  outputPath: `C:/Musicas/${title}.opus`,
  ...over,
});

describe("Selo de confiança na Atividade (F08)", () => {
  it("mostra a % e a fonte nos concluídos identificados automaticamente", async () => {
    const result = mockMetadataFor(FX3_CLIP, { useOfficial: false });
    seedMockJobs([
      done("faixaA", { confidence: result.confidence, metadataResult: result }),
    ]);
    renderApp("/activity");
    await screen.findByRole("tab", { name: /Concluídos/ });
    (await screen.findByRole("tab", { name: /Concluídos/ })).click();
    const badge = await screen.findByTestId("job-confidence");
    expect(badge).toHaveTextContent("96%");
    expect(badge).toHaveAttribute("title", "Identificado por: iTunes");
  });

  it("os que foram para a revisão trazem o aviso", async () => {
    const result = {
      ...mockMetadataFor(FX3_CLIP, { useOfficial: false }),
      confidence: 0.7,
      bucket: "review" as const,
    };
    seedMockJobs([done("faixaB", { confidence: 0.7, metadataResult: result })]);
    renderApp("/activity");
    (await screen.findByRole("tab", { name: /Concluídos/ })).click();
    expect(await screen.findByTestId("job-confidence")).toHaveTextContent("70% · revisar");
  });

  it("sem confiança, sem decisão ou ainda em andamento não há selo", async () => {
    const none = {
      ...mockMetadataFor(FX3_CLIP, { useOfficial: false }),
      bucket: "none" as const,
    };
    const auto = mockMetadataFor(FX3_CLIP, { useOfficial: true });
    seedMockJobs([
      done("semConfianca", { confidence: null, metadataResult: null }),
      done("semDecisao", { confidence: 0, metadataResult: none }),
      {
        sourceUrl: "https://www.youtube.com/watch?v=rodando",
        title: "rodando",
        status: "running",
        stage: "metadata",
        confidence: 1,
        metadataResult: auto,
      },
    ]);
    renderApp("/activity");
    await screen.findAllByTestId("job-card");
    expect(screen.queryByTestId("job-confidence")).not.toBeInTheDocument();
    (await screen.findByRole("tab", { name: /Concluídos/ })).click();
    await screen.findAllByTestId("job-card");
    expect(screen.queryByTestId("job-confidence")).not.toBeInTheDocument();
  });
});
