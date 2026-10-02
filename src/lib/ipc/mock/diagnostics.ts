import type { DiagnosticReport } from "@/bindings/DiagnosticReport";
import { mockRuntimeChoices, mockToolsStatus } from "./tools";
let last: DiagnosticReport | null = null;
export function resetMockDiagnostics() {
  last = null;
}
export function mockDiagnosticsLast() {
  return last;
}
export function mockDiagnosticsRun(): DiagnosticReport {
  const statuses = mockToolsStatus();
  const toolsReady = ["ytdlp", "ffmpeg"].every((id) =>
    statuses.some((tool) => tool.tool === id && tool.installed),
  );
  const runtimeReady = mockRuntimeChoices().length > 0;
  last = {
    createdAt: Math.floor(Date.now() / 1000),
    items: [
      "ytdlp",
      "ffmpeg",
      "ffprobe",
      "runtime",
      "ytdlpVersion",
      "simulate",
      "outputWrite",
      "diskSpace",
      "database",
      "pot",
    ].map((id) => {
      const missing =
        id === "runtime"
          ? !runtimeReady
          : id === "simulate"
            ? !toolsReady || !runtimeReady
            : ["ytdlp", "ffmpeg", "ffprobe"].includes(id) &&
              !statuses.some(
                (tool) => tool.tool === (id === "ffprobe" ? "ffmpeg" : id) && tool.installed,
              );
      return {
        id,
        level: missing ? "error" : "ok",
        detail: missing ? "Tool not installed" : id === "pot" ? "auto; inactive" : "OK",
      };
    }),
  };
  return last;
}
