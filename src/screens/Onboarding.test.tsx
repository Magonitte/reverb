import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it } from "vitest";
import { renderApp, resetFlowTests } from "@/testing/flow";
import { api } from "@/lib/ipc/api";
import { failMockInstall, resetMockTools } from "@/lib/ipc/mock/tools";
import { useSettingsStore } from "@/stores/settings";

beforeEach(async () => {
  resetFlowTests();
  await useSettingsStore.getState().load();
  await useSettingsStore.getState().update({ onboardingCompleted: false });
});
async function reachTools() {
  renderApp("/onboarding");
  await userEvent.click(await screen.findByRole("button", { name: "Continuar" }));
  expect(screen.getByRole("button", { name: "Continuar" })).toBeDisabled();
  await userEvent.click(screen.getByRole("button", { name: "Escolher pasta" }));
  await userEvent.click(screen.getByRole("button", { name: "Continuar" }));
  expect(await screen.findByRole("button", { name: "Instalar ferramentas" })).toBeVisible();
}
describe("F12 onboarding", () => {
  it("validates folder/tools, retries installation and completes all six steps", async () => {
    failMockInstall();
    await reachTools();
    expect(screen.getByRole("button", { name: "Continuar" })).toBeDisabled();
    await userEvent.selectOptions(
      screen.getByRole("combobox", { name: "Runtime JavaScript" }),
      "system-node",
    );
    await userEvent.click(screen.getByRole("button", { name: "Instalar ferramentas" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("A instalação falhou");
    expect((await api.settingsGet()).onboardingCompleted).toBe(false);
    await userEvent.click(screen.getByRole("button", { name: "Tentar de novo" }));
    await screen.findByText("Ferramentas prontas");
    await userEvent.click(screen.getByRole("button", { name: "Continuar" }));
    expect(screen.getByText(/funcionam sem chaves de API/)).toBeVisible();
    await userEvent.click(screen.getByRole("button", { name: "Continuar" }));
    expect(screen.getByRole("switch", { name: "Iniciar com o sistema" })).toHaveAttribute(
      "aria-checked",
      "false",
    );
    await userEvent.click(screen.getByRole("button", { name: "Continuar" }));
    await userEvent.click(screen.getByRole("button", { name: "Começar a usar" }));
    await waitFor(() =>
      expect(useSettingsStore.getState().settings?.onboardingCompleted).toBe(true),
    );
    expect((await api.settingsGet()).jsRuntime).toBe("system-node");
    expect(screen.getByTestId("where")).toHaveTextContent("/");
  });
  it("rechecks required tools before completion", async () => {
    await reachTools();
    await userEvent.click(screen.getByRole("button", { name: "Instalar ferramentas" }));
    await screen.findByText("Ferramentas prontas");
    for (let index = 0; index < 3; index++)
      await userEvent.click(screen.getByRole("button", { name: "Continuar" }));
    resetMockTools();
    await userEvent.click(screen.getByRole("button", { name: "Começar a usar" }));
    expect(await screen.findByRole("alert")).toHaveTextContent(
      "Instale as ferramentas obrigatórias",
    );
    expect((await api.settingsGet()).onboardingCompleted).toBe(false);
  });
});
