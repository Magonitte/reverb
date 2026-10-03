import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import type { LosslessReport } from "@/bindings/LosslessReport";
import { Dialog, Button } from "@/components/ui";
import { api } from "@/lib/ipc/api";
import { errorText } from "@/lib/errors";

export function LosslessDialog({ id, onClose }: { id: number; onClose: () => void }) {
  const { t } = useTranslation();
  const [report, setReport] = useState<LosslessReport | null>(null);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    let active = true;
    api
      .verifyLossless(id)
      .then((r) => {
        if (active) setReport(r);
      })
      .catch((e) => {
        if (active) setError(errorText(t, e));
      });
    return () => {
      active = false;
    };
  }, [id, t]);
  return (
    <Dialog open onClose={onClose} title={t("lossless.title")} closeLabel={t("common.close")}>
      <p className="text-sm text-fg-muted">{t("lossless.heuristic")}</p>
      {error ? (
        <p role="alert">{error}</p>
      ) : report ? (
        <div className="space-y-3">
          <p role="status">{t(`lossless.verdicts.${report.verdict}`)}</p>
          <img
            className="w-full rounded"
            src={`data:image/png;base64,${report.spectrogramPngBase64}`}
            alt={t("lossless.spectrogram")}
          />
          <p className="text-xs text-fg-muted">{report.details}</p>
        </div>
      ) : (
        <p role="status">{t("lossless.checking")}</p>
      )}
      <Button onClick={onClose}>{t("common.close")}</Button>
    </Dialog>
  );
}
