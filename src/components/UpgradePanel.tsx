import { useState } from "react";
import { useTranslation } from "react-i18next";
import type { UpgradeCandidate } from "@/bindings/UpgradeCandidate";
import { Card, Button, Checkbox } from "@/components/ui";
import { api } from "@/lib/ipc/api";
import { errorText } from "@/lib/errors";

export function UpgradePanel({ ids }: { ids?: number[] }) {
  const { t } = useTranslation();
  const [busy, setBusy] = useState(false);
  const [candidates, setCandidates] = useState<UpgradeCandidate[] | null>(null);
  const [selected, setSelected] = useState<number[]>([]);
  const [error, setError] = useState("");
  const [queued, setQueued] = useState(false);
  const scan = async () => {
    setBusy(true);
    setError("");
    setQueued(false);
    try {
      const result = await api.upgradeScan(ids);
      setCandidates(result);
      setSelected(result.map((c) => c.item.id));
    } catch (e) {
      setError(errorText(t, e));
    } finally {
      setBusy(false);
    }
  };
  const enqueue = async () => {
    setBusy(true);
    try {
      await api.upgradeEnqueue(selected);
      setQueued(true);
      setCandidates(null);
    } catch (e) {
      setError(errorText(t, e));
    } finally {
      setBusy(false);
    }
  };
  return (
    <Card className="space-y-3">
      <Button disabled={busy} onClick={() => void scan()}>
        {t(busy ? "common.loading" : ids ? "quality.upgrade" : "quality.scanAll")}
      </Button>
      {error && <p role="alert">{error}</p>}
      {queued && <p role="status">{t("quality.queued")}</p>}
      {candidates?.map((c) => (
        <Checkbox
          key={c.item.id}
          label={`${c.item.title}: ${c.item.sourceAbrKbps} → ${c.availableAbrKbps} kbps`}
          checked={selected.includes(c.item.id)}
          onChange={(checked) =>
            setSelected((old) =>
              checked ? [...old, c.item.id] : old.filter((id) => id !== c.item.id),
            )
          }
        />
      ))}
      {candidates && (
        <>
          <p className="text-xs">{t("quality.found", { count: candidates.length })}</p>
          <Button disabled={busy || !selected.length} onClick={() => void enqueue()}>
            {t("quality.enqueue")}
          </Button>
        </>
      )}
    </Card>
  );
}
