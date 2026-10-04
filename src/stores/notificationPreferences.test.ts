import { beforeEach, expect, it, vi } from "vitest";
import { DEFAULT_NOTIFICATIONS, useNotificationPreferences } from "./notificationPreferences";
beforeEach(() => {
  localStorage.clear();
  useNotificationPreferences.setState({ preferences: DEFAULT_NOTIFICATIONS });
});
it("persists sound, volume, custom audio and reading time", () => {
  useNotificationPreferences
    .getState()
    .update({
      sound: "custom",
      customData: "data:audio/wav;base64,UklGRg==",
      customName: "Meu som.wav",
      volume: 0.3,
      clipboardSeconds: 60,
    });
  useNotificationPreferences.setState({ preferences: DEFAULT_NOTIFICATIONS });
  useNotificationPreferences.getState().reload();
  expect(useNotificationPreferences.getState().preferences).toMatchObject({
    sound: "custom",
    customName: "Meu som.wav",
    volume: 0.3,
    clipboardSeconds: 60,
  });
});
it("does not claim saved preferences when storage fails", () => {
  const spy = vi.spyOn(Storage.prototype, "setItem").mockImplementation(() => {
    throw new Error("quota");
  });
  expect(() => useNotificationPreferences.getState().update({ sound: "marimba" })).toThrow();
  expect(useNotificationPreferences.getState().preferences.sound).toBe("glass");
  spy.mockRestore();
});
it("rejects invalid persisted choices and clamps volume", () => {
  localStorage.setItem(
    "reverb.notifications.v1",
    JSON.stringify({
      sound: "unknown",
      volume: 9,
      clipboardSeconds: 1,
      customData: "https://external.example/sound.wav",
    }),
  );
  useNotificationPreferences.getState().reload();
  expect(useNotificationPreferences.getState().preferences).toMatchObject({
    sound: "glass",
    volume: 1,
    clipboardSeconds: 35,
    customData: "",
  });
});
