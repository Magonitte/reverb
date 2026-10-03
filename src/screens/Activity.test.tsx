import { act, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { ErrorKind } from "@/bindings/ErrorKind";
import { api } from "@/lib/ipc/api";
import { mockBus } from "@/lib/ipc/mock";
import { seedMockJobs } from "@/lib/ipc/mock/queue";
import { applyScenario } from "@/lib/ipc/mock/scenarios";
import ptBR from "@/locales/pt-BR/translation.json";
import { initHealStore } from "@/stores/heal";
import { initJobsStore, useJobsStore } from "@/stores/jobs";
import { useSettingsStore } from "@/stores/settings";
import { renderApp, resetFlowTests } from "@/testing/flow";

beforeEach(async () => {
  resetFlowTests();
  vi.restoreAllMocks();
  await useSettingsStore.getState().load();
  await initJobsStore();
  await initHealStore();
});

const cards = () => screen.queryAllByTestId("job-card");
const tab = (name: RegExp | string) => screen.findByRole("tab", { name });

describe("Atividade (T5)", () => {
  it("mostra contadores, abas e os itens com estágio, velocidade, ETA e tentativas", async () => {
    applyScenario("busy");
    renderApp("/activity");
    expect(await screen.findByText("2 ativos · 3 pendentes · 1 concluídos")).toBeInTheDocument();
    expect(await tab(/Em andamento/)).toHaveTextContent("5");
    expect(await tab(/Concluídos/)).toHaveTextContent("1");
    expect(await tab(/Falhas/)).toHaveTextContent("0");
    expect(cards()).toHaveLength(5);

    const first = cards()[0]!;
    expect(first).toHaveTextContent("Never Gonna Give You Up");
    expect(first).toHaveTextContent("Baixando");
    expect(first).toHaveTextContent("2.4 MB/s");
    expect(first).toHaveTextContent("faltam 0:04");
    expect(within(first).getByRole("progressbar")).toHaveAttribute("aria-valuenow", "28");

    // Só os pendentes têm alça de arrastar.
    expect(screen.getAllByRole("button", { name: /^Reordenar / })).toHaveLength(3);
  });

  it("mostra a tentativa atual quando já houve nova tentativa", async () => {
    seedMockJobs([
      {
        sourceUrl: "https://www.youtube.com/watch?v=abc",
        title: "Faixa",
        status: "running",
        stage: "downloading",
        attempts: 2,
      },
    ]);
    renderApp("/activity");
    expect(await screen.findByTestId("job-card")).toHaveTextContent("Tentativa 2/3");
  });

  it("falhas: erro traduzido, tentar de novo e remover", async () => {
    applyScenario("errors");
    renderApp("/activity");
    await userEvent.click(await tab(/Falhas/));
    const items = cards();
    expect(items).toHaveLength(2);
    expect(within(items[0]!).getByTestId("job-error")).toHaveTextContent(
      "Falha de rede. Verifique a conexão.",
    );
    expect(within(items[1]!).getByTestId("job-error")).toHaveTextContent(
      "Este vídeo está indisponível.",
    );

    const retry = vi.spyOn(api, "jobRetry");
    await userEvent.click(within(items[0]!).getByRole("button", { name: /^Tentar novamente / }));
    await waitFor(() => expect(retry).toHaveBeenCalledTimes(1));
    await waitFor(() => expect(cards()).toHaveLength(1));
    expect(await tab(/Em andamento/)).toHaveTextContent("1");

    const remove = vi.spyOn(api, "jobRemove");
    await userEvent.click(screen.getByRole("button", { name: /^Remover / }));
    await waitFor(() => expect(remove).toHaveBeenCalledTimes(1));
    await waitFor(() => expect(cards()).toHaveLength(0));
  });

  it("cancelar um item em andamento o move para Falhas como cancelado", async () => {
    applyScenario("busy");
    renderApp("/activity");
    const cancel = vi.spyOn(api, "jobCancel");
    await userEvent.click(
      (await screen.findAllByRole("button", { name: /^Cancelar Never Gonna Give You Up/ }))[0]!,
    );
    await waitFor(() => expect(cancel).toHaveBeenCalledTimes(1));
    await userEvent.click(await tab(/Falhas/));
    expect(cards()).toHaveLength(1);
    expect(cards()[0]).toHaveTextContent("Cancelado");
  });

  it.each<ErrorKind>([
    "cancelled",
    "unavailable",
    "age_restricted",
    "bot_check",
    "geo_blocked",
    "disk",
    "ffmpeg",
    "extractor",
    "network",
    "unknown",
  ])("traduz o erro %s", async (kind) => {
    seedMockJobs([
      {
        sourceUrl: "https://www.youtube.com/watch?v=abc",
        title: "Faixa",
        status: "failed",
        errorKind: kind,
        errorMessage: "mensagem crua do yt-dlp",
      },
    ]);
    renderApp("/activity");
    await userEvent.click(await tab(/Falhas/));
    const text = (ptBR.errors as Record<string, string>)[kind]!;
    expect(text).toBeTruthy();
    expect(await screen.findByTestId("job-error")).toHaveTextContent(text);
    expect(screen.getByTestId("job-error")).not.toHaveTextContent("mensagem crua");
  });

  it("usa a chave i18n quando o backend a manda em errorMessage", async () => {
    seedMockJobs([
      {
        sourceUrl: "https://www.youtube.com/watch?v=abc",
        title: "Faixa",
        status: "failed",
        errorKind: "extractor",
        errorMessage: "errors.extractorPersistent",
      },
    ]);
    renderApp("/activity");
    await userEvent.click(await tab(/Falhas/));
    expect(await screen.findByTestId("job-error")).toHaveTextContent(
      "O YouTube mudou e nem o yt-dlp mais recente conseguiu acompanhar.",
    );
  });

  it("concluídos: abrir pasta chama library_reveal com o caminho do arquivo", async () => {
    seedMockJobs([
      {
        sourceUrl: "https://www.youtube.com/watch?v=abc",
        title: "Faixa pronta",
        status: "done",
        stage: "done",
        overallProgress: 1,
        outputPath: "C:/Musicas/Reverb/Faixa pronta.opus",
      },
    ]);
    renderApp("/activity");
    await userEvent.click(await tab(/Concluídos/));
    const reveal = vi.spyOn(api, "libraryReveal");
    await userEvent.click(
      await screen.findByRole("button", { name: "Mostrar Faixa pronta na pasta" }),
    );
    expect(reveal).toHaveBeenCalledWith("C:/Musicas/Reverb/Faixa pronta.opus");
  });

  it("banner de autocura aparece com heal://state e some ao terminar", async () => {
    renderApp("/activity");
    await screen.findByRole("heading", { level: 1, name: "Atividade" });
    expect(screen.queryByTestId("heal-banner")).not.toBeInTheDocument();
    act(() => mockBus.emit("heal://state", { stage: "checking" }));
    expect(await screen.findByTestId("heal-banner")).toHaveTextContent(
      "O YouTube mudou algo — atualizando o yt-dlp…",
    );
    act(() => mockBus.emit("heal://state", { stage: "done" }));
    await waitFor(() => expect(screen.queryByTestId("heal-banner")).not.toBeInTheDocument());
  });

  it("pausar/retomar a fila", async () => {
    applyScenario("busy");
    renderApp("/activity");
    await userEvent.click(await screen.findByRole("button", { name: "Pausar fila" }));
    await waitFor(() => expect(useJobsStore.getState().queue.paused).toBe(true));
    expect(await screen.findByTestId("queue-paused")).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "Retomar fila" }));
    await waitFor(() => expect(useJobsStore.getState().queue.paused).toBe(false));
  });

  it("limpar concluídos e cancelar todos (com confirmação)", async () => {
    applyScenario("busy");
    renderApp("/activity");
    await userEvent.click(await screen.findByRole("button", { name: "Limpar concluídos" }));
    await waitFor(() =>
      expect(Object.values(useJobsStore.getState().jobs).some((j) => j.status === "done")).toBe(
        false,
      ),
    );

    await userEvent.click(screen.getByRole("button", { name: "Cancelar todos" }));
    const dialog = await screen.findByRole("dialog", { name: "Cancelar todos os downloads?" });
    await userEvent.click(within(dialog).getByRole("button", { name: "Cancelar todos" }));
    await waitFor(() =>
      expect(
        Object.values(useJobsStore.getState().jobs).every((j) => j.status === "cancelled"),
      ).toBe(true),
    );
  });

  it("avisa quando a fila passa de 90% do limite", async () => {
    act(() =>
      useSettingsStore.setState({
        settings: { ...useSettingsStore.getState().settings!, queueLimit: 10 },
      }),
    );
    seedMockJobs(
      Array.from({ length: 9 }, (_, i) => ({
        sourceUrl: `https://www.youtube.com/watch?v=q${i}`,
        title: `Faixa ${i}`,
      })),
    );
    renderApp("/activity");
    expect(await screen.findByTestId("queue-limit-warning")).toHaveTextContent(
      "A fila está quase cheia (9/10).",
    );
  });
});
