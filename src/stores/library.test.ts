import { beforeEach, expect, it, vi } from "vitest";
import { api } from "@/lib/ipc/api";
import type { LibraryPage } from "@/bindings/LibraryPage";
import { mockLibraryList, resetMockLibrary, seedMockLibrary } from "@/lib/ipc/mock/library";
import { resetLibraryStore, useLibraryStore } from "./library";

beforeEach(() => {
  vi.restoreAllMocks();
  resetMockLibrary();
  resetLibraryStore();
});
it("ignores stale responses when a newer search resolves first", async () => {
  let finish!: (page: LibraryPage) => void;
  const delayed = new Promise<LibraryPage>((resolve) => {
    finish = resolve;
  });
  const list = vi.spyOn(api, "libraryList");
  list.mockImplementation((query) =>
    query?.text === "old" ? delayed : Promise.resolve(mockLibraryList(query)),
  );
  const old = useLibraryStore.getState().setQuery({ text: "old" });
  await useLibraryStore.getState().setQuery({ text: "new" });
  finish({ items: [], total: 999, offset: 0, limit: 100 });
  await old;
  expect(useLibraryStore.getState().query.text).toBe("new");
  expect(useLibraryStore.getState().total).toBe(0);
});
it("returns to a valid page after deletion of its final item", async () => {
  seedMockLibrary(Array.from({ length: 101 }, () => ({})));
  await useLibraryStore.getState().setQuery({ offset: 100 });
  await api.libraryDelete([useLibraryStore.getState().items[0]!.id]);
  await useLibraryStore.getState().load();
  expect(useLibraryStore.getState().query.offset).toBe(0);
  expect(useLibraryStore.getState().items).toHaveLength(100);
});
