import { invoke, submitLink, waitForJob } from "../helpers.mjs";
describe("F15 importação Deezer no app real", () => {
  it("imports the chart playlist, displays matches and downloads one recording with source ISRC", async () => {
    const chart = await (await fetch("https://api.deezer.com/chart/0/playlists?limit=1")).json();
    const id = chart.data[0].id;
    const previous = await invoke("settings_get");
    try {
      await invoke("settings_update", { patch: { fetchLyrics: false, playlistPacingSeconds: 0 } });
      await submitLink(`https://deezer.com/playlist/${id}`);
      await browser.waitUntil(async () => (await $("h1").getText()) === chart.data[0].title, {
        // The complete chart is matched through the shared three-request limit.
        timeout: 600000,
        timeoutMsg: "Coleção Deezer não abriu",
      });
      const candidates = await $$('input[type="checkbox"]:checked');
      expect(candidates.length).toBeGreaterThan(1);
      const title = await candidates[1].getAttribute("aria-label");
      await $('//button[normalize-space()="Limpar seleção"]').click();
      await $(`input[aria-label=${JSON.stringify(title)}]`).click();
      const button = await $('//button[starts-with(normalize-space(),"Baixar selecionadas")]');
      await button.click();
      await waitForJob((j) => j.status === "done" && j.metadataOverride?.isrc, {
        timeout: 240000,
        message: "Faixa importada não concluiu",
      });
      const jobs = await invoke("jobs_list");
      const job = jobs.find((j) => j.status === "done" && j.metadataOverride?.isrc);
      expect(job).toBeDefined();
      const library = await invoke("library_list", { query: { text: job.title } });
      const item = library.items.find((i) => i.id === job.libraryId);
      expect(item.isrc).toBe(job.metadataOverride.isrc);
      expect(title).toContain(job.title);
    } finally {
      await invoke("settings_update", {
        patch: {
          fetchLyrics: previous.fetchLyrics,
          playlistPacingSeconds: previous.playlistPacingSeconds,
        },
      });
    }
  }).timeout(900000);
});
