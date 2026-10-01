import { create } from "zustand";
import { onEvent } from "@/lib/ipc/events";

/** Etapas emitidas pela autocura (`heal://state`): enabling_pot, checking, done, failed. */
interface HealState {
  stage: string | null;
}

export const useHealStore = create<HealState>(() => ({ stage: null }));

export function isHealing(stage: string | null): boolean {
  return stage !== null && stage !== "done" && stage !== "failed";
}

export async function initHealStore(): Promise<() => void> {
  return onEvent<{ stage: string }>("heal://state", ({ stage }) =>
    useHealStore.setState({ stage }),
  );
}
