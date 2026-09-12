// Build once; each mutating scenario can start its own real daemon/store.
import { execFileSync } from "node:child_process";
import { existsSync, mkdirSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { startTestDaemon } from "./test-daemon";
const here = dirname(fileURLToPath(import.meta.url));
const uiRoot = resolve(here, "../..");
const repoRoot = resolve(uiRoot, "../..");
const outDir = join(uiRoot, "tests/.e2e-out");

export default async function globalSetup(): Promise<void> {
  rmSync(outDir, { recursive: true, force: true });
  mkdirSync(outDir, { recursive: true });
  const bin = process.env.CTXPECT_BIN ?? join(repoRoot, "target/debug/ctxpect");
  if (!existsSync(bin)) {
    execFileSync("cargo", ["build", "-p", "ctxpect-cli"], { cwd: repoRoot, stdio: "inherit", env: { ...process.env, CARGO_NET_OFFLINE: "true" } });
  }
  execFileSync("pnpm", ["exec", "vite", "build", "--outDir", "tests/.e2e-dist", "--logLevel", "error"], { cwd: uiRoot, stdio: "inherit" });
  const runtime = await startTestDaemon();
  try {
    writeFileSync(join(outDir, "state.json"), JSON.stringify({ base: runtime.base, pid: runtime.daemon.pid, scratch: runtime.scratch, project: runtime.project }, null, 2));
    process.env.E2E_BASE_URL = runtime.base;
    process.env.E2E_PROJECT = runtime.project;
    process.env.E2E_STORE = runtime.store;
    runtime.daemon.unref();
  } catch (error) {
    await runtime.stop();
    throw error;
  }
}
