import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, expect, it } from "vitest";
import { seedMockLibrary, mockLibraryGet } from "@/lib/ipc/mock/library";
import { mockMetadataSearch } from "@/lib/ipc/mock/metadata";
import { mockCalls } from "@/lib/ipc/mock/media";
import { initLibraryStore, useLibraryStore } from "@/stores/library";
import { renderApp, resetFlowTests } from "@/testing/flow";

let off: () => void;
beforeEach(async () => {
  resetFlowTests();
  off = await initLibraryStore();
});
afterEach(() => off());

it("J/K navigate, 1–5 choose, Enter applies, M dismisses and the badge updates", async () => {
  const candidates = [
    mockMetadataSearch("Rick Astley")[0],
    {
      ...mockMetadataSearch("Rick Astley")[0],
      providerId: "alternative",
      title: "Alternative title",
    },
  ].map((candidate) => ({ candidate, score: 0.8 }));
  expect(candidates.length).toBe(2);
  seedMockLibrary([
    { id: 1, title: "First", needsReview: true, reviewCandidates: candidates },
    { id: 2, title: "Second", needsReview: true, reviewCandidates: candidates },
  ]);
  renderApp("/review");
  await screen.findByRole("heading", { name: "Second" });
  await userEvent.keyboard("j");
  await screen.findByRole("heading", { name: "First" });
  await userEvent.keyboard("k");
  await screen.findByRole("heading", { name: "Second" });
  await userEvent.click(screen.getAllByRole("radio")[0]);
  await userEvent.keyboard("2");
  expect(screen.getAllByRole("radio")[1]).toBeChecked();
  await userEvent.keyboard("{Enter}");
  await waitFor(() =>
    expect(mockCalls.find((call) => call.cmd === "review_apply")?.args).toMatchObject({
      id: 2,
      candidate: candidates[1].candidate,
    }),
  );
  await screen.findByRole("heading", { name: "First" });
  await waitFor(() => expect(useLibraryStore.getState().reviewCount).toBe(1));
  await userEvent.keyboard("m");
  await screen.findByText("Nada para revisar");
  expect(mockLibraryGet(1)?.needsReview).toBe(false);
  await waitFor(() => expect(useLibraryStore.getState().reviewCount).toBe(0));
});

it("manual editing applies through review_apply and clears the review flag", async () => {
  seedMockLibrary([{ id: 1, title: "Uncertain", needsReview: true }]);
  renderApp("/review");
  await screen.findByRole("heading", { name: "Uncertain" });
  await userEvent.click(screen.getByRole("button", { name: "Editar manualmente" }));
  const input = await screen.findByLabelText("Título");
  await userEvent.clear(input);
  await userEvent.type(input, "Manual title");
  await userEvent.click(screen.getByRole("button", { name: "Salvar" }));
  await waitFor(() =>
    expect(mockLibraryGet(1)).toMatchObject({ title: "Manual title", needsReview: false }),
  );
});
