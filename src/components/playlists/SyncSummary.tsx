import { useEffect, useState } from "react";
import { Link } from "react-router";
import { useTranslation } from "react-i18next";
import { useSyncsStore } from "@/stores/syncs";

export function SyncSummary() {
  const { t } = useTranslation();
  const syncs = useSyncsStore((state) => state.syncs);
  const [now, setNow] = useState(() => Date.now() / 1000);
  useEffect(() => {
    const timer = setInterval(() => setNow(Date.now() / 1000), 60000);
    return () => clearInterval(timer);
  }, []);
  if (!syncs.length) return null;
  const scheduled = syncs.filter((sync) => sync.enabled && sync.intervalHours > 0);
  const due = Math.min(
    ...scheduled.map((sync) => (sync.lastSyncAt ?? 0) + sync.intervalHours * 3600),
  );
  const next =
    scheduled.length === 0
      ? t("sync.manual")
      : due <= now
        ? t("sync.due")
        : t("sync.nextHours", { count: Math.ceil((due - now) / 3600) });
  return (
    <Link
      to="/playlists"
      className="glass mt-6 flex flex-wrap items-center justify-between gap-2 rounded-lg px-4 py-3 text-xs text-fg-secondary hover:bg-glass-hover"
      data-testid="sync-summary"
    >
      <span>{t("sync.summary", { count: syncs.length })}</span>
      <span className="text-accent">{next}</span>
    </Link>
  );
}
