import { create } from "zustand";
import type { CollectionInfo } from "@/bindings/CollectionInfo";
import type { VideoInfo } from "@/bindings/VideoInfo";

/** O que a barra de comando abriu: o Preview (painel) ou a Coleção (página). */
interface FlowState {
  preview: VideoInfo | null;
  collection: CollectionInfo | null;
  openPreview: (info: VideoInfo) => void;
  closePreview: () => void;
  openCollection: (info: CollectionInfo) => void;
  clearCollection: () => void;
}

export const useFlowStore = create<FlowState>((set) => ({
  preview: null,
  collection: null,
  openPreview: (preview) => set({ preview }),
  closePreview: () => set({ preview: null }),
  openCollection: (collection) => set({ collection }),
  clearCollection: () => set({ collection: null }),
}));
