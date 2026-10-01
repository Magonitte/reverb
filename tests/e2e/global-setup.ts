import { chromium, type FullConfig } from "@playwright/test";

/** Aquece o servidor de desenvolvimento: compila cada rota lazy uma vez antes dos testes paralelos. */
export default async function globalSetup(config: FullConfig) {
  const baseURL = config.projects[0]?.use.baseURL ?? "http://localhost:1421";
  const browser = await chromium.launch();
  const page = await browser.newPage();
  for (const path of [
    "/",
    "/library",
    "/playlists",
    "/activity",
    "/review",
    "/settings",
    "/tag-editor",
    "/onboarding",
  ]) {
    await page.goto(`${baseURL}/#${path}`, { waitUntil: "networkidle" });
    await page.getByRole("heading", { level: 1 }).waitFor();
  }
  await browser.close();
}
