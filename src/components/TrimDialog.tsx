import { useEffect, useState, type PointerEvent } from "react";
import { useTranslation } from "react-i18next";
import { Dialog, Input, Button } from "@/components/ui";
import { api } from "@/lib/ipc/api";
import { errorText } from "@/lib/errors";

function seconds(text: string): number {
  if (!/^\d+(?::[0-5]\d(?:\.\d+)?)?$/.test(text.trim())) return NaN;
  const parts = text.split(":").map(Number);
  return parts.reduce((total, value) => total * 60 + value, 0);
}
function time(value: number): string {
  const tenths = Math.floor(value * 10 + 1e-6) / 10;
  return `${Math.floor(tenths / 60)}:${(tenths % 60).toFixed(1).padStart(4, "0")}`;
}
export function TrimDialog({
  path,
  duration,
  onClose,
  onDone,
}: {
  path: string;
  duration: number;
  onClose: () => void;
  onDone: () => void;
}) {
  const { t } = useTranslation();
  const [image, setImage] = useState("");
  const [start, setStart] = useState("0:00.0");
  const [end, setEnd] = useState(time(duration));
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  useEffect(() => {
    let active = true;
    api.waveform(path).then(
      (image) => {
        if (active) setImage(image);
      },
      (e) => {
        if (active) setError(errorText(t, e));
      },
    );
    return () => {
      active = false;
    };
  }, [path, t]);
  const a = seconds(start),
    b = seconds(end);
  const valid = Number.isFinite(a) && Number.isFinite(b) && a >= 0 && b > a && b <= duration;
  const drag = (event: PointerEvent<HTMLButtonElement>, update: (text: string) => void) => {
    if (!event.currentTarget.hasPointerCapture(event.pointerId)) return;
    const box = event.currentTarget.parentElement!.getBoundingClientRect();
    update(
      time(Math.max(0, Math.min(duration, ((event.clientX - box.left) / box.width) * duration))),
    );
  };
  const save = async () => {
    setBusy(true);
    try {
      await api.trimAudio(path, a, b);
      onDone();
      onClose();
    } catch (e) {
      setError(errorText(t, e));
    } finally {
      setBusy(false);
    }
  };
  return (
    <Dialog
      open
      title={t("quality.trim")}
      closeLabel={t("common.close")}
      onClose={() => {
        if (!busy) onClose();
      }}
    >
      <div className="space-y-4">
        {image && (
          <div className="relative aspect-[15/2] overflow-hidden rounded-md">
            <img src={image} alt={t("quality.waveform")} className="size-full object-fill" />
            {(
              [
                { value: a, update: setStart, label: t("quality.start") },
                { value: b, update: setEnd, label: t("quality.end") },
              ] as const
            ).map((marker) => (
              <button
                key={marker.label}
                type="button"
                aria-label={marker.label}
                className="absolute top-0 h-full w-2 -translate-x-1/2 cursor-ew-resize touch-none bg-accent/70 focus-visible:outline-2 focus-visible:outline-accent"
                style={{
                  left: `${Math.max(0, Math.min(100, ((Number.isFinite(marker.value) ? marker.value : 0) / duration) * 100))}%`,
                }}
                onPointerDown={(e) => e.currentTarget.setPointerCapture(e.pointerId)}
                onPointerMove={(e) => drag(e, marker.update)}
                onPointerUp={(e) => e.currentTarget.releasePointerCapture(e.pointerId)}
              />
            ))}
          </div>
        )}
        <Input
          label={t("quality.start")}
          value={start}
          onChange={(e) => setStart(e.target.value)}
        />
        <input
          type="range"
          aria-label={t("quality.start")}
          min={0}
          max={duration}
          step={0.1}
          value={Number.isFinite(a) ? a : 0}
          onChange={(e) => setStart(time(Number(e.target.value)))}
        />
        <Input label={t("quality.end")} value={end} onChange={(e) => setEnd(e.target.value)} />
        <input
          type="range"
          aria-label={t("quality.end")}
          min={0}
          max={duration}
          step={0.1}
          value={Number.isFinite(b) ? b : duration}
          onChange={(e) => setEnd(time(Number(e.target.value)))}
        />
        <p className="text-xs text-fg-muted">{t("quality.trimHint")}</p>
        {error && (
          <p role="alert" className="text-error">
            {error}
          </p>
        )}
        <Button disabled={!valid || busy || !image} onClick={() => void save()}>
          {t(busy ? "common.loading" : "quality.trim")}
        </Button>
      </div>
    </Dialog>
  );
}
