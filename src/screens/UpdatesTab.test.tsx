import { act, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { api } from "@/lib/ipc/api";
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
afterEach(() => vi.restoreAllMocks());

beforeEach(async () => {
  resetMockSettings();
  resetMockTools();
  resetMockUpdater();
  mockBus.clear();
  useSettingsStore.setState({ settings: null });
  useToolsStore.setState({ statuses: [], progress: {}, updating: {}, errors: {} });
  useUiStore.setState({ toasts: [] });
  useUpdaterStore.setState({
    ...initialUpdaterState,
    check: useUpdaterStore.getState().check,
    install: useUpdaterStore.getState().install,
  });
  await initSettingsStore();
  await initToolsStore();
  await initUpdaterStore();
});

describe("Atualizações (T2)", () => {
  it("checks and installs Reverb from its own card with automatic checking disabled", async () => {
    await useSettingsStore.getState().update({ autoCheckAppUpdates: false });
    setMockUpdaterMode("available");
    const toolCheck = vi.spyOn(api, "toolsCheckUpdates");
    const install = vi.spyOn(api, "updaterInstall");
    renderUpdates();
    const card = await screen.findByRole("region", { name: "Reverb" });
    await userEvent.click(
      within(card).getByRole("button", { name: "Verificar atualização do Reverb" }),
    );
    await waitFor(() => expect(within(card).getByText("0.1.0 → 0.1.1 disponível")).toBeVisible());
    expect(within(card).getByText("Versão instalada: 0.1.0")).toBeVisible();
    expect(toolCheck).not.toHaveBeenCalled();
    await userEvent.click(within(card).getByRole("button", { name: "Atualizar agora" }));
    await waitFor(() =>
      expect(screen.getByTestId("updater-phase")).toHaveTextContent("Reiniciando…"),
    );
    expect(install).toHaveBeenCalledTimes(1);
    expect(
      within(card).getByRole("button", { name: "Verificar atualização do Reverb" }),
    ).toBeDisabled();
  });
  it("updates FFmpeg manually with automatic updates disabled and clears the offer", async () => {
    await useSettingsStore.getState().update({ autoUpdateTools: false });
    seedMockToolVersions("ffmpeg", "20260920T190056Z", null);
    await useToolsStore.getState().load();
    const update = vi.spyOn(api, "toolsUpdate");
    renderUpdates();
    const row = await screen.findByRole("listitem", { name: "FFmpeg" });
    await userEvent.click(within(row).getByRole("button", { name: "Atualizar FFmpeg" }));
    await waitFor(() => expect(within(row).getByText("Atualizado")).toBeInTheDocument());
    expect(update).toHaveBeenCalledExactlyOnceWith("ffmpeg");
    expect(within(row).getByText("20260930T190056Z")).toBeInTheDocument();
    expect(within(row).getByRole("button", { name: "Reverter" })).toBeInTheDocument();
  });
  it("keeps failed updates retryable and reports background failures", async () => {
    seedMockToolVersions("ffmpeg", "old", null);
    await useToolsStore.getState().load();
    const update = vi
      .spyOn(api, "toolsUpdate")
      .mockRejectedValueOnce({ kind: "network", message: "Falha de conexão" });
    renderUpdates();
    const row = await screen.findByRole("listitem", { name: "FFmpeg" });
    await userEvent.click(within(row).getByRole("button", { name: "Atualizar FFmpeg" }));
    await waitFor(() => expect(within(row).getByRole("alert")).toBeVisible());
    expect(within(row).getByRole("button", { name: "Atualizar FFmpeg" })).toBeEnabled();
    await userEvent.click(within(row).getByRole("button", { name: "Atualizar FFmpeg" }));
    await waitFor(() => expect(within(row).queryByRole("alert")).not.toBeInTheDocument());
    expect(update).toHaveBeenCalledTimes(2);
    act(() => {
      mockBus.emit("tools://progress", { tool: "ffmpeg", phase: "waiting_jobs", percent: 0 });
    });
    expect(
      within(row).getByRole("progressbar", { name: "Aguardando downloads terminarem…" }),
    ).toBeInTheDocument();
    act(() =>
      mockBus.emit("tools://failed", { tool: "ffmpeg", error: { message: "Falha automática" } }),
    );
    expect(within(row).getByRole("alert")).toHaveTextContent("Falha automática");
    expect(within(row).queryByRole("progressbar")).not.toBeInTheDocument();
  });
  it("installs known tool updates when checking with automatic updates enabled", async () => {
    seedMockTool("ffmpeg", true);
    await useToolsStore.getState().load();
    const update = vi.spyOn(api, "toolsUpdate");
    setMockUpdaterMode("none");
    renderUpdates();
    await userEvent.click(await screen.findByRole("button", { name: "Verificar atualizações" }));
    await waitFor(() =>
      expect(screen.getByTestId("updater-phase")).toHaveTextContent("Atualizado"),
    );
    expect(update).toHaveBeenCalledWith("ffmpeg");
    expect(
      within(screen.getByRole("listitem", { name: "FFmpeg" })).getByText("Atualizado"),
    ).toBeInTheDocument();
  });
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
    expect(
      screen.getByText("O Reverb será fechado para instalar a atualização."),
    ).toBeInTheDocument();

    act(() => useUpdaterStore.setState({ phase: "downloading", downloaded: 40, total: 100 }));
    expect(screen.getByRole("progressbar", { name: "Baixando…" })).toHaveAttribute(
      "aria-valuenow",
      "40",
    );
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
    await waitFor(() =>
      expect(screen.getByTestId("updater-phase")).toHaveTextContent("Atualizado"),
    );
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
