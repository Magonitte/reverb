import { act, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router";
import { beforeEach, describe, expect, it } from "vitest";
import { AppRoutes } from "@/routes";
import { mockBus, resetMockSettings, resetMockTools, resetMockUpdater } from "@/lib/ipc/mock";
import { mockToolsCheckForce, seedMockTool, seedMockToolVersions } from "@/lib/ipc/mock/tools";
import { MOCK_UPDATE, mockUpdaterCheckCount, setMockUpdaterMode } from "@/lib/ipc/mock/updater";
import { initSettingsStore, useSettingsStore } from "@/stores/settings";
import { initToolsStore, useToolsStore } from "@/stores/tools";
import { useUiStore } from "@/stores/ui";
import { initialUpdaterState, initUpdaterStore, useUpdaterStore } from "@/stores/updater";

function renderUpdates() {
  return render(
    <MemoryRouter initialEntries={["/settings/updates"]}>
      <AppRoutes />
    </MemoryRouter>,
  );
}

beforeEach(async () => {
  resetMockSettings();
  resetMockTools();
  resetMockUpdater();
  mockBus.clear();
  useSettingsStore.setState({ settings: null });
  useToolsStore.setState({ statuses: [], progress: {} });
  useUiStore.setState({ toasts: [] });
  useUpdaterStore.setState({ ...initialUpdaterState, check: useUpdaterStore.getState().check, install: useUpdaterStore.getState().install });
  await initSettingsStore();
  await initToolsStore();
  await initUpdaterStore();
});

describe("Atualizações (T2)", () => {
  it("mostra cada estado do bloco do app", async () => {
    renderUpdates();
    expect(await screen.findByTestId("updater-phase")).toHaveTextContent("Ocioso");

    act(() => useUpdaterStore.setState({ phase: "checking" }));
    expect(screen.getByTestId("updater-phase")).toHaveTextContent("Verificando…");

    act(() => useUpdaterStore.setState({ phase: "uptodate" }));
    expect(screen.getByTestId("updater-phase")).toHaveTextContent("Atualizado");

    act(() =>
      useUpdaterStore.setState({
        phase: "available",
        version: "0.1.1",
        currentVersion: "0.1.0",
        notes: "Notas da versão",
      }),
    );
    expect(screen.getByTestId("updater-phase")).toHaveTextContent("Atualização disponível");
    expect(screen.getByText("0.1.0 → 0.1.1 disponível")).toBeInTheDocument();
    expect(screen.getByText("Notas da versão")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Atualizar agora" })).toBeInTheDocument();
    expect(screen.getByText("O Reverb será fechado para instalar a atualização.")).toBeInTheDocument();

    act(() => useUpdaterStore.setState({ phase: "downloading", downloaded: 40, total: 100 }));
    expect(screen.getByRole("progressbar", { name: "Baixando…" })).toHaveAttribute("aria-valuenow", "40");
    expect(screen.getByText("40%")).toBeInTheDocument();

    act(() => useUpdaterStore.setState({ phase: "ready" }));
    expect(screen.getByTestId("updater-phase")).toHaveTextContent("Pronto para reiniciar");

    act(() => useUpdaterStore.setState({ phase: "error", error: "Sem conexão com o GitHub" }));
    expect(screen.getByTestId("updater-phase")).toHaveTextContent("Erro ao atualizar");
    expect(screen.getByText("Sem conexão com o GitHub")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Tentar de novo" })).toBeInTheDocument();
  });

  it("mostra o bloco de ferramentas, inclusive reverter", async () => {
    seedMockTool("deno", false);
    seedMockTool("ffmpeg", false);
    seedMockToolVersions("ytdlp", "2026.08.19", "2026.07.01");
    await useToolsStore.getState().load();
    renderUpdates();
    const list = await screen.findByRole("list");
    expect(within(list).getByText("yt-dlp")).toBeInTheDocument();
    expect(within(list).getByText("2026.08.19")).toBeInTheDocument();
    expect(within(list).getByText("Atualização disponível")).toBeInTheDocument();
    expect(within(list).getByRole("button", { name: "Reverter" })).toBeInTheDocument();
    expect(within(list).getAllByText("Atualizado").length).toBeGreaterThan(0);
  });

  it("verificar atualizações chama o app e as ferramentas com force", async () => {
    const user = userEvent.setup();
    seedMockTool("ytdlp", false);
    seedMockTool("deno", false);
    seedMockTool("ffmpeg", false);
    setMockUpdaterMode("none");
    renderUpdates();
    await user.click(await screen.findByRole("button", { name: "Verificar atualizações" }));
    await waitFor(() => expect(screen.getByTestId("updater-phase")).toHaveTextContent("Atualizado"));
    expect(mockUpdaterCheckCount()).toBe(1);
    expect(mockToolsCheckForce()).toBe(true);
  });

  it("updater://available acende o ponto da sidebar e o toast", async () => {
    renderUpdates();
    await screen.findByTestId("updater-phase");
    mockBus.emit("updater://available", MOCK_UPDATE);
    await waitFor(() => expect(screen.getByTestId("dot-settings")).toBeInTheDocument());
    expect(screen.getByRole("status")).toHaveTextContent("Reverb 0.1.1 está disponível.");
  });
});
