import type { ImportAnalysis } from "@/bindings/ImportAnalysis";
import { create } from "zustand";
import type { CollectionInfo } from "@/bindings/CollectionInfo";
import type { VideoInfo } from "@/bindings/VideoInfo";

/** O que a barra de comando abriu: o Preview (painel) ou a Coleção (página). */
interface FlowState {
  imported: ImportAnalysis | null;
  openImport: (imported: ImportAnalysis) => void;
  preview: VideoInfo | null;
  collection: CollectionInfo | null;
  collectionUrl: string | null;
  openPreview: (info: VideoInfo) => void;
  closePreview: () => void;
  openCollection: (info: CollectionInfo, url?: string) => void;
  clearCollection: () => void;
}

export const useFlowStore = create<FlowState>((set) => ({
  imported: null,
  openImport: (imported) =>
    set({ imported, collection: null, collectionUrl: imported.collection.url }),
  preview: null,
  collection: null,
  collectionUrl: null,
  openPreview: (preview) => set({ preview }),
  closePreview: () => set({ preview: null }),
  openCollection: (collection, url) =>
    set({ collection, collectionUrl: url ?? null, imported: null }),
  clearCollection: () => set({ collection: null, collectionUrl: null, imported: null }),
}));
