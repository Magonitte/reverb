import { create } from "zustand";
import type { LibraryItem } from "@/bindings/LibraryItem";
import type { LibraryQuery } from "@/bindings/LibraryQuery";
import { api } from "@/lib/ipc/api";
import { onEvent } from "@/lib/ipc/events";

export const defaultLibraryQuery: Partial<LibraryQuery> = {
  sort: "added_desc",
  dateRange: "all",
  offset: 0,
  limit: 100,
};
let ticket = 0;
interface LibraryState {
  items: LibraryItem[];
  recent: LibraryItem[];
  total: number;
  artists: string[];
  albums: string[];
  reviewCount: number;
  query: Partial<LibraryQuery>;
  loading: boolean;
  error: unknown;
  load: () => Promise<void>;
  setQuery: (patch: Partial<LibraryQuery>) => Promise<void>;
}
export const useLibraryStore = create<LibraryState>((set, get) => ({
  items: [],
  recent: [],
  total: 0,
  artists: [],
  albums: [],
  reviewCount: 0,
  query: defaultLibraryQuery,
  loading: false,
  error: null,
  load: async () => {
    const current = ++ticket;
    const query = get().query;
    set({ loading: true, error: null });
    try {
      const [initialPage, artists, albums, review, recent] = await Promise.all([
        api.libraryList(query),
        api.libraryArtists(),
        api.libraryAlbums(query.artist ?? undefined),
        api.libraryList({ needsReview: true, limit: 1 }),
        api.libraryList({ sort: "added_desc", limit: 8 }),
      ]);
      let page = initialPage;
      if (page.total > 0 && page.offset >= page.total) {
        const offset = Math.floor((page.total - 1) / page.limit) * page.limit;
        page = await api.libraryList({ ...query, offset });
      }
      if (current !== ticket) return;
      set({
        items: page.items,
        recent: recent.items,
        total: page.total,
        artists,
        albums,
        reviewCount: review.total,
        query: { ...query, offset: page.total === 0 ? 0 : page.offset },
        loading: false,
      });
    } catch (error) {
      if (current === ticket) set({ loading: false, error });
    }
  },
  setQuery: async (patch) => {
    set({ query: { ...get().query, offset: 0, ...patch } });
    await get().load();
  },
}));
export function resetLibraryStore(): void {
  ++ticket;
  useLibraryStore.setState({
    items: [],
    recent: [],
    total: 0,
    artists: [],
    albums: [],
    reviewCount: 0,
    query: defaultLibraryQuery,
    loading: false,
    error: null,
  });
}
export async function initLibraryStore(): Promise<() => void> {
  const off = await onEvent("library://changed", () => {
    void useLibraryStore.getState().load();
  });
  await useLibraryStore.getState().load();
  return off;
}
