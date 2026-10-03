import { fireEvent, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, expect, it, vi } from "vitest";
import { api } from "@/lib/ipc/api";
import { mockCalls } from "@/lib/ipc/mock/media";
import { mockTagsRead, seedMockLibrary, setMockPickedAudio } from "@/lib/ipc/mock/library";
import { renderApp, resetFlowTests } from "@/testing/flow";

beforeEach(() => {
  vi.restoreAllMocks();
  resetFlowTests();
});
it("separates the artist and title so catalog results can be ranked correctly", async () => {
  seedMockLibrary([
    { filePath: "C:/Music/rick.opus", title: "Never Gonna Give You Up", artist: "Rick Astley" },
  ]);
  const search = vi.spyOn(api, "metadataSearch").mockResolvedValue([]);
  renderApp("/tag-editor?path=C:/Music/rick.opus");
  await screen.findByDisplayValue("Never Gonna Give You Up");
  await userEvent.click(screen.getByRole("button", { name: "Buscar metadados" }));
  expect(search).toHaveBeenCalledWith("Rick Astley - Never Gonna Give You Up");
});
it("shows catalog details and artwork, falls back for broken artwork and applies before saving", async () => {
  renderApp("/tag-editor?path=C:/Music/test.opus");
  await screen.findByLabelText("Título");
  const candidate = {
    provider: "itunes",
    providerId: "123",
    title: "Detailed song",
    artists: ["Artist"],
    album: "Album",
    albumArtist: "Artist",
    year: 2026,
    genre: "Pop",
    trackNo: 2,
    trackTotal: 10,
    discNo: 1,
    durationS: 123,
    coverUrl: "https://example.com/cover.jpg",
    mbRecordingId: "recording",
    isrc: "USAAA2600001",
  };
  vi.spyOn(api, "metadataSearch").mockResolvedValue([candidate]);
  vi.spyOn(api, "artworkFetch").mockRejectedValue({ kind: "network", message: "offline" });
  const write = vi.spyOn(api, "tagsWrite");
  await userEvent.click(screen.getByRole("button", { name: "Buscar metadados" }));
  await userEvent.click(await screen.findByRole("button", { name: "Ver detalhes" }));
  const dialog = screen.getByRole("dialog");
  expect(within(dialog).getByText("USAAA2600001")).toBeInTheDocument();
  expect(within(dialog).getByText("123 segundos")).toBeInTheDocument();
  fireEvent.error(within(dialog).getByRole("img", { name: "Capa do álbum" }));
  expect(within(dialog).getByText("Sem capa")).toBeInTheDocument();
  await userEvent.click(within(dialog).getByRole("button", { name: "Usar metadados" }));
  await waitFor(() => expect(screen.getByLabelText("Título")).toHaveValue("Detailed song"));
  expect(write).not.toHaveBeenCalled();
  await userEvent.click(screen.getByRole("button", { name: "Salvar" }));
  await waitFor(() => expect(write).toHaveBeenCalled());
});

it("shows empty and failed catalog searches and lets the user retry", async () => {
  vi.spyOn(api, "metadataSearch")
    .mockResolvedValueOnce([])
    .mockRejectedValueOnce({ kind: "network", message: "offline" })
    .mockResolvedValueOnce([]);
  renderApp("/tag-editor?path=C:/Music/test.opus");
  await screen.findByLabelText("Título");
  const search = screen.getByRole("button", { name: "Buscar metadados" });
  await userEvent.click(search);
  await screen.findByText(/Nenhum resultado encontrado/);
  await userEvent.click(search);
  await screen.findByRole("alert");
  await userEvent.click(search);
  await waitFor(() => expect(screen.queryByRole("alert")).not.toBeInTheDocument());
});
it("opens an external file, edits tags, chooses artwork, fills metadata and saves", async () => {
  setMockPickedAudio("C:/isolated/external.opus");
  renderApp("/tag-editor");
  await userEvent.click(await screen.findByRole("button", { name: /Abrir arquivo de áudio/ }));
  const title = await screen.findByLabelText("Título");
  expect(title).toHaveValue("external");
  await userEvent.clear(title);
  await userEvent.type(title, "Rick Astley");
  const lyrics = screen.getByRole("textbox", { name: "Letra" });
  await userEvent.type(lyrics, "my lyrics");
  await userEvent.click(screen.getByRole("button", { name: /Escolher arquivo de capa/ }));
  await screen.findByRole("img", { name: "Capa do álbum" });
  await userEvent.click(screen.getByRole("button", { name: "Buscar metadados" }));
  const choices = await screen.findAllByRole("button", { name: "Usar metadados" });
  await userEvent.click(choices[0]);
  await waitFor(() => expect(title).not.toHaveValue("Rick Astley"));
  expect(lyrics).toHaveValue("my lyrics");
  await userEvent.click(screen.getByRole("button", { name: "Salvar" }));
  await waitFor(() => expect(mockCalls.some((call) => call.cmd === "tags_write")).toBe(true));
  const saved = mockTagsRead("C:/isolated/external.opus");
  expect(saved.title).toBe(title.getAttribute("value"));
  expect(saved.lyrics).toBe("my lyrics");
  expect(saved.cover).not.toBeNull();
});
