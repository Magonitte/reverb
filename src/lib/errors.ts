import type { TFunction } from "i18next";
import type { Job } from "@/bindings/Job";

interface ErrorLike {
  kind?: string;
  message?: string;
  i18nKey?: string;
}

/** Texto traduzido de um erro de comando (`CoreError`): `i18nKey`, depois `errors.<kind>`, depois a mensagem. */
export function errorText(t: TFunction, error: unknown): string {
  const e = (typeof error === "object" && error !== null ? error : {}) as ErrorLike;
  if (e.i18nKey) return t(e.i18nKey, { defaultValue: e.message ?? "" });
  if (e.kind) {
    const key = `errors.${e.kind}`;
    const text = t(key, { defaultValue: "" });
    if (text) return text;
  }
  return e.message ?? t("errors.unknown");
}

/** Erro de um job: a UI traduz `errors.<kind>`; `errorMessage` pode ser ele próprio uma chave i18n. */
export function jobErrorText(t: TFunction, job: Pick<Job, "errorKind" | "errorMessage">): string {
  if (job.errorMessage?.startsWith("errors.")) {
    return t(job.errorMessage, { defaultValue: job.errorMessage });
  }
  return errorText(t, { kind: job.errorKind ?? "unknown", message: job.errorMessage ?? undefined });
}
