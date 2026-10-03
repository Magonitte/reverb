// Helpers dos specs do app real (WebdriverIO). `browser`, `$` e `expect` são globais do WDIO.

/** Chama um comando Tauri direto da página (o mesmo `invoke` que o frontend usa). */
export async function invoke(command, args = {}) {
  const result = await browser.executeAsync(
    (cmd, params, done) => {
      window.__TAURI_INTERNALS__.invoke(cmd, params).then(
        (value) => done({ ok: value }),
        (error) => done({ error }),
      );
    },
    command,
    args,
  );
  if (result.error !== undefined) {
    throw new Error(`${command} falhou: ${JSON.stringify(result.error)}`);
  }
  return result.ok;
}

/** Espera o app abrir no Início (a barra de comando é o primeiro elemento útil). */
export async function waitForHome() {
  const bar = await $('input[aria-label="Barra de comando"]');
  await bar.waitForDisplayed({ timeout: 60_000 });
  return bar;
}

/** Cola um link na barra de comando e envia. */
export async function submitLink(url) {
  const bar = await waitForHome();
  await bar.setValue(url);
  await browser.keys("Enter");
}

export const byText = (tag, text) => $(`//${tag}[normalize-space()="${text}"]`);

export async function openTab(label) {
  const tab = await $(`//button[@role="tab"][starts-with(normalize-space(), "${label}")]`);
  await tab.waitForClickable();
  await tab.click();
}

/**
 * Espera um job satisfazer `predicate`. Falha logo se ele terminar em erro, para o teste
 * mostrar a causa em vez de estourar o tempo.
 */
export async function waitForJob(predicate, { timeout, message }) {
  let last = null;
  await browser.waitUntil(
    async () => {
      const jobs = await invoke("jobs_list");
      last = jobs[0] ?? null;
      const failed = jobs.find((j) => j.status === "failed");
      if (failed) {
        throw new Error(`job falhou: ${failed.errorKind}: ${failed.errorMessage}`);
      }
      return jobs.find(predicate) !== undefined;
    },
    { timeout, interval: 1000, timeoutMsg: `${message} (último estado: ${JSON.stringify(last)})` },
  );
  return (await invoke("jobs_list")).find(predicate);
}
