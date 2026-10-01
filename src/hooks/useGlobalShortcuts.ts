import { useEffect } from "react";
import { useNavigate } from "react-router";
import { api } from "@/lib/ipc/api";
import { resolveShortcut, targetKindOf, type Action } from "@/lib/shortcuts";
import { useJobsStore } from "@/stores/jobs";
import { useUiStore } from "@/stores/ui";

/** Há um Dialog/Sheet aberto (a barra de comando tem tratamento próprio). */
function dialogIsOpen(): boolean {
  return (
    document.querySelector('[role="dialog"][aria-modal="true"]:not([data-command-bar-overlay])') !==
    null
  );
}

function commandBarState(): { focused: boolean; hasText: boolean } {
  const active = document.activeElement;
  const focused = active?.closest("[data-command-bar]") != null;
  return { focused, hasText: useUiStore.getState().commandBarText.length > 0 };
}

/** Handler global de atalhos (design §5); a decisão fica em `resolveShortcut`. */
export function useGlobalShortcuts(): void {
  const navigate = useNavigate();

  useEffect(() => {
    const run = (action: Action) => {
      const ui = useUiStore.getState();
      switch (action.type) {
        case "navigate":
          navigate(action.to);
          break;
        case "openCommandBar":
          ui.openCommandBar();
          break;
        case "pasteToCommandBar":
          // O texto chega pelo evento `paste` (abaixo); aqui só levamos o foco para a barra.
          navigate("/");
          ui.requestCommandBarFocus();
          break;
        case "toggleQueue": {
          const paused = useJobsStore.getState().queue.paused;
          void (paused ? api.queueResume() : api.queuePause());
          break;
        }
        case "closeCommandBar":
          ui.closeCommandBar();
          break;
        case "clearCommandBar":
          ui.setCommandBarText("");
          break;
        case "closeDialog":
        case "submitCommandBar":
          // Dialog fecha sozinho com Esc; o formulário da barra trata o Enter.
          break;
      }
    };

    const onKeyDown = (event: KeyboardEvent) => {
      if (event.defaultPrevented || event.isComposing) return;
      const bar = commandBarState();
      const action = resolveShortcut(event, {
        target: targetKindOf(document.activeElement),
        dialogOpen: dialogIsOpen(),
        commandBarOpen: useUiStore.getState().commandBarOpen,
        commandBarFocused: bar.focused,
        commandBarHasText: bar.hasText,
      });
      if (!action) return;
      // Ctrl+V precisa seguir para gerar o `paste`; Enter pertence ao formulário.
      if (action.type !== "pasteToCommandBar" && action.type !== "submitCommandBar") {
        event.preventDefault();
      }
      run(action);
    };

    const onPaste = (event: ClipboardEvent) => {
      if (targetKindOf(document.activeElement) === "text-input" || dialogIsOpen()) return;
      const text = event.clipboardData?.getData("text")?.trim();
      if (!text) return;
      event.preventDefault();
      useUiStore.getState().setCommandBarText(text);
    };

    window.addEventListener("keydown", onKeyDown);
    document.addEventListener("paste", onPaste);
    return () => {
      window.removeEventListener("keydown", onKeyDown);
      document.removeEventListener("paste", onPaste);
    };
  }, [navigate]);
}
