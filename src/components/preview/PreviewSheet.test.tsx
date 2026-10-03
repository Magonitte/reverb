import { act, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { api } from "@/lib/ipc/api";
import { FX1_VIDEO, FX2_MUSIC } from "@/lib/ipc/mock/fixtures";
import { mockEnqueue } from "@/lib/ipc/mock/queue";
import { initJobsStore, useJobsStore } from "@/stores/jobs";
import { useFlowStore } from "@/stores/flow";
import { useSettingsStore } from "@/stores/settings";
import { useUiStore } from "@/stores/ui";
import { renderApp, resetFlowTests } from "@/testing/flow";

beforeEach(async () => {
  resetFlowTests();
  vi.restoreAllMocks();
  await initJobsStore();
});

async function openPreview(info = FX1_VIDEO) {
  await useSettingsStore.getState().load();
  renderApp();
  await screen.findByRole("heading", { level: 1, name: "Início" });
  act(() => useFlowStore.getState().openPreview(info));
  return screen.findByRole("dialog", { name: "Pré-visualização" });
}

describe("Preview (T3)", () => {
  it("mostra os dados de FX1 e o selo de qualidade da fonte", async () => {
    const dialog = await openPreview(FX1_VIDEO);
    expect(within(dialog).getByTestId("preview-title")).toHaveTextContent("Me at the zoo");
    expect(within(dialog).getByText(/jawed · 0:19/)).toBeInTheDocument();
    expect(within(dialog).getByTestId("source-quality")).toHaveTextContent("Fonte: Opus 129 kbps");
    expect(within(dialog).getByText("Outro")).toBeInTheDocument();
  });

  it("FX2 é uma faixa de música com artista", async () => {
    const dialog = await openPreview(FX2_MUSIC);
    expect(within(dialog).getByTestId("preview-title")).toHaveTextContent(
      "Never Gonna Give You Up",
    );
    expect(within(dialog).getByText("Música")).toBeInTheDocument();
    expect(within(dialog).getByText(/Rick Astley · 3:34/)).toBeInTheDocument();
  });

  it("avisa que MP3 320 recodifica e o aviso some em Original", async () => {
    const dialog = await openPreview();
    expect(within(dialog).queryByRole("note")).not.toBeInTheDocument();

    await userEvent.click(within(dialog).getByRole("button", { name: "MP3 320 kbps" }));
    expect(within(dialog).getByRole("note")).toHaveTextContent(
      "Recodifica o áudio e pode perder qualidade.",
    );

    await userEvent.click(within(dialog).getByRole("button", { name: /Original/ }));
    expect(within(dialog).queryByRole("note")).not.toBeInTheDocument();
  });

  it("Baixar agora envia priority: true; Adicionar à fila envia false", async () => {
    const enqueue = vi.spyOn(api, "enqueue");
    let dialog = await openPreview();
    await userEvent.click(within(dialog).getByRole("button", { name: "Baixar agora" }));
    await waitFor(() => expect(enqueue).toHaveBeenCalledTimes(1));
    expect(enqueue.mock.calls[0]![0]).toMatchObject({
      sourceId: "jNQXAC9IVRw",
      title: "Me at the zoo",
      profileId: "original",
      priority: true,
      allowDuplicate: false,
    });
    expect(useUiStore.getState().toasts.at(-1)?.message).toBe("Adicionado à fila");
    await waitFor(() => expect(screen.queryByTestId("preview")).not.toBeInTheDocument());

    dialog = await (async () => {
      act(() => useFlowStore.getState().openPreview(FX2_MUSIC));
      return screen.findByRole("dialog", { name: "Pré-visualização" });
    })();
    await userEvent.click(within(dialog).getByRole("button", { name: "Adicionar à fila" }));
    await waitFor(() => expect(enqueue).toHaveBeenCalledTimes(2));
    expect(enqueue.mock.calls[1]![0]).toMatchObject({ sourceId: "lYBUbBu4W08", priority: false });
    expect(Object.values(useJobsStore.getState().jobs)).toHaveLength(2);
  });

  it("duplicata: mostra o selo e pede confirmação antes de baixar de novo", async () => {
    mockEnqueue({
      url: FX1_VIDEO.webpageUrl!,
      sourceId: FX1_VIDEO.id,
      metadataOverride: null,
      priority: false,
      allowDuplicate: false,
    });
    const enqueue = vi.spyOn(api, "enqueue");
    const dialog = await openPreview(FX1_VIDEO);
    expect(await within(dialog).findByText("Já baixada")).toBeInTheDocument();

    await userEvent.click(within(dialog).getByRole("button", { name: "Adicionar à fila" }));
    const confirm = await screen.findByRole("dialog", { name: "Baixar novamente?" });
    expect(enqueue).not.toHaveBeenCalled();

    // Cancelar mantém o Preview aberto e não enfileira nada.
    await userEvent.click(within(confirm).getByRole("button", { name: "Cancelar" }));
    expect(screen.queryByRole("dialog", { name: "Baixar novamente?" })).not.toBeInTheDocument();
    expect(screen.getByTestId("preview")).toBeInTheDocument();
    expect(enqueue).not.toHaveBeenCalled();

    await userEvent.click(within(dialog).getByRole("button", { name: "Baixar agora" }));
    await userEvent.click(
      within(await screen.findByRole("dialog", { name: "Baixar novamente?" })).getByRole("button", {
        name: "Baixar novamente",
      }),
    );
    await waitFor(() => expect(enqueue).toHaveBeenCalledTimes(1));
    expect(enqueue.mock.calls[0]![0]).toMatchObject({ allowDuplicate: true, priority: true });
  });

  it("pasta de destino: escolher usa o seletor e vai em options.outputDir", async () => {
    const enqueue = vi.spyOn(api, "enqueue");
    const dialog = await openPreview();
    expect(within(dialog).getByTestId("preview-folder-path")).toHaveTextContent(
      "Pasta padrão (Músicas/Reverb)",
    );
    await userEvent.click(within(dialog).getByRole("button", { name: "Escolher…" }));
    await waitFor(() =>
      expect(within(dialog).getByTestId("preview-folder-path")).toHaveTextContent(
        "C:/Musicas/Reverb",
      ),
    );
    await userEvent.click(within(dialog).getByRole("button", { name: "Adicionar à fila" }));
    await waitFor(() => expect(enqueue).toHaveBeenCalled());
    expect(enqueue.mock.calls[0]![0].options).toMatchObject({ outputDir: "C:/Musicas/Reverb" });
  });

  it("opções por job só viajam quando diferem das configurações", async () => {
    const enqueue = vi.spyOn(api, "enqueue");
    const dialog = await openPreview();
    const lyrics = within(dialog).getByRole("switch", { name: "Buscar letra" });
    expect(lyrics).toBeChecked();
    await userEvent.click(lyrics);
    expect(lyrics).not.toBeChecked();

    await userEvent.click(within(dialog).getByRole("button", { name: "Adicionar à fila" }));
    await waitFor(() => expect(enqueue).toHaveBeenCalled());
    expect(enqueue.mock.calls[0]![0].options).toEqual({ fetchLyrics: false });
  });

  it("Esc fecha o painel", async () => {
    await openPreview();
    await userEvent.keyboard("{Escape}");
    await waitFor(() => expect(screen.queryByTestId("preview")).not.toBeInTheDocument());
    expect(useFlowStore.getState().preview).toBeNull();
  });
});
