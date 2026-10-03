import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { act, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { renderApp, resetFlowTests } from "@/testing/flow";
import { api } from "@/lib/ipc/api";
import { useSettingsStore } from "@/stores/settings";

beforeEach(async () => {
  resetFlowTests();
  await useSettingsStore.getState().load();
});
afterEach(() => vi.restoreAllMocks());
async function dialog() {
  renderApp("/library?follow=6160");
  const d = await screen.findByRole("dialog", { name: "Seguir artista" });
  return { d, user: userEvent.setup() };
}
it("renders every option visibly and saves all edited preferences", async () => {
  const follow = vi.spyOn(api, "artistFollow");
  const { d, user } = await dialog();
  expect(within(d).getByText("Artista selecionado: Deezer #6160")).toBeVisible();
  for (const name of ["Álbuns", "EPs", "Singles", "Excluir ao vivo, remix e edições duplicadas"]) {
    expect(within(d).getByText(name)).toBeVisible();
    await user.click(within(d).getByRole("checkbox", { name }));
  }
  expect(within(d).getByText("Selecione pelo menos um tipo de lançamento.")).toBeVisible();
  expect(within(d).getByRole("button", { name: "Salvar" })).toBeDisabled();
  await user.click(within(d).getByRole("checkbox", { name: "Álbuns" }));
  for (const value of ["all", "latest", "none"]) {
    await user.selectOptions(within(d).getByLabelText("O que baixar agora"), value);
    expect(within(d).getByLabelText("O que baixar agora")).toHaveValue(value);
  }
  for (const value of ["all", "notify", "none"]) {
    await user.selectOptions(within(d).getByLabelText("Lançamentos novos"), value);
    expect(within(d).getByLabelText("Lançamentos novos")).toHaveValue(value);
  }
  for (const value of ["original", "mp3_v0", "mp3_320", "aac_256", "opus_96", "flac"]) {
    await user.selectOptions(within(d).getByLabelText("Formato de saída"), value);
    expect(within(d).getByLabelText("Formato de saída")).toHaveValue(value);
  }
  const pick = vi
    .spyOn(api, "pickFolder")
    .mockResolvedValueOnce(null)
    .mockResolvedValueOnce("D:\\Colecao");
  await user.type(within(d).getByLabelText("Pasta de destino"), "D:\\Music");
  await user.click(within(d).getByRole("button", { name: "Escolher pasta" }));
  expect(within(d).getByLabelText("Pasta de destino")).toHaveValue("D:\\Music");
  await user.click(within(d).getByRole("button", { name: "Escolher pasta" }));
  await waitFor(() =>
    expect(within(d).getByLabelText("Pasta de destino")).toHaveValue("D:\\Colecao"),
  );
  expect(pick).toHaveBeenCalledTimes(2);
  await user.click(within(d).getByRole("button", { name: "Salvar" }));
  await waitFor(() => expect(d).not.toBeInTheDocument());
  expect(follow).toHaveBeenCalledExactlyOnceWith("6160", {
    monitorExisting: "none",
    monitorNew: "none",
    types: ["album"],
    excludeVariants: false,
    profileId: "flac",
    outputDir: "D:\\Colecao",
  });
  await user.click(await screen.findByRole("button", { name: "Editar acompanhamento" }));
  const edit = screen.getByRole("dialog", { name: "Editar acompanhamento" });
  expect(within(edit).getByLabelText("Formato de saída")).toHaveValue("flac");
  expect(within(edit).getByLabelText("Pasta de destino")).toHaveValue("D:\\Colecao");
});
it("closes after persistence even if catalog verification fails, without saving twice", async () => {
  const follow = vi.spyOn(api, "artistFollow");
  let reject!: (e: unknown) => void;
  vi.spyOn(api, "artistsCheckNow").mockImplementation(
    () =>
      new Promise((_, fail) => {
        reject = fail;
      }),
  );
  const { d, user } = await dialog();
  await user.click(within(d).getByRole("button", { name: "Salvar" }));
  await waitFor(() => expect(d).not.toBeInTheDocument());
  expect(await screen.findByText(/Consultando discografias/)).toBeVisible();
  await act(async () => reject({ message: "Catálogo temporariamente indisponível" }));
  expect(await screen.findByText("Catálogo temporariamente indisponível")).toBeVisible();
  expect(follow).toHaveBeenCalledTimes(1);
  expect(screen.getByRole("button", { name: "Verificar agora" })).toBeEnabled();
});
it("supports search with Enter, selection, empty results and a fresh dialog after cancel", async () => {
  renderApp("/library");
  const user = userEvent.setup();
  await user.click(screen.getByRole("button", { name: "Artistas seguidos" }));
  await user.click(screen.getByRole("button", { name: "Seguir artista" }));
  let d = screen.getByRole("dialog");
  expect(within(d).getByRole("button", { name: "Salvar" })).toBeDisabled();
  await user.type(within(d).getByLabelText("Buscar artista"), "Rick Astley{Enter}");
  await user.click(await within(d).findByRole("button", { name: /Rick Astley.*Deezer/ }));
  expect(within(d).getByText("Artista selecionado: Rick Astley")).toBeVisible();
  await user.click(within(d).getByRole("button", { name: "Cancelar" }));
  await user.click(screen.getByRole("button", { name: "Seguir artista" }));
  d = screen.getByRole("dialog");
  expect(within(d).getByLabelText("Buscar artista")).toHaveValue("");
  expect(within(d).getByRole("button", { name: "Salvar" })).toBeDisabled();
  vi.spyOn(api, "artistsSearch").mockResolvedValueOnce([]);
  await user.type(within(d).getByLabelText("Buscar artista"), "No such artist{Enter}");
  expect(await within(d).findByText("Nenhum artista encontrado. Tente outro nome.")).toBeVisible();
});
it("does not show stale results or select a previous query's artist", async () => {
  let resolve!: (hits: Awaited<ReturnType<typeof api.artistsSearch>>) => void;
  vi.spyOn(api, "artistsSearch").mockImplementationOnce(
    () =>
      new Promise((done) => {
        resolve = done;
      }),
  );
  const { d, user } = await dialog();
  await user.type(within(d).getByLabelText("Buscar artista"), "Old{Enter}");
  await user.clear(within(d).getByLabelText("Buscar artista"));
  await user.type(within(d).getByLabelText("Buscar artista"), "New");
  await act(async () => resolve([{ id: "1", name: "Old result", picture: null, fans: 0 }]));
  expect(within(d).queryByText("Old result")).not.toBeInTheDocument();
  expect(within(d).getByRole("button", { name: "Salvar" })).toBeDisabled();
});
