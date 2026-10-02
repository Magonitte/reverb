import { act, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { api } from "@/lib/ipc/api";
import { FX4_ALBUM } from "@/lib/ipc/mock/fixtures";
import { mockEnqueue } from "@/lib/ipc/mock/queue";
import { useFlowStore } from "@/stores/flow";
import { initJobsStore } from "@/stores/jobs";
import { useSettingsStore } from "@/stores/settings";
import { renderApp, resetFlowTests, stubLayout } from "@/testing/flow";

let restoreLayout: () => void;

beforeEach(async () => {
  resetFlowTests();
  vi.restoreAllMocks();
  restoreLayout = stubLayout();
  await useSettingsStore.getState().load();
  await initJobsStore();
  useFlowStore.getState().openCollection(FX4_ALBUM);
});

afterEach(() => restoreLayout());

const track = (title: string) => screen.findByRole("checkbox", { name: `Selecionar ${title}` });
const download = () => screen.getByRole("button", { name: /^Baixar selecionadas/ });

describe("Coleção (T4)", () => {
  it("mostra o cabeçalho e lista as 10 faixas", async () => {
    renderApp("/collection");
    expect(
      await screen.findByRole("heading", { level: 1, name: /Whenever You Need Somebody/ }),
    ).toBeInTheDocument();
    expect(screen.getByText(/Rick Astley · 10 faixas · /)).toBeInTheDocument();
    expect(screen.getByRole("table", { name: "Faixas da coleção" })).toHaveAttribute(
      "aria-rowcount",
      "10",
    );
    expect(await track("Never Gonna Give You Up")).not.toBeChecked();
    expect(download()).toBeDisabled();
  });

  it("seleciona 3 de 10, o botão mostra (3) e enfileira com o índice certo", async () => {
    const enqueue = vi.spyOn(api, "enqueue");
    renderApp("/collection");
    await userEvent.click(await track("Never Gonna Give You Up"));
    await userEvent.click(await track("Together Forever"));
    await userEvent.click(await track("The Love Has Gone"));
    expect(download()).toHaveTextContent("Baixar selecionadas (3)");

    await userEvent.click(download());
    await waitFor(() => expect(enqueue).toHaveBeenCalledTimes(3));
    const requests = enqueue.mock.calls.map(([r]) => r);
    expect(requests.map((r) => r.sourceId)).toEqual(["lYBUbBu4W08", "i_Q88T1HI_w", "fx4-track-5"]);
    expect(requests.map((r) => r.playlistCtx?.index)).toEqual([1, 3, 5]);
    expect(requests[0]!.playlistCtx).toMatchObject({
      playlistTitle: "Album - Whenever You Need Somebody",
      playlistId: FX4_ALBUM.id,
    });
    expect(requests.every((r) => r.priority === false)).toBe(true);
    // A seleção é limpa depois de enfileirar.
    await waitFor(() => expect(download()).toBeDisabled());
  });

  it("Selecionar só as novas ignora o que já foi baixado", async () => {
    mockEnqueue({
      url: "https://www.youtube.com/watch?v=raBobo3GZYA",
      sourceId: "raBobo3GZYA",
      metadataOverride: null,
      priority: false,
      allowDuplicate: false,
    });
    renderApp("/collection");
    expect(await screen.findByText("Já baixada")).toBeInTheDocument();

    await userEvent.click(screen.getByRole("button", { name: "Selecionar só as novas" }));
    expect(download()).toHaveTextContent("Baixar selecionadas (9)");
    expect(await track("Whenever You Need Somebody")).not.toBeChecked();
    expect(await track("Never Gonna Give You Up")).toBeChecked();
  });

  it("Selecionar tudo / Limpar seleção", async () => {
    renderApp("/collection");
    await userEvent.click(await screen.findByRole("button", { name: "Selecionar tudo" }));
    expect(download()).toHaveTextContent("(10)");
    await userEvent.click(screen.getByRole("button", { name: "Limpar seleção" }));
    expect(download()).toBeDisabled();
  });

  it("o filtro por texto reduz a lista", async () => {
    renderApp("/collection");
    await userEvent.type(
      await screen.findByRole("textbox", { name: "Filtrar faixas" }),
      "together",
    );
    const table = screen.getByRole("table", { name: "Faixas da coleção" });
    expect(table).toHaveAttribute("aria-rowcount", "1");
    expect(within(table).getByText("Together Forever")).toBeInTheDocument();
    expect(within(table).queryByText("Slipping Away")).not.toBeInTheDocument();
  });

  it("avisa quando a fila passa de 90% do limite", async () => {
    act(() =>
      useSettingsStore.setState({
        settings: { ...useSettingsStore.getState().settings!, queueLimit: 10 },
      }),
    );
    for (let i = 0; i < 9; i += 1) {
      mockEnqueue({
        url: `https://www.youtube.com/watch?v=other${i}`,
        sourceId: `other${i}`,
        metadataOverride: null,
        priority: false,
        allowDuplicate: false,
      });
    }
    renderApp("/collection");
    expect(await screen.findByTestId("queue-limit-warning")).toHaveTextContent(
      "A fila está quase cheia (9/10).",
    );
  });

  it("sem coleção aberta mostra o estado vazio", async () => {
    useFlowStore.getState().clearCollection();
    renderApp("/collection");
    expect(await screen.findByText("Nenhuma coleção aberta")).toBeInTheDocument();
  });
});
