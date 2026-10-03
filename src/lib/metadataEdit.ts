import type { MetadataFields } from "@/bindings/MetadataFields";
import type { MetadataResult } from "@/bindings/MetadataResult";

/** Campos que o usuário pode editar no painel de pré-visualização de metadados. */
export const EDIT_FIELDS = ["title", "artist", "album", "year", "genre", "trackNo"] as const;
export type EditField = (typeof EDIT_FIELDS)[number];

/** Texto digitado em cada campo (só os que o usuário mexeu). */
export type MetadataEdit = Partial<Record<EditField, string>>;

const NUMERIC: ReadonlySet<EditField> = new Set<EditField>(["year", "trackNo"]);

/** Valor atual de um campo, como texto de input. */
export function fieldText(fields: MetadataFields, key: EditField): string {
  const value = fields[key];
  return value === null || value === undefined ? "" : String(value);
}

/**
 * Os campos que o usuário mudou em relação ao resultado, no formato de `metadata_override`
 * (números como número; texto vazio = "não informado", portanto fora). `null` se nada mudou.
 */
export function buildOverride(
  result: MetadataResult | null,
  edit: MetadataEdit,
): Record<string, unknown> | null {
  if (!result) return null;
  const override: Record<string, unknown> = {};
  for (const key of EDIT_FIELDS) {
    const typed = edit[key];
    if (typed === undefined) continue;
    const text = typed.trim();
    if (text === "" || text === fieldText(result.fields, key)) continue;
    if (NUMERIC.has(key)) {
      const number = Number(text);
      if (Number.isInteger(number) && number > 0) override[key] = number;
    } else {
      override[key] = text;
    }
  }
  return Object.keys(override).length > 0 ? override : null;
}

/** Algum campo foi realmente alterado? */
export function isEdited(result: MetadataResult | null, edit: MetadataEdit): boolean {
  return buildOverride(result, edit) !== null;
}
