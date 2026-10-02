import AxeBuilder from "@axe-core/playwright";
import { expect, test, type Page } from "@playwright/test";
import { open, setTheme, goHash, stubThumbnails, waitForRoute } from "./helpers";

// T12 — F08 no navegador com o backend falso: o Preview de FX3 sugere a faixa oficial; o usuário
// pré-visualiza os metadados, edita o artista e baixa; o job leva a edição (`metadata_override`).
const FX3_URL = "https://www.youtube.com/watch?v=dQw4w9WgXcQ";
const FX1_URL = "https://youtu.be/jNQXAC9IVRw";
const OFFICIAL_URL = "https://music.youtube.com/watch?v=lYBUbBu4W08";

const bar = (page: Page) => page.getByRole("textbox", { name: "Barra de comando" });

test.beforeEach(async ({ page }) => {
  await stubThumbnails(page);
});

/** Jobs que o backend falso conhece (a mesma instância do módulo que a página usa). */
async function jobs(page: Page): Promise<
  Array<{
    sourceId: string | null;
    sourceUrl: string;
    metadataOverride: Record<string, unknown> | null;
    confidence: number | null;
    status: string;
  }>
> {
  return page.evaluate(async () => {
    const modulePath = "/src/lib/ipc/mock/index.ts";
    const { mockCall } = await import(/* @vite-ignore */ modulePath);
    return mockCall("jobs_list");
  });
}

async function openPreview(page: Page, url: string) {
  await bar(page).fill(url);
  await bar(page).press("Enter");
  const preview = page.getByRole("dialog", { name: "Pré-visualização" });
  await expect(preview.getByTestId("preview-title")).toBeVisible();
  return preview;
}

async function expectNoBlockingViolations(page: Page, where: string) {
  const results = await new AxeBuilder({ page }).analyze();
  const blocking = results.violations
    .filter((v) => v.impact === "serious" || v.impact === "critical")
    .map((v) => ({ onde: where, regra: v.id, alvos: v.nodes.slice(0, 3).map((n) => n.target.join(" ")) }));
  expect(blocking, where).toEqual([]);
}

test("T12: FX3 sugere a oficial ⇒ pré-visualizar ⇒ editar o artista ⇒ baixar ⇒ o job leva o override", async ({
  page,
}) => {
  // Sem cenário: a simulação temporizada anda pelos estágios.
  await page.goto("/#/");
  await waitForRoute(page, "/");
  const preview = await openPreview(page, FX3_URL);

  // Sugestão da versão oficial, com o toggle pré-marcado e o porquê.
  const card = preview.getByTestId("official-card");
  await expect(card).toContainText("Versão oficial disponível");
  await expect(card).toContainText("Never Gonna Give You Up · Rick Astley");
  await expect(card).toContainText("Whenever You Need Somebody");
  await expect(card).toContainText("Áudio de estúdio, metadados completos.");
  await expect(card.getByRole("switch", { name: "Baixar a versão oficial" })).toBeChecked();

  // Pré-visualizar: campos, confiança e fonte.
  await preview.getByRole("button", { name: "Pré-visualizar metadados" }).click();
  const panel = preview.getByTestId("metadata-panel");
  await expect(panel.getByTestId("metadata-confidence")).toHaveText("100% de confiança");
  await expect(panel.getByTestId("metadata-source")).toHaveText("Faixa oficial do YouTube Music");
  await expect(panel.getByLabel("Álbum")).toHaveValue("Whenever You Need Somebody");
  await expectNoBlockingViolations(page, "preview com versão oficial e metadados");

  // Edita o artista e baixa.
  await panel.getByLabel("Artista").fill("Artista Editado");
  await expect(panel.getByTestId("metadata-edited")).toBeVisible();
  await preview.getByRole("button", { name: "Baixar agora" }).click();
  await expect(page.getByText("Adicionado à fila")).toBeVisible();
  await expect(preview).toHaveCount(0);

  // O job leva a faixa oficial como fonte e a edição como override.
  await expect.poll(async () => (await jobs(page)).length).toBe(1);
  const [job] = await jobs(page);
  expect(job).toMatchObject({
    sourceId: "lYBUbBu4W08",
    sourceUrl: OFFICIAL_URL,
    metadataOverride: { artist: "Artista Editado" },
  });

  // Ao concluir, a Atividade mostra o selo de confiança (a edição vale 100 %).
  await page.getByTestId("nav-activity").click();
  await waitForRoute(page, "/activity");
  await page.getByRole("tab", { name: /Concluídos/ }).click();
  const done = page.getByTestId("job-card");
  await expect(done).toHaveCount(1, { timeout: 25_000 });
  await expect(done).toContainText("Artista Editado");
  await expect(done.getByTestId("job-confidence")).toHaveText("100%");
  await expect(done.getByTestId("job-confidence")).toHaveAttribute(
    "title",
    "Identificado por: Edição sua",
  );
});

test("T12: com a oficial desmarcada o job baixa o clipe, sem override", async ({ page }) => {
  await open(page, "/", "empty");
  const preview = await openPreview(page, FX3_URL);
  await preview
    .getByTestId("official-card")
    .getByRole("switch", { name: "Baixar a versão oficial" })
    .click();
  await preview.getByRole("button", { name: "Adicionar à fila" }).click();
  await expect(preview).toHaveCount(0);
  await expect.poll(async () => (await jobs(page)).length).toBe(1);
  const [job] = await jobs(page);
  expect(job).toMatchObject({
    sourceId: "dQw4w9WgXcQ",
    sourceUrl: FX3_URL,
    metadataOverride: null,
  });
});

test("T12: vídeo que não é música não mostra a versão oficial e o painel avisa", async ({
  page,
}) => {
  await open(page, "/", "empty");
  const preview = await openPreview(page, FX1_URL);
  await expect(preview.getByTestId("official-card")).toHaveCount(0);
  await preview.getByRole("button", { name: "Pré-visualizar metadados" }).click();
  await expect(preview.getByTestId("metadata-bucket")).toContainText("Não parece música");
  await expect(preview.getByTestId("metadata-confidence")).toHaveCount(0);
});

for (const theme of ["dark", "light"] as const) {
  test(`T12: Preview com versão oficial e metadados — axe (${theme})`, async ({ page }) => {
    await open(page, "/", "empty");
    await setTheme(page, theme);
    await goHash(page, "/");
    const preview = await openPreview(page, FX3_URL);
    await expect(preview.getByTestId("official-card")).toBeVisible();
    await preview.getByRole("button", { name: "Pré-visualizar metadados" }).click();
    await expect(preview.getByTestId("metadata-panel")).toBeVisible();
    await page.waitForFunction(
      () => document.querySelector('[role="dialog"]')?.getAnimations().length === 0,
    );
    await expectNoBlockingViolations(page, `preview F08 (${theme})`);
  });
}
