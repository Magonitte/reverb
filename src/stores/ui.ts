import { create } from "zustand";

export type ToastTone = "info" | "success" | "warning" | "error";

export interface Toast {
  id: number;
  message: string;
  tone: ToastTone;
  actionLabel?: string;
  onAction?: () => void;
}

interface UiState {
  commandBarOpen: boolean;
  /** Texto da barra de comando (compartilhado entre a embutida no Início e o overlay Ctrl+K). */
  commandBarText: string;
  /** Sobe a cada pedido de foco na barra de comando. */
  commandBarFocusTick: number;
  /** Sobe a cada pedido de envio (ex.: "Colar e baixar"); a barra envia o texto atual. */
  commandBarSubmitTick: number;
  toasts: Toast[];
  openCommandBar: (seed?: string) => void;
  closeCommandBar: () => void;
  setCommandBarText: (text: string) => void;
  requestCommandBarFocus: () => void;
  requestCommandBarSubmit: () => void;
  pushToast: (toast: Omit<Toast, "id" | "tone"> & { tone?: ToastTone }) => number;
  dismissToast: (id: number) => void;
}

let nextToastId = 1;

export const useUiStore = create<UiState>((set) => ({
  commandBarOpen: false,
  commandBarText: "",
  commandBarFocusTick: 0,
  commandBarSubmitTick: 0,
  toasts: [],
  openCommandBar: (seed) =>
    set((s) => ({
      commandBarOpen: true,
      commandBarText: seed ?? s.commandBarText,
      commandBarFocusTick: s.commandBarFocusTick + 1,
    })),
  closeCommandBar: () => set({ commandBarOpen: false, commandBarText: "" }),
  setCommandBarText: (commandBarText) => set({ commandBarText }),
  requestCommandBarFocus: () => set((s) => ({ commandBarFocusTick: s.commandBarFocusTick + 1 })),
  requestCommandBarSubmit: () => set((s) => ({ commandBarSubmitTick: s.commandBarSubmitTick + 1 })),
  pushToast: (toast) => {
    const id = nextToastId++;
    set((s) => ({ toasts: [...s.toasts, { tone: "info", ...toast, id }] }));
    return id;
  },
  dismissToast: (id) => set((s) => ({ toasts: s.toasts.filter((t) => t.id !== id) })),
}));
