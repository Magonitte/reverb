import { mockBus } from "./bus";
import { mockCalls, mockUrlClassify, mockAnalyze, setMockClipboard } from "./media";
import { mockEnqueue } from "./queue";

export const BOOKMARKLET =
  "javascript:location.href='reverb://add?url='+encodeURIComponent(location.href)";
export function mockBookmarkletCopy() {
  setMockClipboard(BOOKMARKLET);
  mockCalls.push({ cmd: "bookmarklet_copy" });
}
export async function mockDeepLinkTest(input: string) {
  const link = new URL(input);
  const invalid = () => {
    throw { kind: "invalid", message: "Invalid Reverb link" };
  };
  if (
    link.protocol !== "reverb:" ||
    link.username ||
    link.password ||
    link.port ||
    link.hash ||
    !["", "/"].includes(link.pathname)
  )
    invalid();
  if (link.hostname === "open" && !link.search) return;
  if (
    link.hostname !== "add" ||
    link.searchParams.getAll("url").length !== 1 ||
    link.searchParams.getAll("profile").length > 1 ||
    [...link.searchParams.keys()].some((key) => !["url", "profile"].includes(key))
  )
    invalid();
  const profileId = link.searchParams.get("profile") ?? undefined;
  if (
    profileId !== undefined &&
    !["original", "mp3_v0", "mp3_320", "aac_256", "opus_96", "flac"].includes(profileId)
  )
    invalid();
  const kind = mockUrlClassify(link.searchParams.get("url") ?? "");
  if (kind.kind === "video") {
    await mockEnqueue({
      url: kind.url,
      sourceId: kind.sourceId,
      profileId,
      metadataOverride: null,
      priority: false,
      allowDuplicate: false,
    });
  } else if (kind.kind === "collection") {
    const analysis = mockAnalyze(kind.url);
    if (analysis.type !== "collection") return invalid();
    const seen = new Set<string>();
    for (const [index, entry] of analysis.info.entries.entries()) {
      if (seen.has(entry.id) || ["[Private video]", "[Deleted video]"].includes(entry.title ?? ""))
        continue;
      seen.add(entry.id);
      await mockEnqueue({
        metadataOverride: null,
        priority: false,
        allowDuplicate: false,
        url: entry.url ?? `https://www.youtube.com/watch?v=${entry.id}`,
        sourceId: entry.id,
        profileId,
        playlistCtx: {
          playlistTitle: analysis.info.title ?? "",
          playlistId: analysis.info.id ?? "",
          index: index + 1,
        },
      });
    }
  } else return invalid();
  mockBus.emit("notice", { level: "success", i18nKey: "integration.linkAdded" });
}
