// F10 T12: actual disk import and tag editing, with synthetic audio and isolated data.
import { execFileSync } from "node:child_process";
import { mkdirSync, readFileSync } from "node:fs";
import { join, dirname } from "node:path";
import { invoke, waitForHome } from "../helpers.mjs";

describe("F10 biblioteca e editor no app real", () => {
  it("imports two audio files, edits a title in the UI, and verifies it using the CLI", async () => {
    await waitForHome();
    const tools = process.env.REVERB_TEST_TOOLS_DIR;
    const manifest = JSON.parse(readFileSync(join(tools, "manifest.json"), "utf8"));
    const ffmpeg = join(tools, "ffmpeg", manifest.tools.ffmpeg.current.version, "ffmpeg.exe");
    const root = join(dirname(process.env.REVERB_E2E_OUTPUT_DIR), "import-fixture");
    mkdirSync(root, { recursive: true });
    mkdirSync(join(root, "nested"), { recursive: true });
    const paths = [join(root, "fixture-one.opus"), join(root, "nested", "fixture-two.flac")];
    for (const path of paths)
      execFileSync(
        ffmpeg,
        ["-nostdin", "-y", "-f", "lavfi", "-i", "sine=frequency=440:duration=0.5", path],
        { stdio: "pipe" },
      );
    const single = await invoke("library_import", { paths: [paths[0]] });
    expect(single.imported).toBe(1);
    expect(single.failures).toHaveLength(0);
    const result = await invoke("library_import", { paths: [root] });
    expect(result.imported).toBe(1);
    expect(result.skipped).toBe(1);
    expect(result.failures).toHaveLength(0);
    await $('[data-testid="nav-library"]').click();
    const search = await $('input[aria-label="Buscar na biblioteca"]');
    await search.waitForDisplayed();
    await search.setValue("fixture");
    await $('//span[text()="fixture-one"]').waitForDisplayed();
    await $('//span[text()="fixture-two"]').waitForDisplayed();
    await $('button[aria-label="Editar tags de fixture-one"]').click();
    const title = await $('//label[normalize-space()="Título"]/following-sibling::input');
    await title.waitForDisplayed();
    await title.setValue("Título editado F10");
    await $('//button[normalize-space()="Salvar"]').click();
    await browser.waitUntil(
      async () => (await invoke("tags_read", { path: paths[0] })).title === "Título editado F10",
      { timeout: 10000, interval: 100 },
    );
    const output = execFileSync(
      process.env.REVERB_E2E_CLI,
      ["--data-dir", process.env.REVERB_DATA_DIR, "tags", "read", paths[0]],
      { encoding: "utf8" },
    );
    expect(JSON.parse(output).title).toBe("Título editado F10");
    await $('[data-testid="nav-library"]').click();
    await search.waitForDisplayed();
    await search.setValue("editado F10");
    await $('//span[text()="Título editado F10"]').waitForDisplayed();
  });

  it("searches real catalogs and opens metadata details with artwork in the UI", async () => {
    await $('[data-testid="nav-home"]').click();
    await waitForHome();
    const root = join(dirname(process.env.REVERB_E2E_OUTPUT_DIR), "import-fixture");
    const path = join(root, "fixture-one.opus");
    await $('[data-testid="nav-library"]').click();
    await $('input[aria-label="Buscar na biblioteca"]').setValue("editado F10");
    await $('button[aria-label="Editar tags de Título editado F10"]').click();
    const title = await $('//label[normalize-space()="Título"]/following-sibling::input');
    await title.waitForDisplayed();
    await title.setValue("Never Gonna Give You Up");
    await $('//label[normalize-space()="Artista"]/following-sibling::input').setValue(
      "Rick Astley",
    );
    await $('//button[normalize-space()="Buscar metadados"]').click();
    const view = await $('//button[normalize-space()="Ver detalhes"]');
    await view.waitForDisplayed({ timeout: 60000 });
    await view.click();
    const dialog = await $('[role="dialog"]');
    await dialog.waitForDisplayed();
    expect(await dialog.getText()).toContain("Rick Astley");
    expect(await dialog.getText()).toContain("Fonte");
    const image = await dialog.$('img[alt="Capa do álbum"]');
    await image.waitForDisplayed();
    await browser.waitUntil(
      async () => browser.execute((img) => img.complete && img.naturalWidth > 0, await image),
      { timeout: 30000 },
    );
    mkdirSync(join(process.cwd(), "test-results"), { recursive: true });
    await browser.saveScreenshot(join(process.cwd(), "test-results", "native-library-details.png"));
    await dialog.$('.//button[normalize-space()="Usar metadados"]').click();
    await $('//button[normalize-space()="Salvar"]').click();
    await browser.waitUntil(
      async () => (await invoke("tags_read", { path })).artist?.includes("Rick Astley"),
      { timeout: 10000 },
    );
    expect((await invoke("tags_read", { path })).cover).not.toBeNull();
  });
});
