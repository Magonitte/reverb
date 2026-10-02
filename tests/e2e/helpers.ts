import { expect, type Page } from "@playwright/test";

/** Rotas do design §3 com o título (h1) esperado em pt-BR. */
export const ROUTES: Array<{ path: string; title: string; nav?: string }> = [
  { path: "/", title: "Início", nav: "home" },
  { path: "/library", title: "Biblioteca", nav: "library" },
  { path: "/playlists", title: "Playlists", nav: "playlists" },
  { path: "/activity", title: "Atividade", nav: "activity" },
  { path: "/review", title: "Revisar", nav: "review" },
  { path: "/settings", title: "Configurações", nav: "settings" },
  { path: "/tag-editor", title: "Editor de tags" },
  { path: "/onboarding", title: "Bem-vindo ao Reverb" },
];

/** Acumula erros de console e exceções da página. */
export function collectErrors(page: Page): string[] {
  const errors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") errors.push(msg.text());
  });
  page.on("pageerror", (err) => errors.push(err.message));
  return errors;
}

export type Scenario =
  "empty" | "busy" | "errors" | "heal" | "update-available" | "update-downloading" | "update-error";

/** Abre a rota (HashRouter) com um cenário do backend falso e espera o título. */
export async function open(page: Page, path = "/", scenario: Scenario = "empty") {
  await page.goto(`/?scenario=${scenario}#${path}`);
  await waitForRoute(page, path);
  await settle(page);
}

/** Espera a tela da rota (não a anterior) estar montada, com o chunk lazy já carregado. */
export async function waitForRoute(page: Page, path: string) {
  const key = `/${path.split("/")[1] ?? ""}`;
  await expect(page.locator(`[data-testid=screen][data-route="${key}"]`)).toBeVisible();
  await expect(page.getByTestId("screen-loading")).toHaveCount(0);
  await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
}

/** Navega sem recarregar (mantém o estado das stores, como o tema escolhido). */
export async function goHash(page: Page, path: string) {
  await page.evaluate((p) => {
    window.location.hash = p;
  }, path);
  await waitForRoute(page, path);
  await settle(page);
}

/**
 * Espera a animação de entrada da tela (fade + 8 px, 240 ms) terminar de verdade: o motion
 * remove o `transform` inline no fim. Sem isso o texto ainda está numa camada composta
 * (antialiasing em tons de cinza) e a screenshot muda de pixel conforme a carga da máquina.
 */
export async function settle(page: Page) {
  await page.waitForFunction(() => {
    const el = document.querySelector<HTMLElement>("[data-testid=screen]");
    if (!el) return true;
    const transform = el.style.transform;
    return (
      (transform === "" || transform === "none") &&
      getComputedStyle(el).opacity === "1" &&
      el.getAnimations().length === 0
    );
  });
  await page.evaluate(
    () =>
      new Promise<void>((done) => requestAnimationFrame(() => requestAnimationFrame(() => done()))),
  );
}

/** Muda o tema pela tela de Configurações (caminho real: store → useAppearance). */
export async function setTheme(page: Page, theme: "dark" | "light") {
  await goHash(page, "/settings");
  await page.getByLabel("Tema").selectOption(theme);
  await expect(page.locator("html")).toHaveAttribute("data-theme", theme);
}

// PNG 1×1 (marrom escuro): as miniaturas reais vêm da rede e não podem mudar o pixel das bases.
const THUMBNAIL_PNG = Buffer.from(
  "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAIAAACQd1PeAAAADElEQVR4nGPwsjcFAAGVAL+msWapAAAAAElFTkSuQmCC",
  "base64",
);

/** Troca as miniaturas do YouTube por uma imagem fixa (testes visuais determinísticos e offline). */
export async function stubThumbnails(page: Page) {
  await page.route(/\/\/i\.ytimg\.com\//, (route) =>
    route.fulfill({ status: 200, contentType: "image/png", body: THUMBNAIL_PNG }),
  );
}
