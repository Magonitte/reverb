import { useEffect, useState } from "react";
import { FolderOpen } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { JobOptions } from "@/bindings/JobOptions";
import type { SettingsView } from "@/bindings/SettingsView";
import type { VideoInfo } from "@/bindings/VideoInfo";
import { Badge } from "@/components/ui/Badge";
import { Button } from "@/components/ui/Button";
import { ProfileChips } from "@/components/ui/Chip";
import { ConfirmDialog } from "@/components/ui/ConfirmDialog";
import { Sheet } from "@/components/ui/Sheet";
import { Toggle } from "@/components/ui/Toggle";
import { VinylDisc } from "@/components/ui/VinylDisc";
import { errorText } from "@/lib/errors";
import { formatDuration, sourceQuality } from "@/lib/format";
import { api } from "@/lib/ipc/api";
import { PROFILE_OPTIONS, profileReencodes } from "@/lib/profiles";
import { useFlowStore } from "@/stores/flow";
import { useSettingsStore } from "@/stores/settings";
import { useUiStore } from "@/stores/ui";

type ToggleKey = "fetchMetadata" | "fetchLyrics" | "fetchArtwork" | "sponsorblock" | "autoOrganize";

/** Chave da opção por job ⇒ chave da configuração global correspondente. */
const OPTION_SETTING: Record<ToggleKey, keyof SettingsView> = {
  fetchMetadata: "fetchMetadata",
  fetchLyrics: "fetchLyrics",
  fetchArtwork: "fetchArtwork",
  sponsorblock: "sponsorblockRemove",
  autoOrganize: "autoOrganize",
};

const TOGGLES: ToggleKey[] = [
  "fetchMetadata",
  "fetchLyrics",
  "fetchArtwork",
  "sponsorblock",
  "autoOrganize",
];

function isMusic(info: VideoInfo): boolean {
  return info.isOfficialTrack || info.categories.includes("Music");
}

/** Painel de Preview (design §3.2), no ponto da F07: sem versão oficial/metadados/capítulos (F08/F13). */
export function PreviewSheet() {
  const info = useFlowStore((s) => s.preview);
  const close = useFlowStore((s) => s.closePreview);
  if (!info) return null;
  return <PreviewBody key={info.id} info={info} onClose={close} />;
}

function PreviewBody({ info, onClose }: { info: VideoInfo; onClose: () => void }) {
  const { t } = useTranslation();
  const settings = useSettingsStore((s) => s.settings);
  const pushToast = useUiStore((s) => s.pushToast);
  const [profileId, setProfileId] = useState(settings?.defaultProfile ?? "original");
  const [overrides, setOverrides] = useState<JobOptions>({});
  const [outputDir, setOutputDir] = useState<string | null>(null);
  const [duplicate, setDuplicate] = useState(false);
  const [confirmPriority, setConfirmPriority] = useState<boolean | null>(null);
  const [busy, setBusy] = useState(false);

  const url = info.webpageUrl ?? `https://www.youtube.com/watch?v=${info.id}`;
  const quality = sourceQuality(info);
  const reencodes = profileReencodes(profileId);
  const folder = outputDir ?? settings?.outputDir ?? "";

  useEffect(() => {
    let alive = true;
    api
      .checkDuplicates([info.id], profileId)
      .then((hits) => alive && setDuplicate(hits.length > 0))
      .catch(() => alive && setDuplicate(false));
    return () => {
      alive = false;
    };
  }, [info.id, profileId]);

  const optionValue = (key: ToggleKey): boolean =>
    overrides[key] ?? Boolean(settings?.[OPTION_SETTING[key]] ?? false);

  const setOption = (key: ToggleKey, value: boolean) =>
    setOverrides((current) => ({ ...current, [key]: value }));

  const pickFolder = async () => {
    try {
      const picked = await api.pickFolder();
      if (picked) setOutputDir(picked);
    } catch (e) {
      pushToast({ message: errorText(t, e), tone: "error" });
    }
  };

  const enqueue = async (priority: boolean, allowDuplicate: boolean) => {
    setBusy(true);
    try {
      await api.enqueue({
        url,
        sourceId: info.id,
        title: info.track ?? info.title,
        thumbnail: info.thumbnail ?? undefined,
        durationS: info.duration ?? undefined,
        profileId,
        options: { ...overrides, ...(outputDir ? { outputDir } : {}) },
        metadataOverride: null,
        priority,
        allowDuplicate,
      });
      pushToast({ message: t("toasts.queued"), tone: "success" });
      onClose();
    } catch (e) {
      if ((e as { kind?: string }).kind === "duplicate" && !allowDuplicate) {
        setConfirmPriority(priority);
      } else {
        pushToast({ message: errorText(t, e), tone: "error" });
      }
    } finally {
      setBusy(false);
    }
  };

  const submit = (priority: boolean) => {
    if (duplicate) setConfirmPriority(priority);
    else void enqueue(priority, false);
  };

  const artist = info.artist ?? info.channel ?? info.uploader;
  return (
    <>
      <Sheet
        open
        onClose={confirmPriority === null ? onClose : () => undefined}
        title={t("preview.title")}
        closeLabel={t("common.close")}
        footer={
          <>
            <Button variant="secondary" loading={busy} onClick={() => submit(false)}>
              {t("preview.addToQueue")}
            </Button>
            <Button variant="primary" loading={busy} onClick={() => submit(true)} data-autofocus>
              {t("preview.downloadNow")}
            </Button>
          </>
        }
      >
        <div className="flex flex-col gap-5" data-testid="preview">
          <div className="relative aspect-video w-full overflow-hidden rounded-lg bg-field">
            {info.thumbnail ? (
              <img src={info.thumbnail} alt="" className="size-full object-cover" />
            ) : (
              <div className="flex size-full items-center justify-center">
                <VinylDisc size={96} />
              </div>
            )}
          </div>
          <div className="min-w-0">
            <h3 className="font-display text-lg font-semibold text-fg" data-testid="preview-title">
              {info.title}
            </h3>
            <p className="mt-1 truncate text-[13px] text-fg-muted">
              {[artist, formatDuration(info.duration)].filter(Boolean).join(" · ")}
            </p>
            {info.channel && info.channel !== artist && (
              <p className="truncate text-xs text-fg-dim">{info.channel}</p>
            )}
            <div className="mt-3 flex flex-wrap gap-2">
              <Badge tone="neutral">
                {isMusic(info) ? t("preview.kindMusic") : t("preview.kindOther")}
              </Badge>
              {quality && (
                <Badge tone="accent" data-testid="source-quality">
                  {t("preview.sourceQuality", { codec: quality.codec, abr: quality.abr })}
                </Badge>
              )}
              {duplicate && <Badge tone="warning">{t("preview.alreadyDownloaded")}</Badge>}
            </div>
          </div>

          <section aria-labelledby="preview-profile">
            <h4 id="preview-profile" className="mb-2 text-xs font-semibold text-fg-secondary">
              {t("preview.profile")}
            </h4>
            <ProfileChips
              groupLabel={t("preview.profile")}
              reencodeLabel={t("profiles.reencodeWarning")}
              value={profileId}
              onChange={setProfileId}
              profiles={PROFILE_OPTIONS.map((p) => ({
                id: p.id,
                label: t(p.labelKey),
                reencodes: p.reencodes,
              }))}
            />
            {reencodes && (
              <p role="note" className="mt-2 text-xs text-warning">
                {t("profiles.reencodeWarning")}
              </p>
            )}
          </section>

          <section aria-labelledby="preview-options">
            <h4 id="preview-options" className="mb-2 text-xs font-semibold text-fg-secondary">
              {t("preview.options")}
            </h4>
            <ul className="flex flex-col gap-2">
              {TOGGLES.map((key) => (
                <li key={key} className="flex items-center justify-between gap-3">
                  <span className="text-[13px] text-fg-secondary">
                    {t(`preview.option.${key}`)}
                  </span>
                  <Toggle
                    label={t(`preview.option.${key}`)}
                    checked={optionValue(key)}
                    onChange={(v) => setOption(key, v)}
                  />
                </li>
              ))}
            </ul>
          </section>

          <section aria-labelledby="preview-folder">
            <h4 id="preview-folder" className="mb-2 text-xs font-semibold text-fg-secondary">
              {t("preview.folder")}
            </h4>
            <div className="flex items-center gap-2">
              <p
                data-testid="preview-folder-path"
                className="min-w-0 flex-1 truncate rounded-md border border-glass-border bg-field px-3 py-2 text-xs text-fg-muted"
              >
                {folder || t("preview.defaultFolder")}
              </p>
              <Button
                size="sm"
                icon={<FolderOpen className="size-3.5" aria-hidden="true" />}
                onClick={() => void pickFolder()}
              >
                {t("preview.chooseFolder")}
              </Button>
            </div>
          </section>
        </div>
      </Sheet>
      <ConfirmDialog
        open={confirmPriority !== null}
        title={t("preview.duplicate.title")}
        message={t("preview.duplicate.message")}
        confirmLabel={t("preview.duplicate.confirm")}
        cancelLabel={t("common.cancel")}
        closeLabel={t("common.close")}
        onCancel={() => setConfirmPriority(null)}
        onConfirm={() => {
          const priority = confirmPriority ?? false;
          setConfirmPriority(null);
          void enqueue(priority, true);
        }}
      />
    </>
  );
}
