import { beforeEach, describe, expect, it } from "vitest";
import { api } from "@/lib/ipc/api";
import { mockBus, resetMockSettings } from "@/lib/ipc/mock";
import { initSettingsStore, useSettingsStore } from "./settings";

beforeEach(() => {
  resetMockSettings();
  mockBus.clear();
  useSettingsStore.setState({ settings: null, loading: false, error: null });
});

describe("store de configurações (T14)", () => {
  it("hidrata a partir do backend mock", async () => {
    const stop = await initSettingsStore();
    const { settings } = useSettingsStore.getState();
    expect(settings?.parallelism).toBe(2);
    expect(settings?.language).toBe("pt-BR");
    expect(settings?.secretsStatus.acoustid).toBe(false);
    stop();
  });

  it("update é otimista: a UI muda antes da resposta do backend", async () => {
    await useSettingsStore.getState().load();
    const pending = useSettingsStore.getState().update({ parallelism: 4, theme: "light" });
    expect(useSettingsStore.getState().settings?.parallelism).toBe(4);
    expect(useSettingsStore.getState().settings?.theme).toBe("light");
    await pending;
    expect(useSettingsStore.getState().settings?.parallelism).toBe(4);
  });

  it("faz rollback e relança o erro quando o backend rejeita", async () => {
    await useSettingsStore.getState().load();
    await useSettingsStore.getState().update({ theme: "light" });

    const pending = useSettingsStore.getState().update({ parallelism: 9, theme: "system" });
    expect(useSettingsStore.getState().settings?.parallelism).toBe(9); // otimista
    await expect(pending).rejects.toMatchObject({
      kind: "invalid",
      i18nKey: "errors.settings.parallelism",
    });

    const { settings, error } = useSettingsStore.getState();
    expect(settings?.parallelism).toBe(2);
    expect(settings?.theme).toBe("light"); // voltou ao valor anterior à tentativa
    expect(error?.kind).toBe("invalid");
  });

  it("aplica settings://changed vindo do backend", async () => {
    const stop = await initSettingsStore();
    await api.settingsUpdate({ language: "en", queueLimit: 100 }); // dispara o evento
    expect(useSettingsStore.getState().settings?.language).toBe("en");
    expect(useSettingsStore.getState().settings?.queueLimit).toBe(100);
    stop();
  });

  it("segredos viram apenas secretsStatus", async () => {
    await useSettingsStore.getState().load();
    await useSettingsStore.getState().update({ acoustidKey: "segredo-123456" });
    const settings = useSettingsStore.getState().settings!;
    expect(settings.secretsStatus.acoustid).toBe(true);
    expect(JSON.stringify(settings)).not.toContain("segredo-123456");
  });

  it("reset volta ao padrão", async () => {
    await useSettingsStore.getState().load();
    await useSettingsStore.getState().update({ parallelism: 4 });
    await useSettingsStore.getState().reset();
    expect(useSettingsStore.getState().settings?.parallelism).toBe(2);
  });
});
