import { isTauri } from "@/lib/ipc/isTauri";

/** Controles de janela. No navegador/mock são no-op (a API de janela só existe no Tauri). */
async function withWindow(action: (win: import("@tauri-apps/api/window").Window) => Promise<void>) {
  if (!isTauri() || import.meta.env.VITE_REVERB_MOCK === "1") return;
  const { getCurrentWindow } = await import("@tauri-apps/api/window");
  await action(getCurrentWindow());
}

export const windowControls = {
  minimize: () => withWindow((w) => w.minimize()),
  toggleMaximize: () => withWindow((w) => w.toggleMaximize()),
  close: () => withWindow((w) => w.close()),
};
