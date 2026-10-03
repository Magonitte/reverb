import { render, screen } from "@testing-library/react";
import { beforeEach, expect, it, vi } from "vitest";
import { LosslessDialog } from "./LosslessDialog";
import { resetFlowTests } from "@/testing/flow";
import { api } from "@/lib/ipc/api";
import { hintKind } from "@/lib/urlkind";
beforeEach(() => {
  resetFlowTests();
  vi.restoreAllMocks();
});
it("shows the probable verdict and spectrogram", async () => {
  render(<LosslessDialog id={1} onClose={() => {}} />);
  expect(await screen.findByText("Provável lossless")).toBeVisible();
  expect(screen.getByRole("img", { name: "Espectrograma do áudio" })).toHaveAttribute(
    "src",
    expect.stringContaining("data:image/png;base64,"),
  );
  expect(screen.getByText(/não garante/)).toBeVisible();
});
it("shows verifier failures instead of a quality verdict", async () => {
  vi.spyOn(api, "verifyLossless").mockRejectedValue({ kind: "ffmpeg" });
  render(<LosslessDialog id={1} onClose={() => {}} />);
  expect(await screen.findByRole("alert")).toBeVisible();
  expect(screen.queryByText("Provável lossless")).not.toBeInTheDocument();
});
it("recognizes source URLs without matching lookalike hosts", () => {
  expect(hintKind("https://archive.org/details/OpenGoldbergVariations")).toBe("collection");
  expect(hintKind("https://artist.bandcamp.com/track/free")).toBe("video");
  expect(hintKind("https://soundcloud.com/artist/track")).toBe("video");
  expect(hintKind("https://www.jamendo.com/track/123")).toBe("video");
  expect(hintKind("https://archive.org.evil.example/details/a")).toBe("unsupported");
});
