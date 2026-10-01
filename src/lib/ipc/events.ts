import { listen } from "@tauri-apps/api/event";
import { isTauri } from "./isTauri";

export type Unlisten = () => void;

/** Escuta um evento do backend (Tauri) ou do barramento do backend falso (navegador/testes). */
export async function onEvent<T>(name: string, handler: (payload: T) => void): Promise<Unlisten> {
  if (import.meta.env.VITE_REVERB_MOCK === "1" || !isTauri()) {
    const { mockBus } = await import("./mock");
    return mockBus.on(name, (payload) => handler(payload as T));
  }
  return listen<T>(name, (event) => handler(event.payload));
}
