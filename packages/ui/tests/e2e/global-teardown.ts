import { existsSync, readFileSync, rmSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const uiRoot = resolve(here, "../..");

export default async function globalTeardown(): Promise<void> {
  const stateFile = join(uiRoot, "tests/.e2e-out/state.json");
  if (!existsSync(stateFile)) return;
  const state = JSON.parse(readFileSync(stateFile, "utf8")) as { pid?: number; scratch?: string };
  if (state.pid) {
    try {
      process.kill(state.pid, "SIGTERM");
    } catch {
      /* already gone */
    }
  }
  if (state.scratch) rmSync(state.scratch, { recursive: true, force: true });
  rmSync(join(uiRoot, "tests/.e2e-dist"), { recursive: true, force: true });
}
