import { render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it } from "vitest";
import { App } from "@/App";
import { resetFlowTests } from "@/testing/flow";
import { useSettingsStore } from "@/stores/settings";

beforeEach(async () => {
  resetFlowTests();
  await useSettingsStore.getState().load();
});

describe("App", () => {
  it("monta a casca com HashRouter e mostra o Início", async () => {
    window.location.hash = "#/";
    render(<App />);
    expect(await screen.findByRole("heading", { level: 1, name: "Início" })).toBeInTheDocument();
    expect(screen.getByRole("textbox", { name: "Barra de comando" })).toBeInTheDocument();
    expect(screen.getByTestId("sidebar")).toBeInTheDocument();
  });
  it("redirects first run to onboarding even when another route was requested", async () => {
    await useSettingsStore.getState().update({ onboardingCompleted: false });
    window.location.hash = "#/settings/advanced";
    render(<App />);
    expect(
      await screen.findByRole("heading", { level: 1, name: "Bem-vindo ao Reverb" }),
    ).toBeVisible();
    expect(window.location.hash).toBe("#/onboarding");
    expect(screen.queryByRole("textbox", { name: "Barra de comando" })).not.toBeInTheDocument();
  });
});
