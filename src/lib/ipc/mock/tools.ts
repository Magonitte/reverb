import type { InstallOutcome } from "@/bindings/InstallOutcome";
import type { Tool } from "@/bindings/Tool";
import type { ToolStatus } from "@/bindings/ToolStatus";
import type { UpdateInfo } from "@/bindings/UpdateInfo";
import { mockBus } from "./bus";

const ORDER: Tool[] = ["ytdlp", "deno", "ffmpeg", "fpcalc", "bgutil"];
const REQUIRED: Tool[] = ["ytdlp", "ffmpeg"];
/** Versão mais recente que o GitHub "publica" no backend falso. */
const LATEST: Record<Tool, string> = {
  ytdlp: "2026.10.01",
  deno: "2.9.7",
  ffmpeg: "20260930T190056Z",
  fpcalc: "1.6.1",
  bgutil: "2.0.0",
};
const OLDER: Record<Tool, string> = {
  ytdlp: "2026.08.19",
  deno: "2.9.6",
  ffmpeg: "20260920T190056Z",
  fpcalc: "1.6.0",
  bgutil: "1.9.0",
};

type State = { current: string | null; previous: string | null; latestKnown: string | null };
let state: Record<Tool, State> = fresh();

function fresh(): Record<Tool, State> {
  return Object.fromEntries(
    ORDER.map((tool) => [tool, { current: null, previous: null, latestKnown: null }]),
  ) as Record<Tool, State>;
}

function statusOf(tool: Tool): ToolStatus {
  const s = state[tool];
  const installed = s.current !== null;
  return {
    tool,
    required: REQUIRED.includes(tool),
    installed,
    version: s.current,
    previousVersion: s.previous,
    channel: tool === "ytdlp" && installed ? "stable" : null,
    installedAt: installed ? "2026-10-01T00:00:00Z" : null,
    path: installed ? `C:\\mock\\tools\\${tool}\\${s.current}\\${tool}.exe` : null,
    latestVersion: s.latestKnown,
    updateAvailable: installed && s.latestKnown !== null && s.latestKnown !== s.current,
    lastChecked: s.latestKnown ? "2026-10-01T00:00:00Z" : null,
  };
}

function simulateInstall(tool: Tool, version: string): InstallOutcome {
  for (const phase of ["downloading", "verifying", "extracting", "testing"] as const) {
    mockBus.emit("tools://progress", { tool, phase, percent: phase === "downloading" ? 50 : 100 });
  }
  const previous = state[tool].current;
  state[tool] = { ...state[tool], current: version, previous };
  mockBus.emit("tools://changed", { tool, version });
  return { result: "installed", version, previous };
}

export function mockToolsStatus(): ToolStatus[] {
  return ORDER.map(statusOf);
}

export function mockToolsInstallMissing(): Tool[] {
  const installed: Tool[] = [];
  for (const tool of ["ytdlp", "ffmpeg", "deno"] as Tool[]) {
    if (state[tool].current === null) {
      simulateInstall(tool, LATEST[tool]);
      installed.push(tool);
    }
  }
  return installed;
}

let lastForce = false;

export function mockToolsCheckForce() {
  return lastForce;
}

export function mockToolsCheckUpdates(force = false): UpdateInfo[] {
  lastForce = force;
  const infos: UpdateInfo[] = [];
  for (const tool of ORDER) {
    const s = state[tool];
    if (s.current === null) continue;
    s.latestKnown = LATEST[tool];
    infos.push({
      tool,
      current: s.current,
      latest: LATEST[tool],
      updateAvailable: s.current !== LATEST[tool],
      checkedAt: "2026-10-01T00:00:00Z",
    });
  }
  return infos;
}

export function mockToolsUpdate(tool: Tool): InstallOutcome {
  const current = state[tool].current;
  if (current === LATEST[tool]) return { result: "upToDate", version: current };
  return simulateInstall(tool, LATEST[tool]);
}

export function mockToolsRollback(tool: Tool): string {
  const s = state[tool];
  if (s.previous === null) {
    throw { kind: "no_previous_version", message: `${tool} não tem versão anterior` };
  }
  [s.current, s.previous] = [s.previous, s.current];
  mockBus.emit("tools://changed", { tool, version: s.current });
  return s.current as string;
}

/** Instala uma versão antiga (testes de atualização/rollback no backend falso). */
export function seedMockTool(tool: Tool, old = true) {
  state[tool] = { current: old ? OLDER[tool] : LATEST[tool], previous: null, latestKnown: null };
}

export function seedMockToolVersions(tool: Tool, current: string, previous: string | null) {
  state[tool] = { current, previous, latestKnown: LATEST[tool] };
}

export function resetMockTools() {
  state = fresh();
  lastForce = false;
}
