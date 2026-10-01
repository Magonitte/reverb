import { beforeEach, describe, expect, it } from "vitest";
import type { ToolChanged } from "@/bindings/ToolChanged";
import type { ToolProgress } from "@/bindings/ToolProgress";
import { api } from "@/lib/ipc/api";
import { mockBus, resetMockTools, seedMockTool } from "@/lib/ipc/mock";

beforeEach(() => {
  resetMockTools();
  mockBus.clear();
});

describe("backend mock das ferramentas", () => {
  it("começa sem nada instalado e marca as obrigatórias", async () => {
    const status = await api.toolsStatus();
    expect(status.map((s) => s.tool)).toEqual(["ytdlp", "deno", "ffmpeg", "fpcalc", "bgutil"]);
    expect(status.every((s) => !s.installed)).toBe(true);
    expect(status.filter((s) => s.required).map((s) => s.tool)).toEqual(["ytdlp", "ffmpeg"]);
  });

  it("instalar o que falta emite progresso e tools://changed", async () => {
    const progress: ToolProgress[] = [];
    const changed: ToolChanged[] = [];
    mockBus.on("tools://progress", (p) => progress.push(p as ToolProgress));
    mockBus.on("tools://changed", (c) => changed.push(c as ToolChanged));

    const installed = await api.toolsInstallMissing();
    expect(installed).toEqual(["ytdlp", "ffmpeg", "deno"]);
    expect(changed.map((c) => c.tool)).toEqual(["ytdlp", "ffmpeg", "deno"]);
    expect(progress.filter((p) => p.tool === "ytdlp").map((p) => p.phase)).toEqual([
      "downloading",
      "verifying",
      "extracting",
      "testing",
    ]);

    const status = await api.toolsStatus();
    expect(status.find((s) => s.tool === "ytdlp")?.installed).toBe(true);
    expect(status.find((s) => s.tool === "fpcalc")?.installed).toBe(false);
    expect(await api.toolsInstallMissing()).toEqual([]);
  });

  it("verifica atualizações, atualiza e faz rollback", async () => {
    seedMockTool("ytdlp");
    const infos = await api.toolsCheckUpdates(true);
    expect(infos).toHaveLength(1);
    expect(infos[0]).toMatchObject({ tool: "ytdlp", updateAvailable: true, latest: "2026.10.01" });
    expect((await api.toolsStatus())[0]?.updateAvailable).toBe(true);

    const outcome = await api.toolsUpdate("ytdlp");
    expect(outcome).toEqual({ result: "installed", version: "2026.10.01", previous: "2026.08.19" });
    expect(await api.toolsUpdate("ytdlp")).toEqual({ result: "upToDate", version: "2026.10.01" });

    expect(await api.toolsRollback("ytdlp")).toBe("2026.08.19");
    expect((await api.toolsStatus())[0]?.version).toBe("2026.08.19");
  });

  it("rollback sem versão anterior falha com o mesmo formato de erro do backend", async () => {
    seedMockTool("deno", false);
    await expect(api.toolsRollback("deno")).rejects.toMatchObject({ kind: "no_previous_version" });
  });
});
