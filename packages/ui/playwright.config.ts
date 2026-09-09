// Browser interaction E2E (optional gate `ui-e2e`, ADR 0006).
//
// `tests/e2e/global-setup.ts` builds the UI into `tests/.e2e-dist`, starts a
// real `ctxpect daemon` on an ephemeral loopback port over a scratch project
// and store, seeds it through the API, and exports `E2E_BASE_URL`. No mock:
// every page under test talks to the daemon it would talk to in production.
import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: "tests/e2e",
  testMatch: /.*\.spec\.ts/,
  globalSetup: "./tests/e2e/global-setup.ts",
  globalTeardown: "./tests/e2e/global-teardown.ts",
  outputDir: "tests/.e2e-out/results",
  fullyParallel: false,
  workers: 1,
  retries: 0,
  reporter: [["list"]],
  timeout: 30_000,
  use: {
    headless: true,
    trace: "retain-on-failure",
  },
  projects: [{ name: "chromium", use: { browserName: "chromium" } }],
});
