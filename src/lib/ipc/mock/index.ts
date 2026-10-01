import type { AppInfo } from "@/bindings/AppInfo";

const handlers: Record<string, (args?: Record<string, unknown>) => unknown> = {
  app_info: (): AppInfo => ({ version: "0.1.0", platform: "windows", arch: "x86_64" }),
};

export async function mockCall<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const handler = handlers[cmd];
  if (!handler) throw { kind: "mock", message: `Comando sem mock: ${cmd}` };
  return handler(args) as T;
}
