import { beforeEach, afterEach, it, expect, vi } from "vitest";
import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { renderApp, resetFlowTests, stubLayout } from "@/testing/flow";
import { api } from "@/lib/ipc/api";
import { useFlowStore } from "@/stores/flow";
import { useSettingsStore } from "@/stores/settings";
import { resetMockCollection } from "@/lib/ipc/mock/collection";
let restore: () => void;
beforeEach(async () => {
  resetFlowTests();
  resetMockCollection();
  restore = stubLayout();
  await useSettingsStore.getState().load();
});
afterEach(() => {
  restore();
  vi.restoreAllMocks();
});
it("preselects confident matches, requires review selection and forbids unmatched", async () => {
  const a = await api.importAnalyze("https://deezer.com/playlist/5207214368");
  useFlowStore.getState().openImport(a);
  renderApp("/collection");
  expect(
    await screen.findByRole("checkbox", { name: "Selecionar Never Gonna Give You Up" }),
  ).toBeChecked();
  expect(screen.getByRole("checkbox", { name: "Selecionar Track 2" })).not.toBeChecked();
  expect(screen.getByRole("checkbox", { name: "Selecionar Track 3" })).toBeDisabled();
  const spy = vi.spyOn(api, "importEnqueue");
  await userEvent.click(screen.getByRole("checkbox", { name: "Selecionar Track 2" }));
  await userEvent.click(screen.getByRole("button", { name: /Baixar selecionadas/ }));
  await waitFor(() =>
    expect(spy).toHaveBeenCalledWith(
      expect.objectContaining({ trackIds: ["1", "2"], mode: "once" }),
    ),
  );
});
it("Spotify without local credentials returns a stable translated error", async () => {
  await expect(api.importAnalyze("https://open.spotify.com/playlist/ABC")).rejects.toMatchObject({
    kind: "spotify_credentials_missing",
  });
});
