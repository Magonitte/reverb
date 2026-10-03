import { byText, invoke, waitForHome } from "../helpers.mjs";

const select = (label) => $(`//label[normalize-space()="${label}"]/..//select`);
const checkbox = (label) => $(`//label[normalize-space()="${label}"]//input`);
describe("artist preferences in the real app", () => {
  it("searches, exercises every option, saves, edits and displays the discography", async () => {
    await waitForHome();
    await $('[data-testid="nav-library"]').click();
    await (await byText("button", "Artistas seguidos")).click();
    await (await byText("button", "Seguir artista")).click();
    const search = await $('//label[normalize-space()="Buscar artista"]/following-sibling::input');
    await search.setValue("Rick Astley");
    await browser.keys("Enter");
    const artist = await $('//button[contains(normalize-space(),"Deezer #6160")]');
    await artist.waitForDisplayed({ timeout: 60000 });
    await artist.click();
    await expect(await byText("p", "Artista selecionado: Rick Astley")).toBeDisplayed();
    for (const value of ["all", "latest", "none"]) {
      const input = await select("O que baixar agora");
      await input.selectByAttribute("value", value);
      await expect(input).toHaveValue(value);
    }
    for (const value of ["all", "notify", "none"]) {
      const input = await select("Lançamentos novos");
      await input.selectByAttribute("value", value);
      await expect(input).toHaveValue(value);
    }
    for (const value of ["original", "mp3_v0", "mp3_320", "aac_256", "opus_96", "flac"]) {
      const input = await select("Formato de saída");
      await input.selectByAttribute("value", value);
      await expect(input).toHaveValue(value);
    }
    for (const label of [
      "Álbuns",
      "EPs",
      "Singles",
      "Excluir ao vivo, remix e edições duplicadas",
    ]) {
      const input = await checkbox(label);
      await input.scrollIntoView();
      await expect(await byText("label", label)).toBeDisplayed();
      await input.click();
      await expect(input).not.toBeSelected();
    }
    await expect(await byText("button", "Salvar")).toBeDisabled();
    await (await checkbox("Álbuns")).click();
    const folder = await $(
      '//label[normalize-space()="Pasta de destino"]/following-sibling::input',
    );
    await folder.setValue(process.env.REVERB_E2E_OUTPUT_DIR);
    await (await byText("button", "Salvar")).click();
    await browser.waitUntil(async () => !(await $('[role="dialog"]').isExisting()), {
      timeout: 10000,
    });
    await browser.waitUntil(
      async () =>
        (await invoke("artists_followed")).some(
          (a) => a.providerArtistId === "6160" && a.lastCheckAt !== null,
        ),
      { timeout: 180000, timeoutMsg: "Discografia não foi verificada" },
    );
    const followed = (await invoke("artists_followed")).find((a) => a.providerArtistId === "6160");
    expect(followed.options).toEqual({
      monitorExisting: "none",
      monitorNew: "none",
      types: ["album"],
      excludeVariants: false,
      profileId: "flac",
      outputDir: process.env.REVERB_E2E_OUTPUT_DIR,
    });
    await (await byText("button", "Ver lançamentos")).click();
    await browser.waitUntil(
      async () => (await $$('//p[normalize-space()="Não monitorado"]')).length > 0,
      { timeout: 10000 },
    );
    await (await byText("button", "Editar acompanhamento")).click();
    await expect(await select("Formato de saída")).toHaveValue("flac");
    await (await select("Formato de saída")).selectByAttribute("value", "mp3_v0");
    await (await byText("button", "Salvar")).click();
    await browser.waitUntil(
      async () =>
        (await invoke("artists_followed")).find((a) => a.id === followed.id)?.options.profileId ===
        "mp3_v0",
      { timeout: 10000 },
    );
    await (await byText("button", "Faltando")).click();
    await expect(
      await byText("h2", "Nenhuma faixa faltante nos lançamentos monitorados"),
    ).toBeDisplayed();
  }).timeout(300000);
});
