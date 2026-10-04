import { useEffect, useId, useRef } from "react";
import { useTranslation } from "react-i18next";
import { SearchInput } from "@/components/ui/Input";
import { Skeleton } from "@/components/ui/Skeleton";
import { hintKind } from "@/lib/urlkind";
import { useUiStore } from "@/stores/ui";
import { SearchResults } from "./SearchResults";
import { useCommandBar } from "./useCommandBar";
import { api } from "@/lib/ipc/api";
import { Button } from "@/components/ui/Button";

export interface CommandBarProps {
  /** Foca ao montar (overlay e Início). */
  autoFocus?: boolean;
  large?: boolean;
  /** É a barra do overlay Ctrl+K (só uma das duas reage ao pedido de envio). */
  overlay?: boolean;
  /** A barra cumpriu seu papel (abriu Preview/Coleção): o overlay se fecha. */
  onDone?: () => void;
}

/**
 * Barra de comando (link ou texto de busca). Classifica a entrada em tempo real só como dica
 * visual; no Enter, quem decide é o backend (`url_classify`). O texto é compartilhado entre o
 * Início e o Ctrl+K.
 */
export function CommandBar({ autoFocus, large = true, overlay = false, onDone }: CommandBarProps) {
  const { t } = useTranslation();
  const text = useUiStore((s) => s.commandBarText);
  const setText = useUiStore((s) => s.setCommandBarText);
  const focusTick = useUiStore((s) => s.commandBarFocusTick);
  const submitTick = useUiStore((s) => s.commandBarSubmitTick);
  const ref = useRef<HTMLInputElement>(null);
  const bar = useCommandBar(onDone);
  const hint = hintKind(text);
  const instructionsId = useId();

  // "Colar e baixar": o pedido chega com o texto já na store; envia na renderização seguinte.
  const submitRef = useRef(bar.submit);
  useEffect(() => {
    submitRef.current = bar.submit;
  });
  const seenTick = useRef(submitTick);
  useEffect(() => {
    if (submitTick === seenTick.current) return;
    seenTick.current = submitTick;
    if (overlay === useUiStore.getState().commandBarOpen) void submitRef.current();
    // Só reage ao pedido de envio.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [submitTick]);

  useEffect(() => {
    if (autoFocus || focusTick > 0) ref.current?.focus();
    // Só reage ao pedido de foco (tick) e à montagem.
  }, [focusTick, autoFocus]);

  return (
    <div className="w-full">
      <form
        data-command-bar
        role="search"
        onSubmit={(e) => {
          e.preventDefault();
          void bar.submit();
        }}
      >
        <div className="flex items-center gap-2 max-sm:flex-col max-sm:items-stretch">
          <SearchInput
            ref={ref}
            large={large}
            value={text}
            onChange={(e) => setText(e.target.value)}
            placeholder={t("commandBar.placeholder")}
            aria-label={t("commandBar.label")}
            autoComplete="off"
            spellCheck={false}
            aria-describedby={instructionsId}
            wrapperClassName="min-w-0 flex-1"
          />
          <Button
            type="submit"
            variant="primary"
            size={large ? "lg" : "md"}
            loading={bar.status === "loading"}
            disabled={!text.trim()}
          >
            {t(
              hint === "search" || hint === "empty"
                ? "commandBar.searchAction"
                : "commandBar.analyzeAction",
            )}
          </Button>
        </div>
      </form>
      <p
        data-testid="command-hint"
        id={instructionsId}
        aria-live="polite"
        className="mt-2 min-h-4 px-1 text-xs text-fg-muted"
      >
        {hint !== "empty" ? t(`commandBar.hint_${hint}`) : t("commandBar.instructions")}
      </p>
      {bar.status === "loading" && (
        <div data-testid="command-loading" className="mt-3 flex flex-col gap-2" aria-busy="true">
          <Skeleton className="h-14" />
          <Skeleton className="h-14" />
        </div>
      )}
      {bar.status === "error" && (
        <p role="alert" data-testid="command-error" className="mt-3 px-1 text-[13px] text-error">
          {bar.error}
        </p>
      )}
      {bar.status === "error" && /https?:\/\/[^/]+\.bandcamp\.com\//.test(text) && (
        <Button variant="secondary" onClick={() => void api.openSourceUrl(text)}>
          {t("sourcesOpen")}
        </Button>
      )}
      {bar.status === "results" && <SearchResults bar={bar} />}
    </div>
  );
}
