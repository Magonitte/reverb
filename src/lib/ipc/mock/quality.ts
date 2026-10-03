import type { UpgradeCandidate } from "@/bindings/UpgradeCandidate";
import { mockLibraryList, mockLibraryGet, seedMockLibrary } from "./library";
import { mockEnqueue } from "./queue";
import { mockSettingsGet } from "./settings";
import { mockBus } from "./bus";
import { mockCalls } from "./media";

export function mockUpgradeScan(ids?: number[]): UpgradeCandidate[] {
  mockCalls.push({ cmd: "upgrade_scan", args: { ids } });
  const settings = mockSettingsGet();
  const available = settings.cookiesSource === "none" ? 129 : 256;
  return mockLibraryList({ limit: 500 })
    .items.filter(
      (i) =>
        i.provider === "youtube" &&
        !i.missing &&
        i.sourceAbrKbps !== null &&
        i.sourceAbrKbps < 200 &&
        available >= i.sourceAbrKbps + 40 &&
        (!ids || ids.includes(i.id)),
    )
    .map((item) => ({ item, availableAbrKbps: available }));
}
export function mockUpgradeEnqueue(ids: number[]) {
  mockCalls.push({ cmd: "upgrade_enqueue", args: { ids } });
  return mockUpgradeScan(ids).map(({ item }) =>
    mockEnqueue({
      url: item.sourceUrl ?? "",
      sourceId: item.sourceId ?? undefined,
      profileId: item.profileId ?? undefined,
      options: { upgradeLibraryId: item.id },
      metadataOverride: null,
      priority: false,
      allowDuplicate: true,
    }),
  );
}
export function mockProviderTest(provider: string) {
  mockCalls.push({ cmd: "provider_test", args: { provider } });
  if (!["spotify", "discogs", "acoustid", "jamendo"].includes(provider))
    throw { kind: "invalid", message: "Unknown provider" };
  if (!mockSettingsGet().secretsStatus[provider as "spotify" | "discogs" | "acoustid" | "jamendo"])
    throw { kind: "provider_key" };
}
export function mockWaveform(path: string) {
  mockCalls.push({ cmd: "waveform", args: { path } });
  if (!path) throw { kind: "file_missing" };
  // A fixed valid PNG fixture is sufficient for the browser; native tests verify 1200×160.
  return "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+jRZkAAAAASUVORK5CYII=";
}
export function mockTrim(path: string, startS: number, endS: number) {
  mockCalls.push({ cmd: "trim_audio", args: { path, startS, endS } });
  const rows = mockLibraryList({ limit: 500 }).items;
  const item = rows.find((i) => i.filePath === path);
  if (
    !Number.isFinite(startS) ||
    !Number.isFinite(endS) ||
    startS < 0 ||
    endS <= startS ||
    endS > (item?.durationS ?? Infinity)
  )
    throw { kind: "invalid" };
  if (item) {
    const original = mockLibraryGet(item.id);
    seedMockLibrary(
      rows.map((i) => (i.id === original?.id ? { ...i, durationS: endS - startS } : i)),
    );
    mockBus.emit("library://changed", {});
  }
}
