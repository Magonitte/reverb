import { create } from "zustand";
export type NotificationSound = "glass" | "marimba" | "sparkle" | "custom" | "silent";
export interface NotificationPreferences {
  sound: NotificationSound;
  volume: number;
  customData: string;
  customName: string;
  clipboardSeconds: number;
}
export const DEFAULT_NOTIFICATIONS: NotificationPreferences = {
  sound: "glass",
  volume: 0.55,
  customData: "",
  customName: "",
  clipboardSeconds: 35,
};
const KEY = "reverb.notifications.v1";
function read(): NotificationPreferences {
  try {
    const saved = JSON.parse(localStorage.getItem(KEY) ?? "{}") as Partial<NotificationPreferences>;
    return {
      sound: ["glass", "marimba", "sparkle", "custom", "silent"].includes(saved.sound ?? "")
        ? saved.sound!
        : "glass",
      volume: typeof saved.volume === "number" ? Math.min(1, Math.max(0, saved.volume)) : 0.55,
      customData:
        typeof saved.customData === "string" && saved.customData.startsWith("data:audio/")
          ? saved.customData
          : "",
      customName: typeof saved.customName === "string" ? saved.customName : "",
      clipboardSeconds: [20, 35, 60].includes(saved.clipboardSeconds ?? 0)
        ? saved.clipboardSeconds!
        : 35,
    };
  } catch {
    return DEFAULT_NOTIFICATIONS;
  }
}
export const useNotificationPreferences = create<{
  preferences: NotificationPreferences;
  update: (patch: Partial<NotificationPreferences>) => void;
  reload: () => void;
}>((set, get) => ({
  preferences: read(),
  update: (patch) => {
    const preferences = { ...get().preferences, ...patch };
    localStorage.setItem(KEY, JSON.stringify(preferences));
    set({ preferences });
  },
  reload: () => set({ preferences: read() }),
}));
if (typeof window !== "undefined")
  window.addEventListener("storage", (e) => {
    if (e.key === KEY) useNotificationPreferences.getState().reload();
  });
