import { expect, test, type Page } from "@playwright/test";
import { goHash, open, waitForRoute } from "./helpers";

const ALBUM_URL =
  "https://music.youtube.com/playlist?list=OLAK5uy_nmDUsWOMoEcz0SsVqUwir0oxu-k1oUyXE";
const VIDEO_URL = "https://youtu.be/jNQXAC9IVRw";

const bar = (page: Page) => page.getByRole("textbox", { name: "Barra de comando" });

/** Chamadas de sistema registradas pelo backend falso (library_reveal, pick_folder…). */
async function mockCalls(page: Page): Promise<Array<{ cmd: string; args?: unknown }>> {
  return page.evaluate(async () => {
    const modulePath = "/src/lib/ipc/mock/index.ts";
    const { mockCalls } = await import(/* @vite-ignore */ modulePath);
    return [...mockCalls];
  });
}

test("T6: colar URL ⇒ Preview ⇒ Baixar ⇒ estágios ⇒ Concluídos ⇒ abrir pasta", async ({ page }) => {
  // Sem cenário: a simulação temporizada anda pelos estágios.
  await page.goto("/#/");
  await waitForRoute(page, "/");
  await bar(page).fill(VIDEO_URL);
  await bar(page).press("Enter");

  const preview = page.getByRole("dialog", { name: "Pré-visualização" });
  await expect(preview.getByTestId("preview-title")).toHaveText("Me at the zoo");
  await expect(preview.getByTestId("source-quality")).toHaveText("Fonte: Opus 129 kbps");
  await preview.getByRole("button", { name: "Baixar agora" }).click();
  await expect(page.getByText("Adicionado à fila")).toBeVisible();
  await expect(preview).toHaveCount(0);

  await page.getByTestId("nav-activity").click();
  await waitForRoute(page, "/activity");
  const card = page.getByTestId("job-card").first();
  await expect(card).toContainText("Me at the zoo");
  // Passa por estágios diferentes antes de terminar.
  await expect(card).toContainText("Baixando", { timeout: 10_000 });
  await expect(card).toContainText("Convertendo", { timeout: 15_000 });

  await page.getByRole("tab", { name: /Concluídos/ }).click();
  const done = page.getByTestId("job-card");
  await expect(done).toHaveCount(1, { timeout: 20_000 });
  await done.getByRole("button", { name: "Mostrar Me at the zoo na pasta" }).click();
  await expect
    .poll(async () => (await mockCalls(page)).filter((c) => c.cmd === "library_reveal"))
    .toEqual([{ cmd: "library_reveal", args: { path: "C:/Musicas/Reverb/Me at the zoo.opus" } }]);
});

test("T7: playlist FX4 ⇒ selecionar 3 ⇒ 3 jobs na Atividade", async ({ page }) => {
  await open(page, "/", "empty");
  await bar(page).fill(ALBUM_URL);
  await bar(page).press("Enter");
  await waitForRoute(page, "/collection");
  await expect(
    page.getByRole("heading", { level: 1, name: /Whenever You Need Somebody/ }),
  ).toBeVisible();

  for (const title of ["Never Gonna Give You Up", "Together Forever", "The Love Has Gone"]) {
    await page.getByRole("checkbox", { name: `Selecionar ${title}` }).check();
  }
  const download = page.getByRole("button", { name: "Baixar selecionadas (3)" });
  await expect(download).toBeVisible();
  await download.click();
  await expect(page.getByText("3 faixas adicionadas à fila")).toBeVisible();

  await page.getByTestId("nav-activity").click();
  await waitForRoute(page, "/activity");
  await expect(page.getByTestId("job-card")).toHaveCount(3);
  await expect(page.getByTestId("badge-activity")).toHaveText("3");
});

test("T7: reordenar arrastando a alça de um item pendente", async ({ page }) => {
  await open(page, "/activity", "busy");
  const titles = () =>
    page.getByTestId("job-card").locator("p.truncate:first-of-type").allTextContents();
  // 2 em andamento (não arrastáveis) + 3 pendentes.
  expect(await titles()).toEqual([
    "Never Gonna Give You Up",
    "Rick Astley - Never Gonna Give You Up (Official Video) (4K Remaster)",
    "Never Gonna Give You Up",
    "Whenever You Need Somebody",
    "Together Forever",
  ]);

  const handle = page.getByRole("button", { name: "Reordenar Together Forever" });
  const first = page.getByRole("button", { name: "Reordenar Never Gonna Give You Up" });
  const from = (await handle.boundingBox())!;
  const to = (await first.boundingBox())!;
  await page.mouse.move(from.x + from.width / 2, from.y + from.height / 2);
  await page.mouse.down();
  await page.mouse.move(from.x + from.width / 2, from.y - 20, { steps: 5 });
  await page.mouse.move(to.x + to.width / 2, to.y - 4, { steps: 15 });
  await page.mouse.up();

  await expect
    .poll(titles)
    .toEqual([
      "Never Gonna Give You Up",
      "Rick Astley - Never Gonna Give You Up (Official Video) (4K Remaster)",
      "Together Forever",
      "Never Gonna Give You Up",
      "Whenever You Need Somebody",
    ]);
});

test("T7: Espaço pausa/retoma a fila e Ctrl+K abre o overlay", async ({ page }) => {
  await open(page, "/activity", "busy");
  await page.getByRole("heading", { level: 1 }).click();
  await page.keyboard.press("Space");
  await expect(page.getByRole("button", { name: "Retomar fila" })).toBeVisible();
  await expect(page.getByTestId("queue-paused")).toBeVisible();
  await page.keyboard.press("Space");
  await expect(page.getByRole("button", { name: "Pausar fila" })).toBeVisible();

  await page.keyboard.press("Control+k");
  await expect(page.getByTestId("command-bar-overlay")).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(page.getByTestId("command-bar-overlay")).toHaveCount(0);
});

test("T8: busca ⇒ resultado ⇒ Baixar direto cria o job", async ({ page }) => {
  await open(page, "/", "empty");
  await bar(page).fill("rick astley never gonna give you up");
  await bar(page).press("Enter");

  const results = page.getByTestId("search-result");
  await expect(results).toHaveCount(3);
  await expect(page.getByRole("tab", { name: "YouTube Music", selected: true })).toBeVisible();
  await expect(page.getByTestId("badge-activity")).toHaveCount(0);

  await results
    .first()
    .getByRole("button", { name: /^Baixar / })
    .click();
  await expect(page.getByText("Adicionado à fila")).toBeVisible();
  await expect(page.getByTestId("badge-activity")).toHaveText("1");

  await goHash(page, "/activity");
  await expect(page.getByTestId("job-card")).toHaveCount(1);
  await expect(page.getByTestId("job-card")).toContainText("Never Gonna Give You Up");
});

test("Preview: perfil que recodifica mostra aviso; duplicata pede confirmação", async ({
  page,
}) => {
  await open(page, "/", "empty");
  await bar(page).fill(VIDEO_URL);
  await bar(page).press("Enter");
  const preview = page.getByRole("dialog", { name: "Pré-visualização" });
  await preview.getByRole("button", { name: "MP3 320 kbps" }).click();
  await expect(preview.getByRole("note")).toContainText("Recodifica o áudio");
  await preview.getByRole("button", { name: /Original/ }).click();
  await preview.getByRole("button", { name: "Adicionar à fila" }).click();
  await expect(preview).toHaveCount(0);

  await bar(page).fill(VIDEO_URL);
  await bar(page).press("Enter");
  await expect(preview.getByText("Já baixada")).toBeVisible();
  await preview.getByRole("button", { name: "Adicionar à fila" }).click();
  const confirm = page.getByRole("dialog", { name: "Baixar novamente?" });
  await expect(confirm).toBeVisible();
  await confirm.getByRole("button", { name: "Baixar novamente" }).click();
  await expect(page.getByTestId("badge-activity")).toHaveText("2");
});
