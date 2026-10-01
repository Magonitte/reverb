import { describe, expect, it } from "vitest";
import {
  compareCommandSets,
  extractCommandsList,
  extractMockCommands,
  extractRustCommands,
  extractWrapperCommands,
} from "../check-ipc.mjs";

const RUST = `
  .invoke_handler(tauri::generate_handler![
      commands::app_info,
      commands::settings::settings_get,
      // comentário
      commands::queue::enqueue,
  ])
`;
const API = `
export const COMMANDS = [
  "app_info",
  "settings_get",
  "enqueue",
] as const;
export const api = {
  appInfo: () => call<AppInfo>("app_info"),
  settingsGet: () => call<SettingsView>("settings_get"),
  enqueue: (r: R) => call<Job>("enqueue", { r }),
  list: () => call<Array<Job>>("jobs_list"),
};
`;
const MOCK = `
const handlers: Record<string, (args?: Record<string, unknown>) => unknown> = {
  app_info: (): AppInfo => ({ version: "0.1.0" }),
  settings_get: () => mockSettingsGet(),
  enqueue: (args) => mockEnqueue(args?.request as EnqueueRequest),
};
`;

describe("check:ipc — extração (T13)", () => {
  it("lê os comandos do generate_handler!, ignorando comentários", () => {
    expect(extractRustCommands(RUST)).toEqual(["app_info", "settings_get", "enqueue"]);
  });

  it("lê COMMANDS, os wrappers e as chaves do mock", () => {
    expect(extractCommandsList(API)).toEqual(["app_info", "settings_get", "enqueue"]);
    expect(extractWrapperCommands(API)).toEqual([
      "app_info",
      "settings_get",
      "enqueue",
      "jobs_list",
    ]);
    expect(extractMockCommands(MOCK)).toEqual(["app_info", "settings_get", "enqueue"]);
  });

  it("devolve vazio quando não encontra a estrutura", () => {
    expect(extractRustCommands("fn main() {}")).toEqual([]);
    expect(extractCommandsList("export const x = 1")).toEqual([]);
    expect(extractMockCommands("const other = {}")).toEqual([]);
  });
});

describe("check:ipc — comparação (T13)", () => {
  const same = ["app_info", "settings_get", "enqueue"];

  it("passa quando os quatro conjuntos são iguais (a ordem não importa)", () => {
    expect(
      compareCommandSets({ rust: same, list: [...same].reverse(), wrappers: same, mock: same }),
    ).toEqual([]);
  });

  it("falha com um comando a mais só no mock", () => {
    const problems = compareCommandSets({
      rust: same,
      list: same,
      wrappers: same,
      mock: [...same, "so_no_mock"],
    });
    expect(problems).toHaveLength(1);
    expect(problems[0]).toContain('"so_no_mock"');
    expect(problems[0]).toContain("rust, list, wrappers");
  });

  it("falha com um comando que falta no mock", () => {
    const problems = compareCommandSets({
      rust: same,
      list: same,
      wrappers: same,
      mock: same.slice(0, 2),
    });
    expect(problems).toEqual(['comando "enqueue" falta em: mock']);
  });

  it("falha com um comando novo no Rust sem wrapper nem mock", () => {
    const problems = compareCommandSets({
      rust: [...same, "novo"],
      list: same,
      wrappers: same,
      mock: same,
    });
    expect(problems).toEqual(['comando "novo" falta em: list, wrappers, mock']);
  });

  it("acusa duplicatas dentro de um mesmo conjunto", () => {
    const problems = compareCommandSets({ rust: same, mock: [...same, "enqueue"] });
    expect(problems).toEqual(["comando duplicado em mock: enqueue"]);
  });
});
