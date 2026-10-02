import { act, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { api } from "@/lib/ipc/api";
import { mockCalls } from "@/lib/ipc/mock/media";
import { seedMockLibrary } from "@/lib/ipc/mock/library";
import { applyScenario } from "@/lib/ipc/mock/scenarios";
import { initLibraryStore, useLibraryStore } from "@/stores/library";
import { renderApp, resetFlowTests, stubLayout } from "@/testing/flow";

let restoreLayout: () => void;
let off: () => void;
beforeEach(async () => {
  resetFlowTests();
  vi.restoreAllMocks();
  restoreLayout = stubLayout();
  off = await initLibraryStore();
});
afterEach(() => {
  off();
  restoreLayout();
});

describe("Biblioteca F10", () => {
  it("paginates 5000 records, virtualizes the DOM, searches accentless prefixes", async () => {
    applyScenario("big");
    renderApp("/library");
    await screen.findByText("1–100 de 5000 faixas");
    const table = await screen.findByRole("table", { name: "Biblioteca" });
    expect(within(table).getAllByRole("row").length).toBeLessThan(200);
    expect(within(table).getAllByRole("row").length).toBeGreaterThan(1);
    await userEvent.click(screen.getByRole("button", { name: "Próxima" }));
    await screen.findByText("101–200 de 5000 faixas");
    await userEvent.type(
      screen.getByRole("textbox", { name: "Buscar na biblioteca" }),
      "musica nev",
    );
    await screen.findByText("1–1 de 1 faixas");
    expect(await screen.findByText("Música Never Gonna Give You Up")).toBeInTheDocument();
  });

  it("combines format, date, artist, album, missing and review filters", async () => {
    const now = Math.floor(Date.now() / 1000);
    seedMockLibrary([
      { title: "Target", artist: "Rick", album: "Album", needsReview: true, missing: true },
      { title: "Wrong format", artist: "Rick", album: "Album", filePath: "other.mp3" },
      { title: "Old", artist: "Jane", album: "Other", addedAt: now - 60 * 86400 },
    ]);
    renderApp("/library");
    await screen.findByText("1–3 de 3 faixas");
    await userEvent.selectOptions(screen.getByLabelText("Formato"), "opus");
    await userEvent.selectOptions(screen.getByLabelText("Adicionado"), "today");
    await userEvent.selectOptions(screen.getByLabelText("Artista"), "Rick");
    await userEvent.selectOptions(screen.getByLabelText("Álbum"), "Album");
    await userEvent.click(screen.getByRole("checkbox", { name: "Precisa revisão" }));
    await userEvent.click(screen.getByRole("checkbox", { name: "Arquivo ausente" }));
    await screen.findByText("1–1 de 1 faixas");
    expect(useLibraryStore.getState().items[0]?.title).toBe("Target");
  });

  it("requires confirmation and passes selected IDs and trash intent", async () => {
    seedMockLibrary([{ title: "A" }, { title: "B" }]);
    renderApp("/library");
    await screen.findByText("1–2 de 2 faixas");
    await userEvent.click(screen.getByRole("checkbox", { name: "Selecionar esta página" }));
    await userEvent.click(screen.getByRole("button", { name: "Mover arquivos para a lixeira" }));
    const dialog = await screen.findByRole("dialog");
    expect(mockCalls.some((c) => c.cmd === "library_delete")).toBe(false);
    await userEvent.click(within(dialog).getByRole("button", { name: "Confirmar" }));
    await screen.findByText("Sua biblioteca está vazia");
    expect(mockCalls).toContainEqual({
      cmd: "library_delete",
      args: { ids: [2, 1], deleteFiles: true },
    });
  });

  it("clears records only after confirmation; cancel changes nothing", async () => {
    seedMockLibrary([{ title: "A" }]);
    renderApp("/library");
    await screen.findByText("1–1 de 1 faixas");
    await userEvent.click(screen.getByRole("button", { name: "Limpar biblioteca" }));
    await userEvent.click(
      within(screen.getByRole("dialog")).getByRole("button", { name: "Cancelar" }),
    );
    expect(mockCalls.some((c) => c.cmd === "library_clear")).toBe(false);
    await userEvent.click(screen.getByRole("button", { name: "Limpar biblioteca" }));
    await userEvent.click(
      within(screen.getByRole("dialog")).getByRole("button", { name: "Confirmar" }),
    );
    await screen.findByText("Sua biblioteca está vazia");
    expect(mockCalls).toContainEqual({ cmd: "library_clear" });
  });

  it("redownloads with the same profile and allowDuplicate", async () => {
    const enqueue = vi.spyOn(api, "enqueue");
    seedMockLibrary([
      { title: "Song", profileId: "mp3_v0", sourceUrl: "https://www.youtube.com/watch?v=abc" },
    ]);
    renderApp("/library");
    await screen.findByText("1–1 de 1 faixas");
    await userEvent.click(screen.getByRole("checkbox", { name: "Selecionar esta página" }));
    await userEvent.click(screen.getByRole("button", { name: "Baixar novamente" }));
    expect(enqueue).toHaveBeenCalledWith({
      url: "https://www.youtube.com/watch?v=abc",
      profileId: "mp3_v0",
      allowDuplicate: true,
    });
  });

  it("dismiss refreshes review count in the sidebar", async () => {
    seedMockLibrary([{ title: "Review", needsReview: true }]);
    renderApp("/library");
    expect(await screen.findByTestId("badge-review")).toHaveTextContent("1");
    await act(async () => {
      await api.reviewDismiss(1);
    });
    await waitFor(() => expect(screen.queryByTestId("badge-review")).not.toBeInTheDocument());
  });

  it("shows query errors and supports retry", async () => {
    vi.spyOn(api, "libraryList").mockRejectedValueOnce({ kind: "disk", message: "failure" });
    renderApp("/library");
    expect(await screen.findByRole("alert")).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "Tentar de novo" }));
    await waitFor(() => expect(screen.queryByRole("alert")).not.toBeInTheDocument());
  });
});
