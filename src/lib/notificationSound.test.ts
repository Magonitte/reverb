import { afterEach, expect, it, vi } from "vitest";
afterEach(() => {
  vi.unstubAllGlobals();
  vi.useRealTimers();
  vi.resetModules();
});
it("does not report an error when a newer preview interrupts a pending playback", async () => {
  let rejectFirst!: (error: Error) => void;
  const first = {
    pause: vi.fn(),
    volume: 0,
    play: () =>
      new Promise<void>((_, reject) => {
        rejectFirst = reject;
      }),
  };
  const second = { pause: vi.fn(), volume: 0, play: vi.fn().mockResolvedValue(undefined) };
  let count = 0;
  vi.stubGlobal(
    "Audio",
    vi.fn(function () {
      return count++ === 0 ? first : second;
    }),
  );
  const { playNotificationSound } = await import("./notificationSound");
  const previous = playNotificationSound(true);
  await playNotificationSound(true);
  rejectFirst(new DOMException("interrupted", "AbortError"));
  await expect(previous).resolves.toBeUndefined();
  expect(first.pause).toHaveBeenCalled();
  expect(second.pause).not.toHaveBeenCalled();
});
it("still reports an actual failure of the current audio", async () => {
  vi.stubGlobal(
    "Audio",
    vi.fn(function () {
      return { pause: vi.fn(), play: vi.fn().mockRejectedValue(new Error("decode")) };
    }),
  );
  const { playNotificationSound } = await import("./notificationSound");
  await expect(playNotificationSound(true)).rejects.toThrow("decode");
});
