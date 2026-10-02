import { act, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { renderApp, resetFlowTests } from "@/testing/flow";
import { useSettingsStore } from "@/stores/settings";
import { api } from "@/lib/ipc/api";

beforeEach(async () => {
  resetFlowTests();
  await useSettingsStore.getState().load();
});
it("persists the cookie browser and tests Premium through IPC", async () => {
  const user = userEvent.setup();
  renderApp("/settings/downloads");
  await user.click(await screen.findByRole("button", { name: "Testar cookies" }));
  await waitFor(() =>
    expect(screen.getByRole("status", { name: "Testar cookies" })).toHaveTextContent(
      "Qualidade padrão · Opus 129 kbps",
    ),
  );
  await user.selectOptions(screen.getByRole("combobox", { name: "Origem dos cookies" }), "firefox");
  await waitFor(async () => expect((await api.settingsGet()).cookiesSource).toBe("firefox"));
  await user.click(screen.getByRole("button", { name: "Testar cookies" }));
  await waitFor(() =>
    expect(screen.getByRole("status", { name: "Testar cookies" })).toHaveTextContent(
      "Premium disponível · AAC 256 kbps",
    ),
  );
});

import { TrimDialog } from "@/components/TrimDialog";
import { UpgradePanel } from "@/components/UpgradePanel";
import { ProviderSettings } from "@/screens/ProviderSettings";
import { seedMockLibrary } from "@/lib/ipc/mock/library";
afterEach(() => vi.restoreAllMocks());
it("shows an actionable cookie failure", async () => {
  vi.spyOn(api, "cookiesTest").mockResolvedValue({
    ok: false,
    premium: false,
    bestAudio: null,
    errorKind: "cookies_decrypt",
  });
  renderApp("/settings/downloads");
  await userEvent.click(await screen.findByRole("button", { name: "Testar cookies" }));
  expect(await screen.findByRole("status", { name: "Testar cookies" })).toHaveTextContent(
    /Firefox/,
  );
});
it("queues selected quality upgrades with the item's profile", async () => {
  await useSettingsStore.getState().update({ cookiesSource: "firefox" });
  seedMockLibrary([
    {
      id: 7,
      title: "Song",
      sourceId: "lYBUbBu4W08",
      sourceUrl: "https://youtu.be/lYBUbBu4W08",
      sourceAbrKbps: 129,
      profileId: "mp3-320",
    },
  ]);
  const enqueue = vi.spyOn(api, "upgradeEnqueue");
  render(<UpgradePanel ids={[7]} />);
  await userEvent.click(screen.getByRole("button", { name: "Melhorar qualidade" }));
  expect(await screen.findByRole("checkbox", { name: "Song: 129 → 256 kbps" })).toBeChecked();
  await userEvent.click(screen.getByRole("button", { name: "Adicionar melhorias à fila" }));
  await waitFor(() => expect(enqueue).toHaveBeenCalledWith([7]));
});
it("keeps keys masked and shows the provider test status", async () => {
  const user = userEvent.setup();
  render(<ProviderSettings />);
  const group = within(screen.getByRole("group", { name: "Discogs" }));
  const input = group.getByLabelText("Token Discogs");
  expect(input).toHaveAttribute("type", "password");
  await user.type(input, "test-discogs");
  await user.click(group.getByRole("button", { name: "Salvar" }));
  await waitFor(() => expect(group.getByText(/Configurado/)).toBeInTheDocument());
  expect(input).toHaveValue("");
  await user.click(group.getByRole("button", { name: "Testar conexão" }));
  expect(await group.findByRole("status")).toHaveTextContent("Conexão válida");
});
it("loads a waveform and rejects empty or reversed trim ranges", async () => {
  const user = userEvent.setup(),
    done = vi.fn(),
    close = vi.fn();
  const trim = vi.spyOn(api, "trimAudio");
  render(<TrimDialog path="C:/test/song.opus" duration={10} onClose={close} onDone={done} />);
  expect(await screen.findByRole("img", { name: "Forma de onda do áudio" })).toHaveAttribute(
    "src",
    expect.stringContaining("data:image/png"),
  );
  const save = screen.getByRole("button", { name: "Cortar áudio" });
  const start = screen.getByRole("textbox", { name: "Início (mm:ss.s)" }),
    end = screen.getByRole("textbox", { name: "Fim (mm:ss.s)" });
  await user.clear(start);
  expect(save).toBeDisabled();
  await user.type(start, "0:04.0");
  await user.clear(end);
  await user.type(end, "0:03.0");
  expect(save).toBeDisabled();
  await user.clear(start);
  await user.type(start, "0:01.0");
  expect(save).toBeEnabled();
  await user.click(save);
  await waitFor(() => expect(trim).toHaveBeenCalledWith("C:/test/song.opus", 1, 3));
  expect(done).toHaveBeenCalled();
  expect(close).toHaveBeenCalled();
});

import { useFlowStore } from "@/stores/flow";
import { FX3_CLIP } from "@/lib/ipc/mock/fixtures";
it("offers chapter splitting and carries the choice to the queue", async () => {
  const enqueue = vi.spyOn(api, "enqueue");
  const official = vi.spyOn(api, "findOfficialVersion");
  renderApp("/");
  await screen.findByRole("heading", { name: "Início" });
  act(() =>
    useFlowStore.getState().openPreview({
      ...FX3_CLIP,
      duration: 601,
      chapters: [
        { title: "One", startTime: 0, endTime: 300 },
        { title: "Two", startTime: 300, endTime: 601 },
      ],
    }),
  );
  const dialog = within(await screen.findByRole("dialog", { name: "Pré-visualização" }));
  const split = dialog.getByRole("switch", { name: "Dividir em faixas" });
  expect(split).not.toBeChecked();
  expect(official).not.toHaveBeenCalled();
  await userEvent.click(split);
  await userEvent.click(dialog.getByRole("button", { name: "Baixar agora" }));
  await waitFor(() =>
    expect(enqueue).toHaveBeenCalledWith(
      expect.objectContaining({ options: expect.objectContaining({ splitChapters: true }) }),
    ),
  );
});
