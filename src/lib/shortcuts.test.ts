import { describe, expect, it } from "vitest";
import {
  eventToAccelerator,
  resolveShortcut,
  targetKindOf,
  type ShortcutContext,
  type ShortcutEvent,
} from "./shortcuts";

const base: ShortcutContext = {
  target: "other",
  dialogOpen: false,
  commandBarOpen: false,
  commandBarFocused: false,
  commandBarHasText: false,
};

function ev(key: string, mods: Partial<ShortcutEvent> = {}): ShortcutEvent {
  return { key, ctrlKey: false, metaKey: false, altKey: false, shiftKey: false, ...mods };
}
const ctrl = (key: string) => ev(key, { ctrlKey: true });

describe("resolveShortcut — tabela do design §5 (T2)", () => {
  it("Ctrl+K abre a barra de comando, até dentro de campos", () => {
    expect(resolveShortcut(ctrl("k"), base)).toEqual({ type: "openCommandBar" });
    expect(resolveShortcut(ctrl("K"), { ...base, target: "text-input" })).toEqual({
      type: "openCommandBar",
    });
  });

  it("Ctrl+D / Ctrl+Q / Ctrl+H / Ctrl+, navegam para as rotas certas", () => {
    expect(resolveShortcut(ctrl("d"), base)).toEqual({ type: "navigate", to: "/" });
    expect(resolveShortcut(ctrl("q"), base)).toEqual({ type: "navigate", to: "/activity" });
    expect(resolveShortcut(ctrl("h"), base)).toEqual({ type: "navigate", to: "/library" });
    expect(resolveShortcut(ctrl(","), base)).toEqual({ type: "navigate", to: "/settings" });
  });

  it("Cmd faz o papel de Ctrl", () => {
    expect(resolveShortcut(ev("k", { metaKey: true }), base)).toEqual({ type: "openCommandBar" });
  });

  it("Ctrl+V fora de campos leva ao Início e cola na barra", () => {
    expect(resolveShortcut(ctrl("v"), base)).toEqual({ type: "pasteToCommandBar" });
    expect(resolveShortcut(ctrl("v"), { ...base, target: "button" })).toEqual({
      type: "pasteToCommandBar",
    });
  });

  it("Ctrl+V dentro de input NÃO é interceptado", () => {
    expect(resolveShortcut(ctrl("v"), { ...base, target: "text-input" })).toBeNull();
  });

  it("Espaço pausa/retoma a fila quando o foco não é de um controle", () => {
    expect(resolveShortcut(ev(" "), base)).toEqual({ type: "toggleQueue" });
  });

  it("Espaço não pausa dentro de input, botão ou link", () => {
    for (const target of ["text-input", "button", "link"] as const) {
      expect(resolveShortcut(ev(" "), { ...base, target })).toBeNull();
    }
  });

  it("Espaço não pausa com diálogo ou barra de comando abertos", () => {
    expect(resolveShortcut(ev(" "), { ...base, dialogOpen: true })).toBeNull();
    expect(resolveShortcut(ev(" "), { ...base, commandBarOpen: true })).toBeNull();
  });

  it("Esc fecha o diálogo; senão fecha a barra; senão limpa o texto; senão nada", () => {
    expect(resolveShortcut(ev("Escape"), { ...base, dialogOpen: true })).toEqual({
      type: "closeDialog",
    });
    expect(
      resolveShortcut(ev("Escape"), { ...base, dialogOpen: true, commandBarOpen: true }),
    ).toEqual({ type: "closeDialog" });
    expect(resolveShortcut(ev("Escape"), { ...base, commandBarOpen: true })).toEqual({
      type: "closeCommandBar",
    });
    expect(
      resolveShortcut(ev("Escape"), {
        ...base,
        target: "text-input",
        commandBarFocused: true,
        commandBarHasText: true,
      }),
    ).toEqual({ type: "clearCommandBar" });
    expect(
      resolveShortcut(ev("Escape"), { ...base, commandBarFocused: true, commandBarHasText: false }),
    ).toBeNull();
    expect(resolveShortcut(ev("Escape"), base)).toBeNull();
  });

  it("Enter envia a barra só quando ela está focada", () => {
    expect(resolveShortcut(ev("Enter"), { ...base, commandBarFocused: true })).toEqual({
      type: "submitCommandBar",
    });
    expect(resolveShortcut(ev("Enter"), base)).toBeNull();
  });

  it("combinações com modificadores extras ou teclas soltas não são atalhos", () => {
    expect(resolveShortcut(ev("k", { ctrlKey: true, shiftKey: true }), base)).toBeNull();
    expect(resolveShortcut(ev("d", { ctrlKey: true, altKey: true }), base)).toBeNull();
    expect(resolveShortcut(ev("d"), base)).toBeNull();
    expect(resolveShortcut(ev(" ", { altKey: true }), base)).toBeNull();
    expect(resolveShortcut(ctrl("x"), base)).toBeNull();
  });
});

describe("targetKindOf", () => {
  const make = (html: string) => {
    const host = document.createElement("div");
    host.innerHTML = html;
    document.body.append(host);
    return host.firstElementChild as HTMLElement;
  };

  it("classifica os elementos com foco", () => {
    expect(targetKindOf(null)).toBe("other");
    expect(targetKindOf(make('<input type="text">'))).toBe("text-input");
    expect(targetKindOf(make('<input type="search">'))).toBe("text-input");
    expect(targetKindOf(make("<textarea></textarea>"))).toBe("text-input");
    expect(targetKindOf(make("<select></select>"))).toBe("text-input");
    expect(targetKindOf(make('<input type="checkbox">'))).toBe("button");
    expect(targetKindOf(make("<button></button>"))).toBe("button");
    expect(targetKindOf(make('<div role="switch"></div>'))).toBe("button");
    expect(targetKindOf(make('<a href="#x">x</a>'))).toBe("link");
    expect(targetKindOf(make("<div></div>"))).toBe("other");
  });
});

describe("eventToAccelerator", () => {
  it("monta o atalho global a partir do evento", () => {
    expect(eventToAccelerator(ev("m", { ctrlKey: true, shiftKey: true }))).toBe("Ctrl+Shift+M");
    expect(eventToAccelerator(ev(" ", { altKey: true }))).toBe("Alt+Space");
    expect(eventToAccelerator(ev("ArrowUp", { ctrlKey: true }))).toBe("Ctrl+Up");
    expect(eventToAccelerator(ev("F5", { ctrlKey: true }))).toBe("Ctrl+F5");
  });

  it("exige modificador e ignora teclas só de modificador", () => {
    expect(eventToAccelerator(ev("m"))).toBeNull();
    expect(eventToAccelerator(ev("Control", { ctrlKey: true }))).toBeNull();
    expect(eventToAccelerator(ev("Shift", { shiftKey: true }))).toBeNull();
  });
});
