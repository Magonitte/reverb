import { useCallback, useEffect, useRef, useState } from "react";
import { useNavigate } from "react-router";
import { useTranslation } from "react-i18next";
import type { SearchResult } from "@/bindings/SearchResult";
import { api, type SearchSource } from "@/lib/ipc/api";
import { errorText } from "@/lib/errors";
import { thumbnailFor } from "@/lib/format";
import { importUrl } from "@/lib/importUrl";
import { useFlowStore } from "@/stores/flow";
import { useUiStore } from "@/stores/ui";

export type BarStatus = "idle" | "loading" | "error" | "results";

export interface CommandBarController {
  status: BarStatus;
  error: string | null;
  results: SearchResult[];
  source: SearchSource;
  /** Id do resultado cujo Preview está sendo aberto. */
  opening: string | null;
  submit: () => Promise<void>;
  changeSource: (source: SearchSource) => void;
  openResult: (result: SearchResult) => Promise<void>;
  downloadResult: (result: SearchResult) => Promise<void>;
  reset: () => void;
}

/**
 * Lógica da barra de comando (§2): classifica no backend e decide entre Preview, Coleção e busca.
 * `onDone` roda quando a barra cumpriu seu papel (o overlay fecha).
 */
export function useCommandBar(onDone?: () => void): CommandBarController {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const text = useUiStore((s) => s.commandBarText);
  const setText = useUiStore((s) => s.setCommandBarText);
  const pushToast = useUiStore((s) => s.pushToast);
  const [status, setStatus] = useState<BarStatus>("idle");
  const [error, setError] = useState<string | null>(null);
  const [results, setResults] = useState<SearchResult[]>([]);
  const [source, setSource] = useState<SearchSource>("ytmusic");
  const [opening, setOpening] = useState<string | null>(null);
  // Descarta respostas de uma busca que já foi substituída (digitou de novo, Esc…).
  const ticket = useRef(0);

  const reset = useCallback(() => {
    ticket.current += 1;
    setStatus("idle");
    setError(null);
    setResults([]);
    setOpening(null);
  }, []);

  // Esc (ou apagar tudo) limpa o estado da barra: ajusta o estado na própria renderização
  // e invalida, num efeito, qualquer resposta ainda em voo.
  const empty = text.trim() === "";
  const [wasEmpty, setWasEmpty] = useState(empty);
  if (empty !== wasEmpty) {
    setWasEmpty(empty);
    if (empty) {
      setStatus("idle");
      setError(null);
      setResults([]);
      setOpening(null);
    }
  }
  useEffect(() => {
    if (empty) ticket.current += 1;
  }, [empty]);

  const fail = useCallback(
    (e: unknown) => {
      setError(errorText(t, e));
      setStatus("error");
    },
    [t],
  );

  const runSearch = useCallback(
    async (query: string, from: SearchSource) => {
      const mine = ++ticket.current;
      setStatus("loading");
      setError(null);
      try {
        const found = await api.search(from, query);
        if (mine !== ticket.current) return;
        setResults(found);
        setStatus("results");
      } catch (e) {
        if (mine === ticket.current) fail(e);
      }
    },
    [fail],
  );

  const analyzeAndOpen = useCallback(
    async (url: string, mine: number) => {
      const analysis = await api.analyze(url);
      if (mine !== ticket.current) return;
      if (analysis.type === "video") {
        useFlowStore.getState().openPreview(analysis.info);
      } else {
        useFlowStore.getState().openCollection(analysis.info, url);
        navigate("/collection");
      }
      setStatus("idle");
      setText("");
      onDone?.();
    },
    [navigate, onDone, setText],
  );

  const submit = useCallback(async () => {
    const input = text.trim();
    if (!input) return;
    const mine = ++ticket.current;
    setStatus("loading");
    setError(null);
    try {
      const imported = importUrl(input);
      if (imported) {
        if (imported.kind === "artist") {
          navigate(`/library?follow=${encodeURIComponent(imported.id)}`);
        } else {
          const analysis = await api.importAnalyze(input);
          if (mine !== ticket.current) return;
          useFlowStore.getState().openImport(analysis);
          navigate("/collection");
        }
        setStatus("idle");
        setText("");
        onDone?.();
        return;
      }
      const kind = await api.urlClassify(input);
      if (mine !== ticket.current) return;
      if (kind.kind === "search") {
        await runSearch(kind.query, source);
      } else if (kind.kind === "unsupported") {
        setError(t("commandBar.unsupported"));
        setStatus("error");
      } else {
        await analyzeAndOpen(kind.url, mine);
      }
    } catch (e) {
      if (mine === ticket.current) fail(e);
    }
  }, [text, source, runSearch, analyzeAndOpen, fail, t, navigate, setText, onDone]);

  const changeSource = useCallback(
    (next: SearchSource) => {
      setSource(next);
      if (status === "results" || status === "loading") void runSearch(text.trim(), next);
    },
    [status, runSearch, text],
  );

  const openResult = useCallback(
    async (result: SearchResult) => {
      const mine = ++ticket.current;
      setOpening(result.id);
      try {
        await analyzeAndOpen(result.url ?? `https://www.youtube.com/watch?v=${result.id}`, mine);
      } catch (e) {
        if (mine === ticket.current) fail(e);
      } finally {
        setOpening(null);
      }
    },
    [analyzeAndOpen, fail],
  );

  const downloadResult = useCallback(
    async (result: SearchResult) => {
      try {
        await api.enqueue({
          url: result.url ?? `https://www.youtube.com/watch?v=${result.id}`,
          sourceId: result.id,
          title: result.title,
          thumbnail: thumbnailFor(result.id),
          durationS: result.duration ?? undefined,
          metadataOverride: null,
          priority: false,
          allowDuplicate: false,
        });
        pushToast({ message: t("toasts.queued"), tone: "success" });
      } catch (e) {
        pushToast({ message: errorText(t, e), tone: "error" });
      }
    },
    [pushToast, t],
  );

  return {
    status,
    error,
    results,
    source,
    opening,
    submit,
    changeSource,
    openResult,
    downloadResult,
    reset,
  };
}
