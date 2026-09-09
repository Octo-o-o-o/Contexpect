// Starts the real product behind the E2E run. See playwright.config.ts.
import { createHash } from "node:crypto";
import { execFileSync, spawn } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, realpathSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const uiRoot = resolve(here, "../..");
const repoRoot = resolve(uiRoot, "../..");
const outDir = join(uiRoot, "tests/.e2e-out");
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

export default async function globalSetup(): Promise<void> {
  rmSync(outDir, { recursive: true, force: true });
  mkdirSync(outDir, { recursive: true });

  const bin = process.env.CTXPECT_BIN ?? join(repoRoot, "target/debug/ctxpect");
  if (!existsSync(bin)) {
    execFileSync("cargo", ["build", "-p", "ctxpect-cli"], { cwd: repoRoot, stdio: "inherit", env: { ...process.env, CARGO_NET_OFFLINE: "true" } });
  }
  execFileSync("pnpm", ["exec", "vite", "build", "--outDir", "tests/.e2e-dist", "--logLevel", "error"], { cwd: uiRoot, stdio: "inherit" });

  const scratch = mkdtempSync(join(tmpdir(), "cx-e2e-"));
  const project = join(scratch, "project");
  const store = join(scratch, "store");
  mkdirSync(project, { recursive: true });
  writeFileSync(join(project, "AGENTS.md"), "hello from e2e\n");
  plantGrants(store, project, ["sessions.import", "sessions.delete", "experiment.persist"]);

  const daemon = spawn(bin, ["daemon", "start", "--project", project, "--store", store, "--ui-root", distDir, "--listen", "127.0.0.1:0"], {
    stdio: ["ignore", "ignore", "pipe"],
  });
  let stderr = "";
  daemon.stderr?.on("data", (chunk) => { stderr += String(chunk); });
  // Recorded before anything can fail, so teardown can always find the
  // daemon: a seed that throws must not leave a detached process behind.
  writeFileSync(join(outDir, "state.json"), JSON.stringify({ base: "", pid: daemon.pid, scratch, project }, null, 2));
  const addrFile = join(store, "daemon.addr");
  let base = "";
  for (let attempt = 0; attempt < 100 && !base; attempt += 1) {
    if (daemon.exitCode !== null) throw new Error(`daemon exited: ${stderr}`);
    if (existsSync(addrFile)) {
      const text = readFileSync(addrFile, "utf8").trim();
      if (text) base = `http://${text}`;
    }
    if (!base) await new Promise((r) => setTimeout(r, 50));
  }
  if (!base) throw new Error(`daemon did not write daemon.addr: ${stderr}`);
  for (let attempt = 0; attempt < 100; attempt += 1) {
    try {
      const res = await fetch(`${base}/api/v1/health`);
      if (res.ok) break;
    } catch {
      /* not up yet */
    }
    await new Promise((r) => setTimeout(r, 50));
  }

  // Seed through the API only: two native sessions and one executed experiment.
  const jsonl = readFileSync(join(repoRoot, "acceptance/corpus/development/native/deepseek-harness-cli-0.1.2-rc.1/session.jsonl"), "utf8");
  await post(base, "/api/v1/sessions/import", { session_id: "s-alpha", mapping_id: "deepseek-harness-cli", jsonl });
  await post(base, "/api/v1/sessions/import", { session_id: "s-beta", mapping_id: "deepseek-harness-cli", jsonl });
  await post(base, "/api/v1/lab", { experiment_id: "e-lab", runs: runsDocument(4) });

  writeFileSync(join(outDir, "state.json"), JSON.stringify({ base, pid: daemon.pid, scratch, project }, null, 2));
  process.env.E2E_BASE_URL = base;
  process.env.E2E_PROJECT = project;
  daemon.unref();
}
