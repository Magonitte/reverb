import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, afterEach, describe, expect, it, vi } from "vitest";
import { api } from "@/lib/ipc/api";
import { FX4_ALBUM } from "@/lib/ipc/mock/fixtures";
import { seedMockSyncs } from "@/lib/ipc/mock/syncs";
import { useFlowStore } from "@/stores/flow";
import { initSyncsStore } from "@/stores/syncs";
import { renderApp, resetFlowTests, stubLayout } from "@/testing/flow";

let off: () => void;
let restore: () => void;
beforeEach(async () => {
  resetFlowTests();
  vi.restoreAllMocks();
  restore = stubLayout();
  off = await initSyncsStore();
});
afterEach(() => {
  off();
  restore();
});
describe("F11 playlists", () => {
  it("validates the limit and creates synchronization from the collection with chosen options", async () => {
    const create = vi.spyOn(api, "syncCreate");
    useFlowStore
      .getState()
      .openCollection(FX4_ALBUM, "https://www.youtube.com/playlist?list=album");
    renderApp("/collection");
    await userEvent.click(await screen.findByRole("button", { name: "Sincronizar esta playlist" }));
    const dialog = screen.getByRole("dialog");
    const form = within(dialog);
    await userEvent.type(
      form.getByRole("spinbutton", { name: "Limitar às primeiras faixas" }),
      "0",
    );
    await userEvent.click(form.getByRole("button", { name: "Criar sincronização" }));
    expect(create).not.toHaveBeenCalled();
    await userEvent.clear(form.getByRole("spinbutton"));
    await userEvent.type(form.getByRole("spinbutton"), "2");
    await userEvent.selectOptions(
      form.getByRole("combobox", { name: "Intervalo de sincronização" }),
      "6",
    );
    await userEvent.selectOptions(
      form.getByRole("combobox", { name: "Perfil de áudio" }),
      "mp3_v0",
    );
    await userEvent.type(form.getByRole("textbox", { name: "Pasta de destino" }), "C:/Playlist");
    await userEvent.click(
      form.getByRole("checkbox", { name: "Remover arquivos que saíram da playlist" }),
    );
    await userEvent.click(form.getByRole("button", { name: "Criar sincronização" }));
    await waitFor(() =>
      expect(create).toHaveBeenCalledWith({
        url: "https://www.youtube.com/playlist?list=album",
        title: null,
        profileId: "mp3_v0",
        outputDir: "C:/Playlist",
        intervalHours: 6,
        maxItems: 2,
        removeDeleted: true,
        writeM3u: true,
      }),
    );
    await waitFor(() => expect(screen.getByTestId("where")).toHaveTextContent("/playlists/sync-1"));
  });
  it("shows last/next, toggles enabled and asks about deleting files", async () => {
    await seedMockSyncs();
    renderApp("/playlists");
    const card = await screen.findByTestId("sync-card");
    expect(within(card).getByText("Última sincronização")).toBeInTheDocument();
    expect(within(card).getByText("Próxima sincronização")).toBeInTheDocument();
    const update = vi.spyOn(api, "syncUpdate");
    await userEvent.click(within(card).getByRole("switch"));
    await waitFor(() =>
      expect(update).toHaveBeenCalledWith("sync-1", expect.objectContaining({ enabled: false })),
    );
    expect(await screen.findByText("Desativada")).toBeInTheDocument();
    const remove = vi.spyOn(api, "syncDelete");
    await userEvent.click(within(card).getByRole("button", { name: "Excluir" }));
    expect(remove).not.toHaveBeenCalled();
    const dialog = within(screen.getByRole("dialog"));
    await userEvent.click(dialog.getByRole("checkbox"));
    await userEvent.click(dialog.getByRole("button", { name: "Excluir" }));
    await waitFor(() => expect(remove).toHaveBeenCalledWith("sync-1", true));
    await waitFor(() => expect(screen.queryByTestId("sync-card")).not.toBeInTheDocument());
  });
  it("lists varied item states and queues a synchronization manually", async () => {
    await seedMockSyncs();
    renderApp("/playlists/sync-1");
    const table = await screen.findByRole("table", { name: "Faixas sincronizadas" });
    await waitFor(() => expect(within(table).getAllByText("Falhou").length).toBeGreaterThan(0));
    expect(within(table).getByText("Removida da playlist")).toBeInTheDocument();
    expect(within(table).getAllByText("Baixada").length).toBeGreaterThan(0);
    const run = vi.spyOn(api, "syncRun");
    await userEvent.click(screen.getByRole("button", { name: "Sincronizar agora" }));
    await waitFor(() => expect(run).toHaveBeenCalledWith("sync-1"));
  });
});
