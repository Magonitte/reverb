import { invoke } from "@tauri-apps/api/core";
import type { AppInfo } from "@/bindings/AppInfo";
import { isTauri } from "./isTauri";

export async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (import.meta.env.VITE_REVERB_MOCK === "1" || !isTauri()) {
    const { mockCall } = await import("./mock");
    return mockCall<T>(cmd, args);
  }
  return invoke<T>(cmd, args);
}

export const api = {
  appInfo: () => call<AppInfo>("app_info"),
};
