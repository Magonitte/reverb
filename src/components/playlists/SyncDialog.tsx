import { useState, type SubmitEvent } from "react";
import { useTranslation } from "react-i18next";
import type { Sync } from "@/bindings/Sync";
import { Button } from "@/components/ui/Button";
import { Checkbox } from "@/components/ui/Checkbox";
import { Dialog } from "@/components/ui/Dialog";
import { Input } from "@/components/ui/Input";
import { Select } from "@/components/ui/Select";
import { api } from "@/lib/ipc/api";
import { errorText } from "@/lib/errors";
import { PROFILE_OPTIONS } from "@/lib/profiles";
import { useSettingsStore } from "@/stores/settings";
import { useSyncsStore } from "@/stores/syncs";

export function SyncDialog({
  sync,
  url: initialUrl = "",
  onClose,
  onSaved,
}: {
  sync?: Sync;
  url?: string;
  onClose: () => void;
  onSaved: (sync: Sync) => void;
}) {
  const { t } = useTranslation();
  const settings = useSettingsStore((state) => state.settings);
  const [url, setUrl] = useState(sync?.url ?? initialUrl);
  const [title, setTitle] = useState(sync?.title ?? "");
  const [profileId, setProfileId] = useState(
    sync?.profileId ?? settings?.defaultProfile ?? "original",
  );
  const [outputDir, setOutputDir] = useState(sync?.outputDir ?? "");
  const [interval, setInterval] = useState(sync?.intervalHours ?? 24);
  const [maxItems, setMaxItems] = useState(sync?.maxItems?.toString() ?? "");
  const [removeDeleted, setRemoveDeleted] = useState(sync?.removeDeleted ?? false);
  const [writeM3u, setWriteM3u] = useState(sync?.writeM3u ?? true);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<unknown>(null);
  const chooseFolder = async () => {
    try {
      const path = await api.pickFolder();
      if (path) setOutputDir(path);
    } catch (error) {
      setError(error);
    }
  };
  const save = async (event: SubmitEvent<HTMLFormElement>) => {
    event.preventDefault();
    setBusy(true);
    setError(null);
    try {
      const options = {
        profileId,
        outputDir: outputDir.trim() || null,
        intervalHours: interval,
        maxItems: maxItems ? Number(maxItems) : null,
        removeDeleted,
        writeM3u,
      };
      const saved = sync
        ? await api.syncUpdate(sync.id, { ...options, title: title.trim(), enabled: sync.enabled })
        : await api.syncCreate({ ...options, url: url.trim(), title: title.trim() || null });
      await useSyncsStore.getState().load();
      onSaved(saved);
    } catch (error) {
      setError(error);
    } finally {
      setBusy(false);
    }
  };
  return (
    <Dialog
      open
      title={t(sync ? "sync.edit" : "sync.create")}
      closeLabel={t("common.close")}
      onClose={() => {
        if (!busy) onClose();
      }}
    >
      <form
        onSubmit={(event) => void save(event)}
        className="flex flex-col gap-4"
        data-testid="sync-form"
      >
        {!sync && (
          <Input
            label={t("sync.url")}
            type="url"
            required
            value={url}
            onChange={(event) => setUrl(event.target.value)}
            data-autofocus
            disabled={busy}
          />
        )}
        <Input
          label={t("sync.name")}
          hint={sync ? undefined : t("sync.nameHint")}
          required={!!sync}
          value={title}
          onChange={(event) => setTitle(event.target.value)}
          disabled={busy}
        />
        <div className="grid grid-cols-2 gap-3 max-sm:grid-cols-1">
          <Select
            label={t("sync.interval")}
            value={interval}
            onChange={(event) => setInterval(Number(event.target.value))}
            disabled={busy}
            options={[
              { value: "0", label: t("sync.manual") },
              ...[6, 12, 24, 168].map((hours) => ({
                value: String(hours),
                label: hours === 168 ? t("sync.weekly") : t("sync.hours", { count: hours }),
              })),
            ]}
          />
          <Select
            label={t("sync.profile")}
            value={profileId}
            onChange={(event) => setProfileId(event.target.value)}
            disabled={busy}
            options={PROFILE_OPTIONS.map((profile) => ({
              value: profile.id,
              label: t(profile.labelKey),
            }))}
          />
        </div>
        <div className="flex items-end gap-2">
          <div className="min-w-0 flex-1">
            <Input
              label={t("sync.folder")}
              placeholder={settings?.outputDir || t("sync.defaultFolder")}
              value={outputDir}
              onChange={(event) => setOutputDir(event.target.value)}
              disabled={busy}
            />
          </div>
          <Button
            type="button"
            variant="secondary"
            disabled={busy}
            onClick={() => void chooseFolder()}
          >
            {t("sync.browse")}
          </Button>
        </div>
        <Input
          label={t("sync.limit")}
          hint={t("sync.limitHint")}
          type="number"
          min={1}
          step={1}
          value={maxItems}
          onChange={(event) => setMaxItems(event.target.value)}
          disabled={busy}
        />
        <Checkbox checked={removeDeleted} onChange={setRemoveDeleted} disabled={busy}>
          {t("sync.removeDeleted")}
        </Checkbox>
        <p className="text-xs text-fg-muted">{t("sync.removeHint")}</p>
        <Checkbox checked={writeM3u} onChange={setWriteM3u} disabled={busy}>
          {t("sync.writeM3u")}
        </Checkbox>
        {error !== null && (
          <p role="alert" className="text-sm text-error">
            {errorText(t, error)}
          </p>
        )}
        {busy && (
          <p role="status" className="text-xs text-fg-muted">
            {t("sync.analyzing")}
          </p>
        )}
        <div className="mt-2 flex justify-end gap-2">
          <Button type="button" variant="secondary" disabled={busy} onClick={onClose}>
            {t("common.cancel")}
          </Button>
          <Button type="submit" disabled={busy}>
            {t(sync ? "common.save" : "sync.create")}
          </Button>
        </div>
      </form>
    </Dialog>
  );
}
