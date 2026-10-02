import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { api } from "@/lib/ipc/api";
import { renderApp, resetFlowTests } from "@/testing/flow";
import { useFlowStore } from "@/stores/flow";
import { useJobsStore, initJobsStore } from "@/stores/jobs";
import { useUiStore } from "@/stores/ui";

beforeEach(async () => {
  resetFlowTests();
  vi.restoreAllMocks();
  await initJobsStore();
});

const bar = () => screen.findByRole("textbox", { name: "Barra de comando" });

describe("Barra de comando (T2)", () => {
  it("URL de vídeo: Enter chama analyze e abre o Preview", async () => {
    const analyze = vi.spyOn(api, "analyze");
    renderApp();
    await userEvent.type(await bar(), "https://youtu.be/jNQXAC9IVRw{Enter}");

    expect(await screen.findByTestId("preview")).toBeInTheDocument();
    expect(analyze).toHaveBeenCalledWith("https://www.youtube.com/watch?v=jNQXAC9IVRw");
    expect(screen.getByTestId("preview-title")).toHaveTextContent("Me at the zoo");
    expect(useFlowStore.getState().preview?.id).toBe("jNQXAC9IVRw");
    // A barra se limpa depois de cumprir o papel.
    expect(await bar()).toHaveValue("");
  });

  it("URL de coleção: Enter abre a Coleção", async () => {
    renderApp();
    await userEvent.type(
      await bar(),
      "https://music.youtube.com/playlist?list=OLAK5uy_nmDUsWOMoEcz0SsVqUwir0oxu-k1oUyXE{Enter}",
    );
    await waitFor(() => expect(screen.getByTestId("where")).toHaveTextContent("/collection"));
    expect(
      await screen.findByRole("heading", { level: 1, name: /Whenever You Need Somebody/ }),
    ).toBeInTheDocument();
    expect(useFlowStore.getState().collection?.entries).toHaveLength(10);
  });

  it("texto: chama search e mostra as abas; trocar de aba pesquisa de novo", async () => {
    const search = vi.spyOn(api, "search");
    renderApp();
    await userEvent.type(await bar(), "rick astley never gonna give you up{Enter}");

    expect(await screen.findAllByTestId("search-result")).toHaveLength(3);
    expect(search).toHaveBeenLastCalledWith("ytmusic", "rick astley never gonna give you up");
    expect(screen.getByRole("tab", { name: "YouTube Music", selected: true })).toBeInTheDocument();

    await userEvent.click(screen.getByRole("tab", { name: "YouTube" }));
    await waitFor(() =>
      expect(search).toHaveBeenLastCalledWith("youtube", "rick astley never gonna give you up"),
    );
    // O YouTube comum informa a duração; o YouTube Music não.
    expect(await screen.findByText(/Rick Astley · 3:34/)).toBeInTheDocument();
  });

  it("mostra o estado de carregamento enquanto espera a resposta", async () => {
    let release: (value: Awaited<ReturnType<typeof api.search>>) => void = () => undefined;
    vi.spyOn(api, "search").mockImplementation(() => new Promise((resolve) => (release = resolve)));
    renderApp();
    await userEvent.type(await bar(), "musica{Enter}");
    expect(await screen.findByTestId("command-loading")).toBeInTheDocument();
    release([]);
    expect(await screen.findByText("Nada encontrado")).toBeInTheDocument();
  });

  it("Esc limpa o texto e os resultados", async () => {
    renderApp();
    await userEvent.type(await bar(), "rick astley{Enter}");
    await screen.findAllByTestId("search-result");

    await userEvent.keyboard("{Escape}");
    expect(await bar()).toHaveValue("");
    await waitFor(() => expect(screen.queryByTestId("search-result")).not.toBeInTheDocument());
  });

  it("mostra a dica de tipo em tempo real, sem chamar o backend", async () => {
    const classify = vi.spyOn(api, "urlClassify");
    renderApp();
    await userEvent.type(await bar(), "https://youtu.be/jNQXAC9IVRw");
    expect(screen.getByTestId("command-hint")).toHaveTextContent("Vídeo");
    expect(classify).not.toHaveBeenCalled();
  });

  it("link não suportado e erro do yt-dlp aparecem traduzidos", async () => {
    renderApp();
    await userEvent.type(await bar(), "https://vimeo.com/123{Enter}");
    expect(await screen.findByTestId("command-error")).toHaveTextContent(
      "Esse link não é do YouTube nem do YouTube Music.",
    );

    await userEvent.clear(await bar());
    await userEvent.type(await bar(), "https://youtu.be/unavailable01{Enter}");
    expect(await screen.findByTestId("command-error")).toHaveTextContent(
      "Este vídeo está indisponível.",
    );
  });

  it("resultado: clicar abre o Preview e Baixar cria o job direto", async () => {
    const enqueue = vi.spyOn(api, "enqueue");
    renderApp();
    await userEvent.type(await bar(), "rick astley{Enter}");
    const [first] = await screen.findAllByTestId("search-result");

    await userEvent.click(
      within(first!).getByRole("button", { name: "Baixar Never Gonna Give You Up" }),
    );
    await waitFor(() => expect(enqueue).toHaveBeenCalledTimes(1));
    expect(enqueue.mock.calls[0]![0]).toMatchObject({
      sourceId: "lYBUbBu4W08",
      title: "Never Gonna Give You Up",
      priority: false,
    });
    expect(useUiStore.getState().toasts.at(-1)?.message).toBe("Adicionado à fila");
    expect(Object.values(useJobsStore.getState().jobs)).toHaveLength(1);

    await userEvent.click(
      within(first!).getByRole("button", {
        name: "Abrir a pré-visualização de Never Gonna Give You Up",
      }),
    );
    expect(await screen.findByTestId("preview")).toBeInTheDocument();
  });

  it("Ctrl+K: o overlay também abre o Preview e se fecha", async () => {
    renderApp();
    await userEvent.keyboard("{Control>}k{/Control}");
    const overlay = await screen.findByTestId("command-bar-overlay");
    await userEvent.type(
      within(overlay).getByRole("textbox"),
      "https://youtu.be/jNQXAC9IVRw{Enter}",
    );
    expect(await screen.findByTestId("preview")).toBeInTheDocument();
    await waitFor(() =>
      expect(screen.queryByTestId("command-bar-overlay")).not.toBeInTheDocument(),
    );
  });
});
