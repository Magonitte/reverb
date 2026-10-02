// T10 e T11 (F07): fluxo de download no app Tauri real, com rede (yt-dlp e FFmpeg de verdade).
import { existsSync, readdirSync, statSync } from "node:fs";
import { join } from "node:path";
import { byText, invoke, openTab, submitLink, waitForHome, waitForJob } from "../helpers.mjs";

const FX1_URL = "https://www.youtube.com/watch?v=jNQXAC9IVRw";
const FX3_URL = "https://www.youtube.com/watch?v=dQw4w9WgXcQ";
const outputDir = process.env.REVERB_E2E_OUTPUT_DIR;

/** Arquivos com a extensão dada, em qualquer subpasta. */
function filesWithExtension(dir, extension) {
  return readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
    const path = join(dir, entry.name);
    if (entry.isDirectory()) return filesWithExtension(path, extension);
    return path.endsWith(extension) ? [path] : [];
  });
}

describe("fluxo de download no app real", () => {
  // Falha clara se a janela não subir.
  before(async () => {
    await waitForHome();
  });

  it("T10: colar FX1 ⇒ Preview ⇒ Adicionar à fila ⇒ Concluído com o arquivo .opus", async () => {
    await submitLink(FX1_URL);

    const title = await $('[data-testid="preview-title"]');
    await title.waitForDisplayed({ timeout: 90_000 });
    await expect(title).toHaveText("Me at the zoo");
    // O codec real varia com o que o YouTube serve (AAC 130 kbps ou Opus); o selo precisa existir.
    await expect($('[data-testid="source-quality"]')).toHaveText(
      expect.stringMatching(/^Fonte: \S+ \d+ kbps$/),
    );

    const add = await byText("button", "Adicionar à fila");
    await add.click();

    await $('[data-testid="nav-activity"]').click();
    await openTab("Concluídos");

    const job = await waitForJob((j) => j.status === "done", {
      timeout: 120_000,
      message: "o download não concluiu em 120 s",
    });
    const card = await $('[data-testid="job-card"][data-status="done"]');
    await card.waitForDisplayed({ timeout: 10_000 });
    await expect(card).toHaveText(expect.stringContaining("Me at the zoo"));

    expect(job.outputPath).toMatch(/\.opus$/);
    expect(job.outputPath.startsWith(outputDir)).toBe(true);
    expect(existsSync(job.outputPath)).toBe(true);
    expect(statSync(job.outputPath).size).toBeGreaterThan(10_000);
    expect(filesWithExtension(outputDir, ".opus")).toContain(job.outputPath);
  });

  it("T11: cancelar um download em andamento (limite de velocidade baixo) ⇒ Cancelado", async () => {
    await invoke("settings_update", { patch: { speedLimitMbps: 0.1 } });
    try {
      // O app reabre no Início a cada sessão; a Atividade (T10) pode estar aberta ainda.
      await $('[data-testid="nav-home"]').click();
      await submitLink(FX3_URL);
      const title = await $('[data-testid="preview-title"]');
      await title.waitForDisplayed({ timeout: 90_000 });
      await (await byText("button", "Baixar agora")).click();

      await $('[data-testid="nav-activity"]').click();
      const running = await waitForJob(
        (j) => j.sourceId === "dQw4w9WgXcQ" && j.stage === "downloading",
        {
          timeout: 120_000,
          message: "o download não começou em 120 s",
        },
      );
      await browser.pause(2000); // deixa alguns bytes baixarem antes de cancelar
      expect(running.status).toBe("running");

      const cancel = await $('//button[starts-with(@aria-label, "Cancelar ")]');
      await cancel.waitForClickable();
      await cancel.click();

      await waitForJob((j) => j.sourceId === "dQw4w9WgXcQ" && j.status === "cancelled", {
        timeout: 60_000,
        message: "o job não ficou Cancelado em 60 s",
      });
      await openTab("Falhas");
      const card = await $('[data-testid="job-card"][data-status="cancelled"]');
      await card.waitForDisplayed({ timeout: 10_000 });
      await expect(card).toHaveText(expect.stringContaining("Cancelado"));
      expect(filesWithExtension(outputDir, ".opus")).toHaveLength(1); // só o do T10
    } finally {
      await invoke("settings_update", { patch: { speedLimitMbps: 0 } });
    }
  });
});
