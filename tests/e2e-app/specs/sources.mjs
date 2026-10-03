import { execFileSync } from "node:child_process";
import { mkdirSync, readFileSync } from "node:fs";
import { join, dirname } from "node:path";
import { invoke, waitForHome } from "../helpers.mjs";

describe("F14 lossless no app real", () => {
  it("verifies imported FLAC automatically, persists the verdict, and renders the spectrum", async () => {
    await waitForHome();
    const tools = process.env.REVERB_TEST_TOOLS_DIR;
    const manifest = JSON.parse(readFileSync(join(tools, "manifest.json"), "utf8"));
    const ffmpeg = join(tools, "ffmpeg", manifest.tools.ffmpeg.current.version, "ffmpeg.exe");
    const root = join(dirname(process.env.REVERB_E2E_OUTPUT_DIR), "f14-fixture");
    mkdirSync(root, { recursive: true });
    const path = join(root, "F14 white noise.flac");
    execFileSync(
      ffmpeg,
      [
        "-nostdin",
        "-y",
        "-f",
        "lavfi",
        "-i",
        "anoisesrc=color=white:sample_rate=44100:duration=4:seed=17",
        "-c:a",
        "flac",
        path,
      ],
      { stdio: "pipe", windowsHide: true },
    );
    const wav = join(root, "F14 float WAV.wav");
    execFileSync(
      ffmpeg,
      [
        "-nostdin",
        "-y",
        "-f",
        "lavfi",
        "-i",
        "anoisesrc=sample_rate=22050:duration=1",
        "-c:a",
        "pcm_f32le",
        wav,
      ],
      { stdio: "pipe", windowsHide: true },
    );
    const previous = await invoke("settings_get");
    try {
      await invoke("settings_update", { patch: { verifyLosslessOnImport: true } });
      const imported = await invoke("library_import", { paths: [path, wav] });
      expect(imported.imported).toBe(2);
      expect(imported.failures).toHaveLength(0);
      const page = await invoke("library_list", { query: { text: "F14 white noise" } });
      expect(page.items).toHaveLength(1);
      expect(page.items[0].losslessVerdict).toBe("lossless");
      const wavPage = await invoke("library_list", { query: { text: "F14 float WAV" } });
      expect(wavPage.items).toHaveLength(1);
      expect(wavPage.items[0].codec).toBe("pcm_f32le");
      expect(wavPage.items[0].losslessVerdict).toBe("inconclusive");
      await $('[data-testid="nav-library"]').click();
      const search = await $('input[aria-label="Buscar na biblioteca"]');
      await search.waitForDisplayed();
      await search.setValue("F14 white noise");
      await $('button[aria-label="Detalhes e ações de F14 white noise"]').click();
      const button = await $('button[aria-label="Verificar lossless F14 white noise"]');
      await button.waitForClickable();
      await button.click();
      const spectrum = await $('img[alt="Espectrograma do áudio"]');
      await spectrum.waitForDisplayed();
      await browser.waitUntil(
        async () =>
          browser.execute(
            () => document.querySelector('img[alt="Espectrograma do áudio"]')?.naturalWidth === 800,
          ),
        { timeout: 10000 },
      );
      const dimensions = await browser.execute(() => {
        const image = document.querySelector('img[alt="Espectrograma do áudio"]');
        return [image.naturalWidth, image.naturalHeight];
      });
      expect(dimensions).toEqual([800, 300]);
      await $('//button[normalize-space()="Fechar"]').click();
    } finally {
      await invoke("settings_update", {
        patch: { verifyLosslessOnImport: previous.verifyLosslessOnImport },
      });
    }
  });
});
