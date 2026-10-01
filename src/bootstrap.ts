import { isTauri } from "@/lib/ipc/isTauri";
import { initHealStore } from "@/stores/heal";
import { initJobsStore } from "@/stores/jobs";
import { initNotices } from "@/stores/notices";
import { initSettingsStore } from "@/stores/settings";
import { initToolsStore } from "@/stores/tools";
import { useAppInfoStore } from "@/stores/appInfo";

/** Hidrata as stores e assina os eventos do backend (real ou falso). */
export async function bootstrap(): Promise<void> {
  const useMock = import.meta.env.VITE_REVERB_MOCK === "1" || !isTauri();
  await Promise.all([
    useAppInfoStore.getState().load(),
    initSettingsStore(),
    initJobsStore(),
    initToolsStore(),
    initHealStore(),
    initNotices(),
  ]);
  if (useMock) {
    // Cenário da URL (?scenario=) ou simulação contínua; só depois dos assinantes existirem.
    const { startMock } = await import("@/lib/ipc/mock");
    startMock();
  }
}
