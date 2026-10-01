import { Wrench } from "lucide-react";
import { useTranslation } from "react-i18next";
import { isHealing, useHealStore } from "@/stores/heal";
import { useJobsStore } from "@/stores/jobs";

/** Banner de autocura (`heal://state` ativo ou `queue://state.healing`). */
export function HealBanner() {
  const { t } = useTranslation();
  const stage = useHealStore((s) => s.stage);
  const queueHealing = useJobsStore((s) => s.queue.healing);
  if (!isHealing(stage) && !queueHealing) return null;
  return (
    <div
      role="status"
      data-testid="heal-banner"
      className="glass mb-6 flex items-center gap-3 rounded-lg border-accent/30 px-4 py-3 text-[13px] text-fg"
    >
      <Wrench className="size-4 shrink-0 text-accent" aria-hidden="true" />
      {t("heal.banner")}
    </div>
  );
}
