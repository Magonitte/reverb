import { Download, Search } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/Button";
import { EmptyState } from "@/components/ui/EmptyState";
import { Tabs } from "@/components/ui/Tabs";
import { formatDuration, thumbnailFor } from "@/lib/format";
import type { SearchSource } from "@/lib/ipc/api";
import type { CommandBarController } from "./useCommandBar";

/** Resultados de busca com abas YouTube Music | YouTube (design §3.1). */
export function SearchResults({ bar }: { bar: CommandBarController }) {
  const { t } = useTranslation();
  return (
    <Tabs
      className="mt-3"
      label={t("commandBar.sourcesLabel")}
      value={bar.source}
      onChange={(id) => bar.changeSource(id as SearchSource)}
      tabs={[
        { id: "ytmusic", label: t("commandBar.sources.ytmusic") },
        { id: "youtube", label: t("commandBar.sources.youtube") },
      ]}
    >
      {bar.results.length === 0 ? (
        <EmptyState
          icon={<Search aria-hidden="true" />}
          title={t("commandBar.noResults.title")}
          description={t("commandBar.noResults.description")}
        />
      ) : (
        <ul className="flex flex-col gap-2" aria-label={t("commandBar.resultsLabel")}>
          {bar.results.map((r) => (
            <li
              key={r.id}
              data-testid="search-result"
              className="glass flex items-center gap-3 rounded-lg p-2 pr-3"
            >
              <button
                type="button"
                onClick={() => void bar.openResult(r)}
                disabled={bar.opening !== null}
                aria-label={t("commandBar.openPreview", { title: r.title })}
                className="flex min-w-0 flex-1 items-center gap-3 rounded-md text-left"
              >
                <img
                  src={thumbnailFor(r.id)}
                  alt=""
                  loading="lazy"
                  className="size-12 shrink-0 rounded-md bg-field object-cover"
                />
                <span className="min-w-0 flex-1">
                  <span className="block truncate text-[13px] font-medium text-fg">{r.title}</span>
                  <span className="block truncate text-xs text-fg-muted">
                    {[r.channel, formatDuration(r.duration)].filter(Boolean).join(" · ")}
                  </span>
                </span>
              </button>
              <Button
                size="sm"
                variant="secondary"
                icon={<Download className="size-3.5" aria-hidden="true" />}
                aria-label={t("commandBar.downloadItem", { title: r.title })}
                onClick={() => void bar.downloadResult(r)}
              >
                {t("commandBar.download")}
              </Button>
            </li>
          ))}
        </ul>
      )}
    </Tabs>
  );
}
