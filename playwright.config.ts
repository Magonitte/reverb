import { defineConfig } from "@playwright/test";

const port = Number(process.env.REVERB_E2E_PORT ?? 1421);
const url = `http://localhost:${port}`;

export default defineConfig({
  testDir: "tests/e2e",
  fullyParallel: true,
  globalSetup: "./tests/e2e/global-setup.ts",
  reporter: "list",
  use: { baseURL: url },
  webServer: {
    command: `npm run dev:mock -- --port ${port} --strictPort`,
    url,
    reuseExistingServer: false,
  },
  projects: [{ name: "desktop-1366", use: { viewport: { width: 1366, height: 768 } } }],
});
