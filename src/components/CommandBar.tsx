import { useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";
import { SearchInput } from "@/components/ui/Input";
import { useUiStore } from "@/stores/ui";

export interface CommandBarProps {
  /** Foca ao montar (overlay e Início). */
  autoFocus?: boolean;
  large?: boolean;
}

/**
 * Barra de comando (link ou texto de busca). Nesta fase só guarda o texto e recebe foco;
 * a análise de URL e a busca entram na F07. O texto é compartilhado entre o Início e o Ctrl+K.
 */
export function CommandBar({ autoFocus, large = true }: CommandBarProps) {
  const { t } = useTranslation();
  const text = useUiStore((s) => s.commandBarText);
  const setText = useUiStore((s) => s.setCommandBarText);
  const focusTick = useUiStore((s) => s.commandBarFocusTick);
  const ref = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (autoFocus || focusTick > 0) ref.current?.focus();
    // Só reage ao pedido de foco (tick) e à montagem.
  }, [focusTick, autoFocus]);

  return (
    <form data-command-bar role="search" onSubmit={(e) => e.preventDefault()} className="w-full">
      <SearchInput
        ref={ref}
        large={large}
        value={text}
        onChange={(e) => setText(e.target.value)}
        placeholder={t("commandBar.placeholder")}
        aria-label={t("commandBar.label")}
        autoComplete="off"
        spellCheck={false}
      />
    </form>
  );
}
