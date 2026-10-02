import { useTranslation } from "react-i18next";
import type { MetadataResult } from "@/bindings/MetadataResult";
import type { OfficialMatch } from "@/bindings/OfficialMatch";
import { Badge, type BadgeTone } from "@/components/ui/Badge";
import { Button } from "@/components/ui/Button";
import { Input } from "@/components/ui/Input";
import { Toggle } from "@/components/ui/Toggle";
import { EDIT_FIELDS, fieldText, isEdited, type MetadataEdit } from "@/lib/metadataEdit";

/** Card "Versão oficial disponível" (design §3.2): a faixa de estúdio no lugar do clipe. */
export function OfficialVersionCard({
  official,
  checked,
  onChange,
}: {
  official: OfficialMatch;
  checked: boolean;
  onChange: (value: boolean) => void;
}) {
  const { t } = useTranslation();
  return (
    <section
      aria-labelledby="preview-official"
      data-testid="official-card"
      className="rounded-lg border border-accent/30 bg-accent-muted p-3"
    >
      <div className="flex items-start justify-between gap-3">
        <div className="min-w-0">
          <h4 id="preview-official" className="text-xs font-semibold text-accent">
            {t("preview.official.title")}
          </h4>
          <p className="mt-1 truncate text-[13px] font-medium text-fg" data-testid="official-track">
            {official.title} · {official.artist}
          </p>
          {official.album && (
            <p className="truncate text-xs text-fg-muted" data-testid="official-album">
              {official.album}
            </p>
          )}
          <p className="mt-1 text-xs text-fg-muted">{t("preview.official.reason")}</p>
        </div>
        <Toggle label={t("preview.official.toggle")} checked={checked} onChange={onChange} />
      </div>
    </section>
  );
}

function bucketTone(result: MetadataResult): BadgeTone {
  if (result.bucket === "auto") return "success";
  if (result.bucket === "review") return "warning";
  return "neutral";
}

export interface MetadataSectionProps {
  result: MetadataResult | null;
  loading: boolean;
  error: string | null;
  edit: MetadataEdit;
  onEdit: (edit: MetadataEdit) => void;
  onPreview: () => void;
}

/** "Pré-visualizar metadados": campos, % de confiança, fonte e edição antes de baixar. */
export function MetadataSection({
  result,
  loading,
  error,
  edit,
  onEdit,
  onPreview,
}: MetadataSectionProps) {
  const { t } = useTranslation();
  const percent = result ? Math.round(result.confidence * 100) : 0;
  return (
    <section aria-labelledby="preview-metadata">
      <div className="mb-2 flex items-center justify-between gap-3">
        <h4 id="preview-metadata" className="text-xs font-semibold text-fg-secondary">
          {t("preview.metadata.title")}
        </h4>
        <Button size="sm" loading={loading} onClick={onPreview}>
          {t("preview.metadata.preview")}
        </Button>
      </div>
      {error && (
        <p role="alert" className="text-xs text-error">
          {error}
        </p>
      )}
      {result && (
        <div className="flex flex-col gap-3" data-testid="metadata-panel">
          <div className="flex flex-wrap items-center gap-2">
            {result.contentType === "music" ? (
              <Badge tone={bucketTone(result)} data-testid="metadata-confidence">
                {t("preview.metadata.confidence", { percent })}
              </Badge>
            ) : (
              <Badge tone="neutral">{t("preview.kindOther")}</Badge>
            )}
            <span className="text-xs text-fg-muted" data-testid="metadata-source">
              {t(`preview.metadata.source.${result.source}`, { defaultValue: result.source })}
            </span>
          </div>
          <p className="text-xs text-fg-muted" data-testid="metadata-bucket">
            {result.contentType === "music"
              ? t(`preview.metadata.bucket.${result.bucket}`)
              : t("preview.metadata.notMusic")}
          </p>
          <div className="grid grid-cols-2 gap-3">
            {EDIT_FIELDS.map((key) => (
              <Input
                key={key}
                label={t(`preview.metadata.field.${key}`)}
                inputMode={key === "year" || key === "trackNo" ? "numeric" : undefined}
                value={edit[key] ?? fieldText(result.fields, key)}
                onChange={(event) => onEdit({ ...edit, [key]: event.target.value })}
              />
            ))}
          </div>
          {isEdited(result, edit) && (
            <p className="text-xs text-accent" data-testid="metadata-edited">
              {t("preview.metadata.edited")}
            </p>
          )}
        </div>
      )}
    </section>
  );
}
