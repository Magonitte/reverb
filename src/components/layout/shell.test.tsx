import { act, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter, useLocation } from "react-router";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import i18n from "@/lib/i18n";
import { mockBus, resetMockQueue, resetMockSettings, resetMockTools, resetMockUpdater } from "@/lib/ipc/mock";
import { applyScenario } from "@/lib/ipc/mock/scenarios";
import { AppRoutes } from "@/routes";
import { useHealStore } from "@/stores/heal";
import { initHealStore } from "@/stores/heal";
import { initJobsStore, useJobsStore } from "@/stores/jobs";
import { useSettingsStore } from "@/stores/settings";
import { useToolsStore } from "@/stores/tools";
import { useUiStore } from "@/stores/ui";
import { initialUpdaterState, useUpdaterStore } from "@/stores/updater";

function Where() {
  const { pathname } = useLocation();
  return <output data-testid="where">{pathname}</output>;
}

function renderApp(path = "/") {
  return render(
    <MemoryRouter initialEntries={[path]}>
      <AppRoutes />
      <Where />
    </MemoryRouter>,
  );
}

const sidebar = () => screen.getByTestId("sidebar");

beforeEach(() => {
  resetMockQueue();
  resetMockSettings();
  resetMockTools();
  resetMockUpdater();
  mockBus.clear();
  useJobsStore.setState({
    jobs: {},
    queue: { paused: false, running: 0, queued: 0, healing: false },
  });
  useToolsStore.setState({ statuses: [], progress: {}, updating: {}, errors: {} });
  useHealStore.setState({ stage: null });
  useUpdaterStore.setState(initialUpdaterState);
  useUiStore.setState({
    commandBarOpen: false,
    commandBarText: "",
    commandBarFocusTick: 0,
    toasts: [],
  });
  useSettingsStore.setState({ settings: null });
});

afterEach(async () => {
  await i18n.changeLanguage("pt-BR");
});

describe("Shell: layout e navegação (T7)", () => {
  it("mostra Titlebar, Sidebar com os itens do design e a tela inicial", async () => {
    renderApp();
    expect(await screen.findByRole("heading", { level: 1, name: "Início" })).toBeInTheDocument();
    expect(screen.getByTestId("titlebar")).toBeInTheDocument();
    for (const name of [
      "Início",
      "Biblioteca",
      "Playlists",
      "Atividade",
      "Revisar",
      "Configurações",
    ]) {
      expect(within(sidebar()).getByRole("link", { name })).toBeInTheDocument();
    }
    expect(screen.getByRole("button", { name: "Minimizar" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Maximizar" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Fechar janela" })).toBeInTheDocument();
  });

  it("clicar em cada item leva à rota e mostra o título", async () => {
    renderApp();
    const cases: Array<[string, string, string]> = [
      ["Biblioteca", "/library", "Biblioteca"],
      ["Playlists", "/playlists", "Playlists"],
      ["Atividade", "/activity", "Atividade"],
      ["Revisar", "/review", "Revisar"],
      ["Configurações", "/settings", "Configurações"],
      ["Início", "/", "Início"],
    ];
    for (const [link, path, title] of cases) {
      await userEvent.click(within(sidebar()).getByRole("link", { name: link }));
      expect(screen.getByTestId("where")).toHaveTextContent(new RegExp(`^${path}$`));
      expect(await screen.findByRole("heading", { level: 1, name: title })).toBeInTheDocument();
    }
  });

  it("o item da rota atual fica marcado como página atual", async () => {
    renderApp("/library");
    await screen.findByRole("heading", { level: 1, name: "Biblioteca" });
    expect(within(sidebar()).getByRole("link", { name: "Biblioteca" })).toHaveAttribute(
      "aria-current",
      "page",
    );
    expect(within(sidebar()).getByRole("link", { name: "Início" })).not.toHaveAttribute(
      "aria-current",
    );
  });

  it("rotas desconhecidas voltam ao Início; detalhe de playlist e tag-editor existem", async () => {
    renderApp("/nao-existe");
    expect(await screen.findByRole("heading", { level: 1, name: "Início" })).toBeInTheDocument();
  });

  it.each([
    ["/playlists/42", "Playlist"],
    ["/tag-editor", "Editor de tags"],
    ["/settings/updates", "Configurações"],
  ])("a rota %s abre com título", async (path, title) => {
    renderApp(path);
    expect(await screen.findByRole("heading", { level: 1, name: title })).toBeInTheDocument();
  });

  it("onboarding abre sem sidebar", async () => {
    renderApp("/onboarding");
    expect(
      await screen.findByRole("heading", { level: 1, name: "Bem-vindo ao Reverb" }),
    ).toBeInTheDocument();
    expect(screen.queryByTestId("sidebar")).not.toBeInTheDocument();
  });

  it("cada tela sem dados mostra o estado vazio", async () => {
    for (const path of ["/library", "/playlists", "/review", "/tag-editor", "/activity", "/"]) {
      const { unmount } = renderApp(path);
      expect(await screen.findByTestId("empty-state")).toBeInTheDocument();
      unmount();
    }
  });
});

describe("Shell: atalhos (T7)", () => {
  it("Ctrl+D, Ctrl+Q, Ctrl+H e Ctrl+, navegam", async () => {
    renderApp("/review");
    await screen.findByRole("heading", { level: 1, name: "Revisar" });
    const cases: Array<[string, string]> = [
      ["{Control>}q{/Control}", "/activity"],
      ["{Control>}h{/Control}", "/library"],
      ["{Control>},{/Control}", "/settings"],
      ["{Control>}d{/Control}", "/"],
    ];
    for (const [keys, path] of cases) {
      await userEvent.keyboard(keys);
      await waitFor(() => expect(screen.getByTestId("where")).toHaveTextContent(`${path}`));
      expect(screen.getByTestId("where").textContent).toBe(path);
    }
  });

  it("Ctrl+K abre o overlay com a barra focada; Esc fecha", async () => {
    renderApp("/library");
    await screen.findByRole("heading", { level: 1, name: "Biblioteca" });
    await userEvent.keyboard("{Control>}k{/Control}");
    const overlay = await screen.findByTestId("command-bar-overlay");
    const input = within(overlay).getByRole("textbox", { name: "Barra de comando" });
    expect(input).toHaveFocus();
    await userEvent.keyboard("{Escape}");
    expect(screen.queryByTestId("command-bar-overlay")).not.toBeInTheDocument();
  });

  it("Esc limpa o texto da barra do Início", async () => {
    renderApp("/");
    const input = await screen.findByRole("textbox", { name: "Barra de comando" });
    await userEvent.type(input, "rick astley");
    expect(input).toHaveValue("rick astley");
    await userEvent.keyboard("{Escape}");
    expect(input).toHaveValue("");
  });

  it("Ctrl+V fora de campos leva ao Início e foca a barra", async () => {
    renderApp("/library");
    await screen.findByRole("heading", { level: 1, name: "Biblioteca" });
    await userEvent.keyboard("{Control>}v{/Control}");
    expect(await screen.findByRole("heading", { level: 1, name: "Início" })).toBeInTheDocument();
    await waitFor(() =>
      expect(screen.getByRole("textbox", { name: "Barra de comando" })).toHaveFocus(),
    );
  });

  it("Espaço pausa e retoma a fila (fora de controles)", async () => {
    const off = await initJobsStore();
    renderApp("/library");
    await screen.findByRole("heading", { level: 1, name: "Biblioteca" });
    await userEvent.keyboard(" ");
    await waitFor(() => expect(useJobsStore.getState().queue.paused).toBe(true));
    await userEvent.keyboard(" ");
    await waitFor(() => expect(useJobsStore.getState().queue.paused).toBe(false));
    off();
  });

  it("Espaço dentro do campo de busca não pausa a fila", async () => {
    const off = await initJobsStore();
    renderApp("/");
    const input = await screen.findByRole("textbox", { name: "Barra de comando" });
    await userEvent.type(input, "a b");
    expect(input).toHaveValue("a b");
    expect(useJobsStore.getState().queue.paused).toBe(false);
    off();
  });
});

describe("Shell: indicadores vindos das stores (T11)", () => {
  it("o badge de Atividade conta ativos e pendentes (cenário busy)", async () => {
    const off = await initJobsStore();
    applyScenario("busy");
    renderApp("/");
    await waitFor(() => expect(screen.getByTestId("badge-activity")).toHaveTextContent("5"));
    off();
  });

  it("sem jobs não há badge", async () => {
    renderApp("/");
    await screen.findByRole("heading", { level: 1, name: "Início" });
    expect(screen.queryByTestId("badge-activity")).not.toBeInTheDocument();
  });

  it("ponto de atualização em Configurações quando uma ferramenta tem versão nova", async () => {
    renderApp("/");
    await screen.findByRole("heading", { level: 1, name: "Início" });
    expect(screen.queryByTestId("dot-settings")).not.toBeInTheDocument();
    act(() => {
      useToolsStore.setState({
        statuses: [
          {
            tool: "ytdlp",
            required: true,
            installed: true,
            version: "1",
            previousVersion: null,
            channel: "stable",
            installedAt: null,
            path: null,
            latestVersion: "2",
            updateAvailable: true,
            lastChecked: null,
          },
        ],
      });
    });
    expect(screen.getByTestId("dot-settings")).toBeInTheDocument();
  });

  it("ponto de atualização também aparece quando o app tem versão nova", async () => {
    renderApp("/");
    await screen.findByRole("heading", { level: 1, name: "Início" });
    act(() => useUpdaterStore.setState({ available: true, version: "9.9.9" }));
    expect(screen.getByTestId("dot-settings")).toBeInTheDocument();
  });

  it("o banner de autocura aparece com heal://state ativo e some ao terminar (cenário heal)", async () => {
    const off = await initHealStore();
    renderApp("/");
    await screen.findByRole("heading", { level: 1, name: "Início" });
    expect(screen.queryByTestId("heal-banner")).not.toBeInTheDocument();
    act(() => mockBus.emit("heal://state", { stage: "checking" }));
    expect(screen.getByTestId("heal-banner")).toHaveTextContent(
      "O YouTube mudou algo — atualizando o yt-dlp…",
    );
    act(() => mockBus.emit("heal://state", { stage: "done" }));
    expect(screen.queryByTestId("heal-banner")).not.toBeInTheDocument();
    off();
  });

  it("Início lista os jobs em andamento (até 3) com barra de progresso", async () => {
    const off = await initJobsStore();
    applyScenario("busy");
    renderApp("/");
    await waitFor(() => expect(screen.getAllByTestId("job-card")).toHaveLength(3));
    expect(screen.getAllByRole("progressbar")).toHaveLength(3);
    off();
  });

  it("Atividade separa em andamento, concluídos e falhas com contadores", async () => {
    const off = await initJobsStore();
    applyScenario("busy");
    applyScenario("errors");
    renderApp("/activity");
    await waitFor(() => expect(screen.getAllByTestId("job-card")).toHaveLength(5));
    expect(screen.getByText("2 ativos · 3 pendentes · 1 concluídos")).toBeInTheDocument();
    await userEvent.click(screen.getByRole("tab", { name: /Falhas/ }));
    expect(screen.getAllByTestId("job-card")).toHaveLength(2);
    await userEvent.click(screen.getByRole("tab", { name: /Concluídos/ }));
    expect(screen.getAllByTestId("job-card")).toHaveLength(1);
    off();
  });
});

describe("Shell: BottomNav (T8)", () => {
  it("existe no DOM (o CSS decide quando aparece) com 4 itens + Mais", async () => {
    renderApp("/");
    await screen.findByRole("heading", { level: 1, name: "Início" });
    const bottom = screen.getByTestId("bottom-nav");
    for (const id of ["home", "library", "playlists", "activity"]) {
      expect(within(bottom).getByTestId(`bottom-nav-${id}`)).toBeInTheDocument();
    }
    await userEvent.click(within(bottom).getByTestId("bottom-nav-more"));
    const dialog = screen.getByRole("dialog", { name: "Mais" });
    expect(within(dialog).getByRole("link", { name: "Revisar" })).toBeInTheDocument();
    await userEvent.click(within(dialog).getByRole("link", { name: "Configurações" }));
    expect(screen.getByTestId("where")).toHaveTextContent("/settings");
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });
});
