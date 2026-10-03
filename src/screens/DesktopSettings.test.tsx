import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, expect, it, vi } from "vitest";
import { renderApp, resetFlowTests } from "@/testing/flow";
import { useSettingsStore } from "@/stores/settings";
import { api } from "@/lib/ipc/api";
import { mockCalls } from "@/lib/ipc/mock";
beforeEach(async () => {
  resetFlowTests();
  await useSettingsStore.getState().load();
});

it("persists all desktop toggles", async () => {
  renderApp("/settings");
  const user = userEvent.setup();
  for (const [label, key] of [
    ["Iniciar com o sistema", "launchAtStartup"],
    ["Iniciar minimizado", "startMinimized"],
    ["Minimizar para a bandeja", "minimizeToTray"],
    ["Fechar para a bandeja", "closeToTray"],
    ["Notificar ao concluir downloads", "completionNotifications"],
  ] as const) {
    const before = (await api.settingsGet())[key];
    await user.click(await screen.findByRole("switch", { name: label }));
    expect((await api.settingsGet())[key]).toBe(!before);
  }
});

it("shows missing tools, reruns after installation, and retains the latest report", async () => {
  const view = renderApp("/settings/advanced");
  const user = userEvent.setup();
  await user.click(await screen.findByRole("button", { name: "Rodar agora" }));
  const results = await screen.findByRole("list", { name: "Resultados do diagnóstico" });
  expect(within(results).getAllByText("Erro").length).toBeGreaterThan(0);
  await api.toolsInstallMissing();
  await user.click(screen.getByRole("button", { name: "Rodar agora" }));
  expect(within(results).queryByText("Erro")).not.toBeInTheDocument();
  expect(within(results).getAllByText("OK", { selector: "span" })).toHaveLength(10);
  view.unmount();
  renderApp("/settings/advanced");
  const retained = await screen.findByRole("list", { name: "Resultados do diagnóstico" });
  await waitFor(() => expect(retained).toBeVisible());
});

it("updates runtime, channel and weekly diagnostics, and handles run failures", async () => {
  renderApp("/settings/advanced");
  const user = userEvent.setup();
  await user.selectOptions(await screen.findByLabelText("Runtime JavaScript"), "system-node");
  await user.selectOptions(screen.getByLabelText("Canal do yt-dlp"), "nightly");
  await user.click(screen.getByRole("switch", { name: "Rodar diagnóstico semanal" }));
  expect(await api.settingsGet()).toMatchObject({
    jsRuntime: "system-node",
    ytdlpChannel: "nightly",
    weeklySelfTest: false,
  });
  const spy = vi.spyOn(api, "diagnosticsRun").mockRejectedValueOnce(new Error("offline"));
  await user.click(screen.getByRole("button", { name: "Rodar agora" }));
  expect(await screen.findByText(/A operação falhou/)).toBeVisible();
  expect(screen.getByRole("button", { name: "Rodar agora" })).toBeEnabled();
  spy.mockRestore();
});

it("exports archives and opens the data directory; restore requires confirmation", async () => {
  renderApp("/settings/advanced");
  const user = userEvent.setup();
  await user.click(await screen.findByRole("button", { name: "Exportar logs" }));
  await user.click(screen.getByRole("button", { name: "Criar backup" }));
  await user.click(screen.getByRole("button", { name: "Abrir pasta de dados" }));
  expect(mockCalls.map((call) => call.cmd)).toEqual([
    "logs_export",
    "pick_backup_path",
    "data_export",
    "open_data_dir",
  ]);
  await user.click(screen.getByRole("button", { name: "Restaurar backup" }));
  expect(mockCalls.some((call) => call.cmd === "data_import")).toBe(false);
  await user.click(
    within(await screen.findByRole("dialog")).getByRole("button", { name: "Cancelar" }),
  );
  expect(mockCalls.some((call) => call.cmd === "data_import")).toBe(false);
  await user.click(screen.getByRole("button", { name: "Restaurar backup" }));
  await user.click(
    within(await screen.findByRole("dialog")).getByRole("button", { name: "Confirmar" }),
  );
  expect(mockCalls.find((call) => call.cmd === "data_import")?.args).toEqual({
    path: "C:/Reverb/backup.zip",
  });
});

it("confirms reset, preserves secret status and starts onboarding again", async () => {
  await useSettingsStore
    .getState()
    .update({ acoustidKey: "private-reset-key", clipboardWatch: true });
  renderApp("/settings/advanced");
  const user = userEvent.setup();
  await user.click(await screen.findByRole("button", { name: "Restaurar padrões" }));
  await user.click(
    within(await screen.findByRole("dialog")).getByRole("button", { name: "Confirmar" }),
  );
  expect(await api.settingsGet()).toMatchObject({
    clipboardWatch: false,
    onboardingCompleted: false,
    secretsStatus: { acoustid: true },
  });
});
