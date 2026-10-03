import { expect, test } from "@playwright/test";
import { goHash, open } from "./helpers";

test("F09 T10: prévia ao vivo, validação e configurações persistidas", async ({ page }) => {
  await open(page, "/settings/downloads");
  const input = page.getByLabel("Modelo de nomes de arquivos");
  await expect(page.getByTestId("template-preview")).toContainText("01 - Never Gonna Give You Up.opus");
  await input.fill("{artist}/{title}");
  await expect(page.getByTestId("template-preview")).toHaveText("Rick Astley/Never Gonna Give You Up.opus");
  await page.getByRole("button", { name: "Salvar" }).click();
  await input.fill("{invalid}");
  await expect(page.getByText("Confira as variáveis e as chaves do modelo.")).toBeVisible();
  await expect(page.getByRole("button", { name: "Salvar" })).toBeDisabled();
  await goHash(page, "/settings/metadata");
  await page.getByRole("switch", { name: "Buscar letras no LRCLIB" }).click();
  await expect(page.getByRole("switch", { name: "Buscar letras no LRCLIB" })).not.toBeChecked();
  await goHash(page, "/settings/downloads");
  await expect(input).toHaveValue("{artist}/{title}");
});

test("F09 T10: concluído mostra capa e avisos e permite abrir o arquivo", async ({ page }) => {
  await open(page, "/activity");
  await page.evaluate(async () => {
    const modulePath = "/src/lib/ipc/mock/queue.ts";
    const { seedMockJobs } = await import(/* @vite-ignore */ modulePath);
    seedMockJobs([{ sourceUrl: "https://youtu.be/lYBUbBu4W08", title: "Never Gonna Give You Up", status: "done", stage: "done",
      libraryId: 7, outputPath: "C:/Musicas/faixa.opus", warnings: ["warnings.lyrics"] }]);
  });
  await page.getByRole("tab", { name: /Concluídos/ }).click();
  const image = page.getByTestId("job-cover");
  await expect(image).toHaveAttribute("src", /^data:image/);
  await expect.poll(() => image.evaluate((el) => (el as HTMLImageElement).naturalWidth)).toBe(256);
  await expect(page.getByTestId("job-warnings")).toContainText("Não foi possível buscar a letra.");
  await page.getByRole("button", { name: "Abrir arquivo" }).click();
  await expect.poll(async () => page.evaluate(async () => {
    const modulePath = "/src/lib/ipc/mock/index.ts";
    const { mockCalls } = await import(/* @vite-ignore */ modulePath);
    return mockCalls.filter((call: { cmd: string }) => call.cmd === "library_open_file");
  })).toEqual([{ cmd: "library_open_file", args: "C:/Musicas/faixa.opus" }]);
});
