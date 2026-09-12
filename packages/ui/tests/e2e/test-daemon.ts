// Starts the real product behind the E2E run. See playwright.config.ts.
import { createHash } from "node:crypto";
import { spawn } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, realpathSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const uiRoot = resolve(here, "../..");
const repoRoot = resolve(uiRoot, "../..");
const distDir = join(uiRoot, "tests/.e2e-dist");
const HEADERS = { "X-Ctxpect-Client": "desktop", "Content-Type": "application/json" };

const PASS_LAYERS =
  '[{"layer":"organization","mode":"enforceable","rules":[]},{"layer":"team","mode":"enforceable","rules":[]},{"layer":"project","mode":"enforceable","rules":[]},{"layer":"user","mode":"detect-only","rules":[]},{"layer":"session","mode":"detect-only","rules":[]}]';

function sha256(text: string): string {
  return createHash("sha256").update(text, "utf8").digest("hex");
}

/** The same scope digest `ctxpect_cli::project_scope_digest` computes. */
function projectDigest(project: string): string {
  return sha256(`project:${realpathSync(project)}`);
}

function plantGrants(store: string, project: string, actions: string[]): void {
  mkdirSync(join(store, "policies"), { recursive: true });
  mkdirSync(join(store, "exceptions"), { recursive: true });
  writeFileSync(join(store, "policies/active.json"), PASS_LAYERS);
  const digest = projectDigest(project);
  for (const action of actions) {
    const id = `ex-e2e-${action.replace(/\./g, "-")}`;
    writeFileSync(
      join(store, `exceptions/${id}.json`),
      JSON.stringify({
        exception_id: id,
        requester: "operator",
        action,
        project_digest: digest,
        target: "*",
        state: "approved",
        created_at: 1,
        expires_at: 4102444800,
        reason: "e2e grant",
        approver: "enrolled-out-of-band",
      }),
    );
  }
}

async function post(base: string, path: string, body: unknown): Promise<unknown> {
  const res = await fetch(`${base}${path}`, { method: "POST", headers: HEADERS, body: JSON.stringify(body) });
  const json = await res.json();
  if (!res.ok) throw new Error(`${path}: ${res.status} ${JSON.stringify(json)}`);
  return json;
}

function runsDocument(pairs: number): unknown {
  const runs = [];
  for (let index = 0; index < pairs; index += 1) {
    for (const [arm, outcome] of [["control", "fail"], ["treatment", "pass"]]) {
      runs.push({
        run_id: `${arm}-${index}`, arm, task_id: `t${index}`, outcome,
        code_digest: "c", model: "m", harness: "h", tool_availability_digest: "t",
        started_at: "101.0Z", ended_at: "102.0Z",
      });
    }
  }
  return {
    schema: "ctxpect-effect-runs-v1",
    contract: {
      schema: "experiment-contract-v1", experiment_id: "e-lab", primary_outcome: "task-pass", margin_pp: 10,
      pairing: "paired-by-task", n_planned: pairs, alpha: "0.05", power: "0.80", multiplicity: "none",
      itt: "count-as-fail", locked: { code_digest: "c", model: "m", harness: "h", tool_availability_digest: "t" },
      frozen_at: "100.0Z", invalidation: ["harness update"],
    },
    runs,
  };
}

export async function startTestDaemon(extraGrants: string[] = [], periodic = false) {
  const bin = process.env.CTXPECT_BIN ?? join(repoRoot, "target/debug/ctxpect");
  const scratch = mkdtempSync(join(tmpdir(), "cx-e2e-"));
  const project = join(scratch, "project");
  const store = join(scratch, "store");
  mkdirSync(project, { recursive: true });
  writeFileSync(join(project, "AGENTS.md"), "hello from e2e\n");
  plantGrants(store, project, ["sessions.import", "sessions.delete", "experiment.persist", "assets.copy", ...extraGrants]);
  // A registered asset for the /assets write loop (DEMO-06/07/12). The
  // registry digest pins the seeded bytes; `assets.rollback` stays
  // un-granted on purpose so an unauthorized write has something to refuse.
  mkdirSync(join(project, "vendor"), { recursive: true });
  mkdirSync(join(project, ".ctxpect"), { recursive: true });
  const assetBytes = "synthetic e2e skill body (not a real skill)\n";
  writeFileSync(join(project, "vendor/skill-e2e.md"), assetBytes);
  writeFileSync(
    join(project, ".ctxpect/assets.json"),
    JSON.stringify({
      schema: "ctxpect-assets-v1",
      assets: [
        {
          asset_id: "skill-e2e",
          origin: "project:vendor/skill-e2e.md",
          license: "MIT",
          digest: sha256(assetBytes),
          target_rel: ".ctxpect/skills/skill-e2e.md",
        },
      ],
    }),
  );

  const daemon = spawn(bin, ["daemon", "start", ...(periodic ? ["--execute", "--poll-ms", "100"] : []), "--project", project, "--store", store, "--ui-root", distDir, "--listen", "127.0.0.1:0"], {
    stdio: ["ignore", "ignore", "pipe"],
  });
  let stderr = "";
  let spawnError: Error | undefined;
  daemon.on("error", (error) => { spawnError = error; });
  daemon.stderr?.on("data", (chunk) => { stderr += String(chunk); });
  async function stop() {
    if (daemon.exitCode === null && daemon.signalCode === null && daemon.pid) {
      await new Promise<void>((resolve) => {
        const timer = setTimeout(() => { daemon.kill("SIGKILL"); }, 2000);
        daemon.once("exit", () => { clearTimeout(timer); resolve(); });
        daemon.kill("SIGTERM");
      });
    }
    rmSync(scratch, { recursive: true, force: true });
  }
  try {
  const addrFile = join(store, "daemon.addr");
  let base = "";
  for (let attempt = 0; attempt < 100 && !base; attempt += 1) {
    if (spawnError) throw spawnError;
    if (daemon.exitCode !== null) throw new Error(`daemon exited: ${stderr}`);
    if (existsSync(addrFile)) {
      const text = readFileSync(addrFile, "utf8").trim();
      if (text) base = `http://${text}`;
    }
    if (!base) await new Promise((r) => setTimeout(r, 50));
  }
  if (!base) throw new Error(`daemon did not write daemon.addr: ${stderr}`);
  let healthy = false;
  for (let attempt = 0; attempt < 100; attempt += 1) {
    try {
      const res = await fetch(`${base}/api/v1/health`);
      if (res.ok) { healthy = true; break; }
    } catch {
      /* not up yet */
    }
    await new Promise((r) => setTimeout(r, 50));
  }

  if (!healthy) throw new Error("daemon health check timed out");
  // Seed synthetic native-format sessions and synthetic experiment results through the API.
  const jsonl = readFileSync(join(repoRoot, "acceptance/corpus/development/native/deepseek-harness-cli-0.1.2-rc.1/session.jsonl"), "utf8");
  await post(base, "/api/v1/sessions/import", { session_id: "s-alpha", mapping_id: "deepseek-harness-cli", jsonl });
  await post(base, "/api/v1/sessions/import", { session_id: "s-beta", mapping_id: "deepseek-harness-cli", jsonl });
  await post(base, "/api/v1/lab", { experiment_id: "e-lab", runs: runsDocument(4) });

  return { base, project, store, scratch, daemon, stop };
  } catch (error) {
    await stop();
    throw error;
  }
}

export type TestDaemon = Awaited<ReturnType<typeof startTestDaemon>>;
