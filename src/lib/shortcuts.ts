/** Atalhos de teclado (plano/02-design.md §5). Lógica pura: o handler global só traduz o evento do DOM. */

export type TargetKind = "text-input" | "button" | "link" | "other";

export interface ShortcutEvent {
  key: string;
  ctrlKey: boolean;
  metaKey: boolean;
  altKey: boolean;
  shiftKey: boolean;
}

export interface ShortcutContext {
  /** Onde está o foco. */
  target: TargetKind;
  /** Há um diálogo/painel aberto (Dialog/Sheet). */
  dialogOpen: boolean;
  /** A barra de comando (Ctrl+K) está aberta. */
  commandBarOpen: boolean;
  /** O foco está na barra de comando (embutida ou overlay). */
  commandBarFocused: boolean;
  commandBarHasText: boolean;
}

export type Action =
  | { type: "navigate"; to: string }
  | { type: "openCommandBar" }
  | { type: "pasteToCommandBar" }
  | { type: "toggleQueue" }
  | { type: "closeDialog" }
  | { type: "closeCommandBar" }
  | { type: "clearCommandBar" }
  | { type: "submitCommandBar" };

export interface ShortcutDef {
  /** Texto exibido (Ctrl é Cmd no macOS, irrelevante aqui). */
  keys: string;
  /** Chave i18n da descrição. */
  descriptionKey: string;
}

export const SHORTCUTS: ShortcutDef[] = [
  { keys: "Ctrl+K", descriptionKey: "shortcuts.commandBar" },
  { keys: "Ctrl+V", descriptionKey: "shortcuts.paste" },
  { keys: "Ctrl+D", descriptionKey: "shortcuts.home" },
  { keys: "Ctrl+Q", descriptionKey: "shortcuts.activity" },
  { keys: "Ctrl+H", descriptionKey: "shortcuts.library" },
  { keys: "Ctrl+,", descriptionKey: "shortcuts.settings" },
  { keys: "Space", descriptionKey: "shortcuts.toggleQueue" },
  { keys: "Esc", descriptionKey: "shortcuts.close" },
  { keys: "Enter", descriptionKey: "shortcuts.submit" },
];

const NAV: Record<string, string> = {
  d: "/",
  q: "/activity",
  h: "/library",
  ",": "/settings",
};

/** Decide a ação de um atalho, ou `null` se o evento não deve ser interceptado. */
export function resolveShortcut(event: ShortcutEvent, ctx: ShortcutContext): Action | null {
  const mod = event.ctrlKey || event.metaKey;
  const key = event.key.length === 1 ? event.key.toLowerCase() : event.key;

  if (mod && !event.altKey && !event.shiftKey) {
    if (key === "k") return { type: "openCommandBar" };
    if (key === "v") return ctx.target === "text-input" ? null : { type: "pasteToCommandBar" };
    const to = NAV[key];
    if (to) return { type: "navigate", to };
    return null;
  }

  if (mod || event.altKey) return null;

  if (key === " ") {
    if (ctx.target !== "other" || ctx.dialogOpen || ctx.commandBarOpen) return null;
    return { type: "toggleQueue" };
  }
  if (key === "Escape") {
    if (ctx.dialogOpen) return { type: "closeDialog" };
    if (ctx.commandBarOpen) return { type: "closeCommandBar" };
    if (ctx.commandBarFocused && ctx.commandBarHasText) return { type: "clearCommandBar" };
    return null;
  }
  if (key === "Enter" && ctx.commandBarFocused) return { type: "submitCommandBar" };
  return null;
}

const TEXT_INPUT_TYPES = new Set([
  "text",
  "search",
  "url",
  "email",
  "password",
  "tel",
  "number",
  "",
]);

/** Classifica o elemento com foco para as regras de atalho. */
export function targetKindOf(element: Element | null): TargetKind {
  if (!element) return "other";
  const tag = element.tagName.toLowerCase();
  if (tag === "textarea" || tag === "select") return "text-input";
  if (tag === "input") {
    return TEXT_INPUT_TYPES.has((element as HTMLInputElement).type) ? "text-input" : "button";
  }
  if ((element as HTMLElement).isContentEditable) return "text-input";
  if (tag === "button" || element.getAttribute("role") === "button") return "button";
  if (element.getAttribute("role") === "switch" || element.getAttribute("role") === "tab") {
    return "button";
  }
  if (tag === "a") return "link";
  return "other";
}

const KEY_NAMES: Record<string, string> = {
  " ": "Space",
  ArrowUp: "Up",
  ArrowDown: "Down",
  ArrowLeft: "Left",
  ArrowRight: "Right",
};

/** Converte um evento num atalho global ("Ctrl+Shift+M"); `null` sem modificador ou só com modificadores. */
export function eventToAccelerator(event: ShortcutEvent): string | null {
  if (["Control", "Shift", "Alt", "Meta"].includes(event.key)) return null;
  if (!(event.ctrlKey || event.metaKey || event.altKey)) return null;
  const parts: string[] = [];
  if (event.ctrlKey || event.metaKey) parts.push("Ctrl");
  if (event.altKey) parts.push("Alt");
  if (event.shiftKey) parts.push("Shift");
  const name =
    KEY_NAMES[event.key] ?? (event.key.length === 1 ? event.key.toUpperCase() : event.key);
  parts.push(name);
  return parts.join("+");
}
