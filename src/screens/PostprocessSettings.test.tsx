import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { api } from "@/lib/ipc/api";
import { mockCalls } from "@/lib/ipc/mock/media";
import { seedMockJobs } from "@/lib/ipc/mock/queue";
import { initJobsStore } from "@/stores/jobs";
import { useSettingsStore } from "@/stores/settings";
import { renderApp, resetFlowTests } from "@/testing/flow";

beforeEach(async () => {
  vi.restoreAllMocks();
  resetFlowTests();
  await useSettingsStore.getState().load();
  await initJobsStore();
});

describe("F09: nomes, metadados e concluídos", () => {
  it("atualiza a prévia e salva apenas um modelo válido", async () => {
    const user = userEvent.setup();
    renderApp("/settings/downloads");
    expect(await screen.findByTestId("template-preview")).toBeInTheDocument();
    await waitFor(() => expect(screen.getByTestId("template-preview")).toHaveTextContent("01 - Never Gonna Give You Up.opus"));
    const input = screen.getByLabelText("Modelo de nomes de arquivos");
    await user.clear(input);
    await user.paste("{artist}/{title}");
    await waitFor(() => expect(screen.getByTestId("template-preview")).toHaveTextContent("Rick Astley/Never Gonna Give You Up.opus"));
    await user.click(screen.getByRole("button", { name: "Salvar" }));
    await waitFor(() => expect(useSettingsStore.getState().settings?.fileTemplate).toBe("{artist}/{title}"));
    await user.clear(input);
    await user.paste("{unknown}");
    expect(await screen.findByText("Confira as variáveis e as chaves do modelo.")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Salvar" })).toBeDisabled();
  });

  it("grava os toggles de letras e ReplayGain", async () => {
    const user = userEvent.setup();
    renderApp("/settings/metadata");
    await user.click(await screen.findByRole("switch", { name: "Buscar letras no LRCLIB" }));
    await user.click(screen.getByRole("switch", { name: "Analisar ReplayGain" }));
    await waitFor(() => expect(useSettingsStore.getState().settings).toMatchObject({ fetchLyrics: false, normalizeVolume: false }));
  });

  it("mostra a capa embutida e avisos e abre o arquivo e sua pasta", async () => {
    const user = userEvent.setup();
    seedMockJobs([{ sourceUrl: "https://youtu.be/lYBUbBu4W08", title: "Faixa", status: "done", stage: "done",
      outputPath: "C:/Musicas/Faixa.opus", libraryId: 7, warnings: ["warnings.lyrics"] }]);
    renderApp("/activity");
    await user.click(await screen.findByRole("tab", { name: /Concluídos/ }));
    expect(await screen.findByTestId("job-cover")).toHaveAttribute("src", expect.stringMatching(/^data:image/));
    expect(screen.getByTestId("job-warnings")).toHaveTextContent("Não foi possível buscar a letra.");
    await user.click(screen.getByRole("button", { name: "Abrir arquivo" }));
    await user.click(screen.getByRole("button", { name: "Mostrar Faixa na pasta" }));
    expect(mockCalls).toContainEqual({ cmd: "library_open_file", args: "C:/Musicas/Faixa.opus" });
    expect(mockCalls.some((call) => call.cmd === "library_reveal")).toBe(true);
  });

  it("ignora uma resposta de prévia que chegou depois da edição seguinte", async () => {
    const user = userEvent.setup();
    let resolveOld!: (value: string) => void;
    vi.spyOn(api, "templatePreview").mockImplementationOnce(() => new Promise((resolve) => { resolveOld = resolve; }))
      .mockResolvedValue("novo.opus");
    renderApp("/settings/downloads");
    await waitFor(() => expect(resolveOld).toBeDefined());
    const input = screen.getByLabelText("Modelo de nomes de arquivos");
    await user.clear(input);
    await user.paste("{title}");
    await waitFor(() => expect(screen.getByTestId("template-preview")).toHaveTextContent("novo.opus"));
    resolveOld("velho.opus");
    await waitFor(() => expect(screen.getByTestId("template-preview")).toHaveTextContent("novo.opus"));
  });
});
