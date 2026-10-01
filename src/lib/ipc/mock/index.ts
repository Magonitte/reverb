import type { AppInfo } from "@/bindings/AppInfo";
import type { SettingsPatch } from "@/bindings/SettingsPatch";
import { mockSettingsGet, mockSettingsReset, mockSettingsUpdate } from "./settings";

export { mockBus } from "./bus";
export { resetMockSettings } from "./settings";

const handlers: Record<string, (args?: Record<string, unknown>) => unknown> = {
  app_info: (): AppInfo => ({ version: "0.1.0", platform: "windows", arch: "x86_64" }),
  settings_get: () => mockSettingsGet(),
  settings_update: (args) => mockSettingsUpdate((args?.patch ?? {}) as SettingsPatch),
  settings_reset: () => mockSettingsReset(),
};

export async function mockCall<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const handler = handlers[cmd];
  if (!handler) throw { kind: "mock", message: `Comando sem mock: ${cmd}` };
  return handler(args) as T;
}
