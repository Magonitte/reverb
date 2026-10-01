import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: "tests/e2e",
  fullyParallel: true,
  globalSetup: "./tests/e2e/global-setup.ts",
  reporter: "list",
  use: { baseURL: "http://localhost:1421" },
  webServer: {
    command: "npm run dev:mock",
    url: "http://localhost:1421",
    reuseExistingServer: !process.env.CI,
  },
  projects: [{ name: "desktop-1366", use: { viewport: { width: 1366, height: 768 } } }],
});
