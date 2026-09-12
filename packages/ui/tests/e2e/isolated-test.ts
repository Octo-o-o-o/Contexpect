import { test as base, expect } from "@playwright/test";
import { startTestDaemon, type TestDaemon } from "./test-daemon";

export const test = base.extend<{ daemon: TestDaemon; extraGrants: string[] }>({
  extraGrants: [[], { option: true }],
  daemon: async ({ extraGrants }, use) => {
    const runtime = await startTestDaemon(extraGrants);
    try { await use(runtime); } finally { await runtime.stop(); }
  },
});
export { expect };
