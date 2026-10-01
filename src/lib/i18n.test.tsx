import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { useAppearance } from "@/hooks/useAppearance";
import i18n from "@/lib/i18n";
import { resetMockSettings } from "@/lib/ipc/mock";
import { AppRoutes } from "@/routes";
import { initSettingsStore, useSettingsStore } from "@/stores/settings";

function Root() {
  useAppearance();
  return (
    <MemoryRouter initialEntries={["/settings"]}>
      <AppRoutes />
    </MemoryRouter>
  );
}

describe("i18n: troca de idioma pela configuração (T5)", () => {
  beforeEach(() => {
    resetMockSettings();
    useSettingsStore.setState({ settings: null });
  });
  afterEach(async () => {
    await i18n.changeLanguage("pt-BR");
  });

  it("trocar o idioma na tela de configurações muda a sidebar sem recarregar", async () => {
    const off = await initSettingsStore();
    render(<Root />);
    const nav = () => screen.getByTestId("sidebar");
    expect(await screen.findByRole("heading", { level: 1, name: "Configurações" })).toBeVisible();
    expect(within(nav()).getByRole("link", { name: "Início" })).toBeInTheDocument();

    await userEvent.selectOptions(await screen.findByLabelText("Idioma"), "en");

    await waitFor(() =>
      expect(within(nav()).getByRole("link", { name: "Home" })).toBeInTheDocument(),
    );
    expect(within(nav()).getByRole("link", { name: "Library" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { level: 1, name: "Settings" })).toBeInTheDocument();
    expect(document.documentElement.lang).toBe("en");

    await userEvent.selectOptions(screen.getByLabelText("Language"), "pt-BR");
    await waitFor(() =>
      expect(within(nav()).getByRole("link", { name: "Início" })).toBeInTheDocument(),
    );
    off();
  });

  it("o tema escolhido nas configurações vai para data-theme", async () => {
    const off = await initSettingsStore();
    render(<Root />);
    await userEvent.selectOptions(await screen.findByLabelText("Tema"), "light");
    await waitFor(() => expect(document.documentElement.dataset.theme).toBe("light"));
    await userEvent.selectOptions(screen.getByLabelText("Tema"), "dark");
    await waitFor(() => expect(document.documentElement.dataset.theme).toBe("dark"));
    off();
  });
});
