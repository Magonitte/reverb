import { act, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { VideoInfo } from "@/bindings/VideoInfo";
import { api } from "@/lib/ipc/api";
import { FX1_VIDEO, FX2_MUSIC, FX3_CLIP } from "@/lib/ipc/mock/fixtures";
import { initJobsStore, useJobsStore } from "@/stores/jobs";
import { useFlowStore } from "@/stores/flow";
import { useSettingsStore } from "@/stores/settings";
import { renderApp, resetFlowTests } from "@/testing/flow";

const OFFICIAL_URL = "https://music.youtube.com/watch?v=lYBUbBu4W08";
const SWITCH = "Baixar a versão oficial";
const PREVIEW_BUTTON = "Pré-visualizar metadados";

beforeEach(async () => {
  resetFlowTests();
  vi.restoreAllMocks();
  await initJobsStore();
});

async function openPreview(info: VideoInfo) {
  await useSettingsStore.getState().load();
  renderApp();
  await screen.findByRole("heading", { level: 1, name: "Início" });
  act(() => useFlowStore.getState().openPreview(info));
  return screen.findByRole("dialog", { name: "Pré-visualização" });
}

describe("Versão oficial no Preview (F08/T11)", () => {
  it("o clipe FX3 mostra o card da versão oficial com o toggle pré-marcado", async () => {
    const dialog = await openPreview(FX3_CLIP);
    const card = await within(dialog).findByTestId("official-card");
    expect(card).toHaveTextContent("Versão oficial disponível");
    expect(within(card).getByTestId("official-track")).toHaveTextContent(
      "Never Gonna Give You Up · Rick Astley",
    );
    expect(within(card).getByTestId("official-album")).toHaveTextContent(
      "Whenever You Need Somebody",
    );
    expect(card).toHaveTextContent("Áudio de estúdio, metadados completos.");
    expect(within(card).getByRole("switch", { name: SWITCH })).toBeChecked();
  });

  it("FX1 (não é música) e FX2 (já é a faixa oficial) não procuram nem mostram o card", async () => {
    const find = vi.spyOn(api, "findOfficialVersion");
    let dialog = await openPreview(FX2_MUSIC);
    expect(within(dialog).queryByTestId("official-card")).not.toBeInTheDocument();
    await userEvent.keyboard("{Escape}");
    await waitFor(() => expect(screen.queryByTestId("preview")).not.toBeInTheDocument());

    act(() => useFlowStore.getState().openPreview(FX1_VIDEO));
    dialog = await screen.findByRole("dialog", { name: "Pré-visualização" });
    expect(within(dialog).queryByTestId("official-card")).not.toBeInTheDocument();
    expect(find).not.toHaveBeenCalled();
  });

  it("modo offline não procura a versão oficial", async () => {
    await useSettingsStore.getState().load();
    await useSettingsStore.getState().update({ offlineMode: true });
    const find = vi.spyOn(api, "findOfficialVersion");
    const dialog = await openPreview(FX3_CLIP);
    expect(within(dialog).queryByTestId("official-card")).not.toBeInTheDocument();
    expect(find).not.toHaveBeenCalled();
  });

  it("o toggle altera a URL e o id enviados no enqueue", async () => {
    const enqueue = vi.spyOn(api, "enqueue");
    const dialog = await openPreview(FX3_CLIP);
    await within(dialog).findByRole("switch", { name: SWITCH });

    // Marcado (padrão): baixa a faixa oficial.
    await userEvent.click(within(dialog).getByRole("button", { name: "Adicionar à fila" }));
    await waitFor(() => expect(enqueue).toHaveBeenCalledTimes(1));
    expect(enqueue.mock.calls[0]![0]).toMatchObject({
      url: OFFICIAL_URL,
      sourceId: "lYBUbBu4W08",
      title: "Never Gonna Give You Up",
      metadataOverride: null,
    });
    await waitFor(() => expect(screen.queryByTestId("preview")).not.toBeInTheDocument());

    // Desmarcado: fica o clipe.
    act(() => useFlowStore.getState().openPreview(FX3_CLIP));
    const second = await screen.findByRole("dialog", { name: "Pré-visualização" });
    await userEvent.click(await within(second).findByRole("switch", { name: SWITCH }));
    await userEvent.click(within(second).getByRole("button", { name: "Adicionar à fila" }));
    await waitFor(() => expect(enqueue).toHaveBeenCalledTimes(2));
    expect(enqueue.mock.calls[1]![0]).toMatchObject({
      url: FX3_CLIP.webpageUrl,
      sourceId: "dQw4w9WgXcQ",
    });
  });

  it("preferOfficialAudio desligado deixa o toggle desmarcado", async () => {
    await useSettingsStore.getState().load();
    await useSettingsStore.getState().update({ preferOfficialAudio: false });
    const dialog = await openPreview(FX3_CLIP);
    const toggle = await within(dialog).findByRole("switch", { name: SWITCH });
    expect(toggle).not.toBeChecked();
  });

  it("a duplicata é checada pelo id da fonte escolhida", async () => {
    const check = vi.spyOn(api, "checkDuplicates");
    const dialog = await openPreview(FX3_CLIP);
    await within(dialog).findByTestId("official-card");
    await waitFor(() => expect(check).toHaveBeenLastCalledWith(["lYBUbBu4W08"], "original"));
    await userEvent.click(within(dialog).getByRole("switch", { name: SWITCH }));
    await waitFor(() => expect(check).toHaveBeenLastCalledWith(["dQw4w9WgXcQ"], "original"));
  });
});

describe("Pré-visualização de metadados no Preview (F08/T11)", () => {
  it("mostra campos, % de confiança e fonte", async () => {
    const dialog = await openPreview(FX3_CLIP);
    await within(dialog).findByTestId("official-card");
    expect(within(dialog).queryByTestId("metadata-panel")).not.toBeInTheDocument();
    await userEvent.click(within(dialog).getByRole("button", { name: PREVIEW_BUTTON }));
    const panel = await within(dialog).findByTestId("metadata-panel");
    expect(within(panel).getByTestId("metadata-confidence")).toHaveTextContent(
      "100% de confiança",
    );
    expect(within(panel).getByTestId("metadata-source")).toHaveTextContent(
      "Faixa oficial do YouTube Music",
    );
    expect(within(panel).getByTestId("metadata-bucket")).toHaveTextContent(
      "Aplicado automaticamente.",
    );
    expect(within(panel).getByLabelText("Título")).toHaveValue("Never Gonna Give You Up");
    expect(within(panel).getByLabelText("Artista")).toHaveValue("Rick Astley");
    expect(within(panel).getByLabelText("Álbum")).toHaveValue("Whenever You Need Somebody");
    expect(within(panel).getByLabelText("Ano")).toHaveValue("1987");
    expect(within(panel).queryByTestId("metadata-edited")).not.toBeInTheDocument();
  });

  it("editar um campo vira metadata_override no enqueue", async () => {
    const enqueue = vi.spyOn(api, "enqueue");
    const dialog = await openPreview(FX3_CLIP);
    await within(dialog).findByTestId("official-card");
    await userEvent.click(within(dialog).getByRole("button", { name: PREVIEW_BUTTON }));
    const panel = await within(dialog).findByTestId("metadata-panel");

    const artist = within(panel).getByLabelText("Artista");
    await userEvent.clear(artist);
    await userEvent.type(artist, "Outro artista");
    const year = within(panel).getByLabelText("Ano");
    await userEvent.clear(year);
    await userEvent.type(year, "1988");
    expect(within(panel).getByTestId("metadata-edited")).toBeInTheDocument();

    await userEvent.click(within(dialog).getByRole("button", { name: "Adicionar à fila" }));
    await waitFor(() => expect(enqueue).toHaveBeenCalledTimes(1));
    expect(enqueue.mock.calls[0]![0]).toMatchObject({
      url: OFFICIAL_URL,
      metadataOverride: { artist: "Outro artista", year: 1988 },
    });
    // O job criado leva a edição.
    const job = Object.values(useJobsStore.getState().jobs)[0]!;
    expect(job.metadataOverride).toEqual({ artist: "Outro artista", year: 1988 });
  });

  it("sem editar nada o override é nulo, mesmo depois de pré-visualizar", async () => {
    const enqueue = vi.spyOn(api, "enqueue");
    const dialog = await openPreview(FX3_CLIP);
    await within(dialog).findByTestId("official-card");
    await userEvent.click(within(dialog).getByRole("button", { name: PREVIEW_BUTTON }));
    await within(dialog).findByTestId("metadata-panel");
    await userEvent.click(within(dialog).getByRole("button", { name: "Adicionar à fila" }));
    await waitFor(() => expect(enqueue).toHaveBeenCalled());
    expect(enqueue.mock.calls[0]![0].metadataOverride).toBeNull();
  });

  it("trocar a versão oficial refaz a pré-visualização e descarta as edições", async () => {
    const dialog = await openPreview(FX3_CLIP);
    await within(dialog).findByTestId("official-card");
    await userEvent.click(within(dialog).getByRole("button", { name: PREVIEW_BUTTON }));
    const panel = await within(dialog).findByTestId("metadata-panel");
    const artist = within(panel).getByLabelText("Artista");
    await userEvent.clear(artist);
    await userEvent.type(artist, "Outro");
    expect(within(panel).getByTestId("metadata-edited")).toBeInTheDocument();

    await userEvent.click(within(dialog).getByRole("switch", { name: SWITCH }));
    await waitFor(() =>
      expect(within(dialog).getByTestId("metadata-source")).toHaveTextContent("iTunes"),
    );
    expect(within(dialog).queryByTestId("metadata-edited")).not.toBeInTheDocument();
    expect(within(dialog).getByLabelText("Artista")).toHaveValue("Rick Astley");
  });

  it("FX1 mostra que não é música no painel", async () => {
    const dialog = await openPreview(FX1_VIDEO);
    await userEvent.click(within(dialog).getByRole("button", { name: PREVIEW_BUTTON }));
    const panel = await within(dialog).findByTestId("metadata-panel");
    expect(within(panel).getByTestId("metadata-bucket")).toHaveTextContent("Não parece música");
    expect(within(panel).queryByTestId("metadata-confidence")).not.toBeInTheDocument();
    expect(within(panel).getByLabelText("Título")).toHaveValue("Me at the zoo");
  });

  it("erro da pré-visualização aparece como alerta", async () => {
    vi.spyOn(api, "metadataPreview").mockRejectedValueOnce({
      kind: "network",
      message: "sem rede",
    });
    const dialog = await openPreview(FX3_CLIP);
    await userEvent.click(within(dialog).getByRole("button", { name: PREVIEW_BUTTON }));
    expect(await within(dialog).findByRole("alert")).toBeInTheDocument();
    expect(within(dialog).queryByTestId("metadata-panel")).not.toBeInTheDocument();
  });
});
