import { mockIPC } from "@tauri-apps/api/mocks";
import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { App } from "@/App";

describe("App (T6)", () => {
  it("mostra o nome e a versão vindos do app_info mockado", async () => {
    mockIPC((cmd) => {
      if (cmd === "app_info") return { version: "9.9.9", platform: "windows", arch: "x86_64" };
      return undefined;
    });
    render(<App />);
    expect(screen.getByRole("heading", { name: "Reverb" })).toBeInTheDocument();
    expect(await screen.findByTestId("app-info")).toHaveTextContent("9.9.9 · windows");
  });
});
