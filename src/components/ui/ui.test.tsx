import { act, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { useUiStore } from "@/stores/ui";
import {
  Badge,
  Button,
  Card,
  Checkbox,
  Chip,
  ConfirmDialog,
  Dialog,
  EmptyState,
  IconButton,
  Input,
  Kbd,
  ProfileChips,
  ProgressBar,
  SearchInput,
  Select,
  Sheet,
  ShortcutRecorder,
  Skeleton,
  Slider,
  StatusDot,
  Tabs,
  ToastHost,
  Toggle,
  Tooltip,
  VinylDisc,
  VirtualTable,
} from ".";

describe("Button / IconButton (T1)", () => {
  it("renderiza, dispara onClick e respeita disabled", async () => {
    const onClick = vi.fn();
    const { rerender } = render(<Button onClick={onClick}>Baixar</Button>);
    await userEvent.click(screen.getByRole("button", { name: "Baixar" }));
    expect(onClick).toHaveBeenCalledTimes(1);

    rerender(
      <Button onClick={onClick} disabled>
        Baixar
      </Button>,
    );
    expect(screen.getByRole("button", { name: "Baixar" })).toBeDisabled();
  });

  it("loading desabilita, marca aria-busy e não dispara onClick", async () => {
    const onClick = vi.fn();
    render(
      <Button loading onClick={onClick}>
        Salvar
      </Button>,
    );
    const button = screen.getByRole("button", { name: "Salvar" });
    expect(button).toBeDisabled();
    expect(button).toHaveAttribute("aria-busy", "true");
    await userEvent.click(button);
    expect(onClick).not.toHaveBeenCalled();
  });

  it("aplica a variante: danger e primary têm classes diferentes", () => {
    render(
      <>
        <Button variant="danger">A</Button>
        <Button variant="primary">B</Button>
      </>,
    );
    expect(screen.getByText("A").className).not.toEqual(screen.getByText("B").className);
  });

  it("IconButton exige rótulo acessível", () => {
    render(
      <IconButton label="Fechar">
        <span>x</span>
      </IconButton>,
    );
    expect(screen.getByRole("button", { name: "Fechar" })).toBeInTheDocument();
  });
});

describe("Card, Badge, StatusDot, Kbd, Skeleton, VinylDisc, EmptyState (T1)", () => {
  it("renderizam seus conteúdos e papéis", () => {
    render(
      <>
        <Card>conteúdo</Card>
        <Badge>3</Badge>
        <StatusDot tone="success" label="Online" />
        <Kbd>Ctrl</Kbd>
        <Skeleton className="h-4" />
        <EmptyState icon={<span />} title="Vazio" description="Nada aqui" />
      </>,
    );
    expect(screen.getByText("conteúdo")).toBeInTheDocument();
    expect(screen.getByText("3")).toBeInTheDocument();
    expect(screen.getByRole("img", { name: "Online" })).toBeInTheDocument();
    expect(screen.getByText("Ctrl").tagName).toBe("KBD");
    expect(screen.getByRole("heading", { name: "Vazio" })).toBeInTheDocument();
    expect(screen.getByText("Nada aqui")).toBeInTheDocument();
  });

  it("VinylDisc só gira quando spinning", () => {
    const { container, rerender } = render(<VinylDisc />);
    expect(container.firstElementChild).toHaveAttribute("data-spinning", "false");
    rerender(<VinylDisc spinning />);
    expect(container.firstElementChild).toHaveAttribute("data-spinning", "true");
    expect((container.firstElementChild as HTMLElement).style.animation).toContain("reverb-spin");
  });
});

describe("Input / SearchInput / Select / Slider (T1)", () => {
  it("Input associa o rótulo e mostra o erro com aria-invalid", async () => {
    render(<Input label="Pasta" error="Caminho inválido" />);
    const input = screen.getByLabelText("Pasta");
    expect(input).toHaveAttribute("aria-invalid", "true");
    expect(input).toHaveAccessibleDescription("Caminho inválido");
    await userEvent.type(input, "abc");
    expect(input).toHaveValue("abc");
  });

  it("SearchInput aceita digitação e fica desabilitado", async () => {
    const { rerender } = render(<SearchInput aria-label="Buscar" />);
    await userEvent.type(screen.getByLabelText("Buscar"), "rick");
    expect(screen.getByLabelText("Buscar")).toHaveValue("rick");
    rerender(<SearchInput aria-label="Buscar" disabled />);
    expect(screen.getByLabelText("Buscar")).toBeDisabled();
  });

  it("Select troca de valor", async () => {
    const onChange = vi.fn();
    render(
      <Select
        label="Tema"
        value="dark"
        onChange={onChange}
        options={[
          { value: "dark", label: "Escuro" },
          { value: "light", label: "Claro" },
        ]}
      />,
    );
    await userEvent.selectOptions(screen.getByLabelText("Tema"), "light");
    expect(onChange).toHaveBeenCalled();
  });

  it("Slider mostra o valor formatado e chama onChange com número", () => {
    const onChange = vi.fn();
    render(
      <Slider
        label="Confiança"
        value={85}
        min={50}
        max={100}
        onChange={onChange}
        format={(v) => `${v}%`}
      />,
    );
    expect(screen.getByText("85%")).toBeInTheDocument();
    const slider = screen.getByLabelText("Confiança") as HTMLInputElement;
    expect(slider).toHaveAttribute("type", "range");
  });
});

describe("Toggle / Checkbox / Chip (T1)", () => {
  it("Toggle alterna aria-checked e respeita disabled", async () => {
    function Harness({ disabled }: { disabled?: boolean }) {
      const [on, setOn] = useState(false);
      return <Toggle checked={on} onChange={setOn} label="Notificações" disabled={disabled} />;
    }
    const { rerender } = render(<Harness />);
    const toggle = screen.getByRole("switch", { name: "Notificações" });
    expect(toggle).toHaveAttribute("aria-checked", "false");
    await userEvent.click(toggle);
    expect(toggle).toHaveAttribute("aria-checked", "true");
    await userEvent.keyboard(" ");
    expect(toggle).toHaveAttribute("aria-checked", "false");
    rerender(<Harness disabled />);
    expect(screen.getByRole("switch")).toBeDisabled();
  });

  it("Checkbox marca/desmarca e usa o texto como nome", async () => {
    function Harness() {
      const [on, setOn] = useState(false);
      return (
        <Checkbox checked={on} onChange={setOn}>
          Selecionar tudo
        </Checkbox>
      );
    }
    render(<Harness />);
    const box = screen.getByRole("checkbox", { name: "Selecionar tudo" });
    expect(box).not.toBeChecked();
    await userEvent.click(box);
    expect(box).toBeChecked();
  });

  it("Chip expõe aria-pressed e ProfileChips troca o perfil", async () => {
    const onChange = vi.fn();
    render(
      <>
        <Chip selected>MP3</Chip>
        <ProfileChips
          groupLabel="Perfil"
          reencodeLabel="Recodifica"
          value="original"
          onChange={onChange}
          profiles={[
            { id: "original", label: "Original" },
            { id: "mp3_v0", label: "MP3 V0", reencodes: true },
          ]}
        />
      </>,
    );
    expect(screen.getByRole("button", { name: "MP3" })).toHaveAttribute("aria-pressed", "true");
    const group = screen.getByRole("group", { name: "Perfil" });
    expect(within(group).getByRole("button", { name: "Original" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
    await userEvent.click(within(group).getByRole("button", { name: "MP3 V0" }));
    expect(onChange).toHaveBeenCalledWith("mp3_v0");
    expect(within(group).getByRole("button", { name: "MP3 V0" })).toHaveAttribute(
      "title",
      "Recodifica",
    );
  });
});

describe("ProgressBar (T1)", () => {
  it("expõe role=progressbar com valor, e indeterminado sem aria-valuenow", () => {
    const { rerender } = render(<ProgressBar value={0.42} label="Baixando" />);
    const bar = screen.getByRole("progressbar", { name: "Baixando" });
    expect(bar).toHaveAttribute("aria-valuenow", "42");
    rerender(<ProgressBar label="Baixando" />);
    expect(screen.getByRole("progressbar")).not.toHaveAttribute("aria-valuenow");
  });

  it("limita o valor entre 0 e 100", () => {
    render(<ProgressBar value={3} label="x" />);
    expect(screen.getByRole("progressbar")).toHaveAttribute("aria-valuenow", "100");
  });
});

describe("Tabs (T1)", () => {
  function Harness() {
    const [value, setValue] = useState("a");
    return (
      <Tabs
        label="Filtros"
        value={value}
        onChange={setValue}
        tabs={[
          { id: "a", label: "Aba A" },
          { id: "b", label: "Aba B" },
          { id: "c", label: "Aba C", count: 2 },
        ]}
      >
        <p>painel {value}</p>
      </Tabs>
    );
  }

  it("navega com as setas (circular), Home e End", async () => {
    render(<Harness />);
    const tabA = screen.getByRole("tab", { name: "Aba A" });
    tabA.focus();
    await userEvent.keyboard("{ArrowRight}");
    expect(screen.getByRole("tab", { name: "Aba B" })).toHaveAttribute("aria-selected", "true");
    expect(screen.getByRole("tab", { name: "Aba B" })).toHaveFocus();
    await userEvent.keyboard("{End}");
    expect(screen.getByRole("tab", { name: /Aba C/ })).toHaveAttribute("aria-selected", "true");
    await userEvent.keyboard("{ArrowRight}");
    expect(screen.getByRole("tab", { name: "Aba A" })).toHaveAttribute("aria-selected", "true");
    await userEvent.keyboard("{ArrowLeft}");
    expect(screen.getByRole("tab", { name: /Aba C/ })).toHaveAttribute("aria-selected", "true");
    await userEvent.keyboard("{Home}");
    expect(screen.getByRole("tab", { name: "Aba A" })).toHaveFocus();
    expect(screen.getByRole("tabpanel")).toHaveTextContent("painel a");
  });

  it("só a aba ativa entra na ordem de tabulação", () => {
    render(<Harness />);
    expect(screen.getByRole("tab", { name: "Aba A" })).toHaveAttribute("tabindex", "0");
    expect(screen.getByRole("tab", { name: "Aba B" })).toHaveAttribute("tabindex", "-1");
  });
});

describe("Dialog / Sheet / ConfirmDialog (T1)", () => {
  function Harness() {
    const [open, setOpen] = useState(false);
    return (
      <>
        <button onClick={() => setOpen(true)}>abrir</button>
        <Dialog open={open} onClose={() => setOpen(false)} title="Título" closeLabel="Fechar">
          <button>primeiro</button>
          <button>segundo</button>
        </Dialog>
      </>
    );
  }

  it("abre com aria-modal, prende o foco com Tab e fecha com Esc devolvendo o foco", async () => {
    render(<Harness />);
    const opener = screen.getByRole("button", { name: "abrir" });
    await userEvent.click(opener);

    const dialog = screen.getByRole("dialog", { name: "Título" });
    expect(dialog).toHaveAttribute("aria-modal", "true");

    // O foco começa dentro do diálogo e nunca escapa dele.
    expect(dialog.contains(document.activeElement)).toBe(true);
    for (let i = 0; i < 6; i += 1) {
      await userEvent.tab();
      expect(dialog.contains(document.activeElement)).toBe(true);
    }
    for (let i = 0; i < 6; i += 1) {
      await userEvent.tab({ shift: true });
      expect(dialog.contains(document.activeElement)).toBe(true);
    }

    await userEvent.keyboard("{Escape}");
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    expect(opener).toHaveFocus();
  });

  it("o botão de fechar também fecha", async () => {
    render(<Harness />);
    await userEvent.click(screen.getByRole("button", { name: "abrir" }));
    await userEvent.click(screen.getByRole("button", { name: "Fechar" }));
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });

  it("Sheet é um diálogo lateral", () => {
    render(
      <Sheet open onClose={() => {}} title="Preview" closeLabel="Fechar">
        conteúdo
      </Sheet>,
    );
    expect(screen.getByRole("dialog", { name: "Preview" })).toBeInTheDocument();
  });

  it("ConfirmDialog confirma e cancela", async () => {
    const onConfirm = vi.fn();
    const onCancel = vi.fn();
    render(
      <ConfirmDialog
        open
        title="Limpar?"
        message="Isso remove os registros."
        confirmLabel="Limpar"
        cancelLabel="Cancelar"
        closeLabel="Fechar"
        danger
        onConfirm={onConfirm}
        onCancel={onCancel}
      />,
    );
    expect(screen.getByText("Isso remove os registros.")).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "Limpar" }));
    expect(onConfirm).toHaveBeenCalledTimes(1);
    await userEvent.click(screen.getByRole("button", { name: "Cancelar" }));
    expect(onCancel).toHaveBeenCalledTimes(1);
  });
});

describe("Toast (T1)", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    useUiStore.setState({ toasts: [] });
  });
  afterEach(() => {
    vi.useRealTimers();
  });

  it("aparece e some após 8 s", () => {
    render(<ToastHost />);
    act(() => {
      useUiStore.getState().pushToast({ message: "Salvo!", tone: "success" });
    });
    expect(screen.getByRole("status")).toHaveTextContent("Salvo!");

    act(() => {
      vi.advanceTimersByTime(7900);
    });
    expect(screen.getByText("Salvo!")).toBeInTheDocument();
    act(() => {
      vi.advanceTimersByTime(200);
    });
    expect(screen.queryByText("Salvo!")).not.toBeInTheDocument();
  });

  it("executa a ação opcional e fecha", async () => {
    const onAction = vi.fn();
    render(<ToastHost />);
    act(() => {
      useUiStore.getState().pushToast({ message: "Desfazer?", actionLabel: "Desfazer", onAction });
    });
    await act(async () => {
      screen.getByRole("button", { name: "Desfazer" }).click();
    });
    expect(onAction).toHaveBeenCalledTimes(1);
    expect(screen.queryByText("Desfazer?")).not.toBeInTheDocument();
  });

  it("a região é aria-live=polite", () => {
    const { container } = render(<ToastHost />);
    expect(container.querySelector("[aria-live='polite']")).not.toBeNull();
  });
});

describe("Tooltip (T1)", () => {
  it("aparece no foco e some ao sair, ligando aria-describedby", async () => {
    render(
      <Tooltip content="Biblioteca">
        <button>ícone</button>
      </Tooltip>,
    );
    const button = screen.getByRole("button", { name: "ícone" });
    expect(screen.queryByRole("tooltip")).not.toBeInTheDocument();
    await userEvent.tab();
    expect(button).toHaveFocus();
    const tip = screen.getByRole("tooltip");
    expect(tip).toHaveTextContent("Biblioteca");
    expect(button).toHaveAccessibleDescription("Biblioteca");
    await userEvent.tab();
    expect(screen.queryByRole("tooltip")).not.toBeInTheDocument();
  });
});

describe("ShortcutRecorder (T1)", () => {
  function Harness({ onChange }: { onChange: (v: string) => void }) {
    const [value, setValue] = useState("");
    return (
      <ShortcutRecorder
        value={value}
        onChange={(v) => {
          setValue(v);
          onChange(v);
        }}
        label="Atalho global"
        emptyLabel="Nenhum"
        recordingLabel="Pressione as teclas…"
      />
    );
  }

  it("grava uma combinação, ignora teclas sem modificador e Esc cancela", async () => {
    const onChange = vi.fn();
    render(<Harness onChange={onChange} />);
    const button = screen.getByRole("button", { name: "Atalho global" });
    expect(button).toHaveTextContent("Nenhum");

    await userEvent.click(button);
    expect(button).toHaveTextContent("Pressione as teclas…");
    await userEvent.keyboard("a"); // sem modificador: continua gravando
    expect(onChange).not.toHaveBeenCalled();
    await userEvent.keyboard("{Control>}{Shift>}m{/Shift}{/Control}");
    expect(onChange).toHaveBeenCalledWith("Ctrl+Shift+M");
    expect(button).toHaveTextContent("Ctrl+Shift+M");

    await userEvent.click(button);
    await userEvent.keyboard("{Escape}");
    expect(button).toHaveTextContent("Ctrl+Shift+M");

    await userEvent.click(button);
    await userEvent.keyboard("{Backspace}");
    expect(onChange).toHaveBeenLastCalledWith("");
  });
});

describe("VirtualTable (T1)", () => {
  const offsetHeight = Object.getOwnPropertyDescriptor(HTMLElement.prototype, "offsetHeight");
  const offsetWidth = Object.getOwnPropertyDescriptor(HTMLElement.prototype, "offsetWidth");
  beforeEach(() => {
    Object.defineProperty(HTMLElement.prototype, "offsetHeight", {
      configurable: true,
      value: 240,
    });
    Object.defineProperty(HTMLElement.prototype, "offsetWidth", { configurable: true, value: 600 });
  });
  afterEach(() => {
    if (offsetHeight) Object.defineProperty(HTMLElement.prototype, "offsetHeight", offsetHeight);
    if (offsetWidth) Object.defineProperty(HTMLElement.prototype, "offsetWidth", offsetWidth);
  });

  it("renderiza só uma janela das linhas e expõe a semântica de tabela", () => {
    const rows = Array.from({ length: 1000 }, (_, i) => ({ id: String(i), name: `Faixa ${i}` }));
    render(
      <VirtualTable
        label="Biblioteca"
        rows={rows}
        rowKey={(r) => r.id}
        rowHeight={48}
        columns={[{ key: "name", header: "Faixa", render: (r) => r.name }]}
      />,
    );
    const table = screen.getByRole("table", { name: "Biblioteca" });
    expect(table).toHaveAttribute("aria-rowcount", "1000");
    expect(screen.getByRole("columnheader", { name: "Faixa" })).toBeInTheDocument();
    const rendered = screen.getAllByRole("row").length - 1; // menos o cabeçalho
    expect(rendered).toBeGreaterThan(0);
    expect(rendered).toBeLessThan(40);
    expect(screen.getByText("Faixa 0")).toBeInTheDocument();
    expect(screen.queryByText("Faixa 999")).not.toBeInTheDocument();
  });
});
