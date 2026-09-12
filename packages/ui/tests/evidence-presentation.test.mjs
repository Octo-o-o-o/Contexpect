import assert from "node:assert/strict";
import test from "node:test";
import { displayTime, findingScope } from "../src/evidence-presentation.js";
import { classifyPayload } from "../src/page-state.js";

test("an unsupported sibling never invalidates a selected supported catalog coordinate", () => {
  const families = [
    { family_id: "codex", active_coordinate: true, reason_code: "static-resolver-available" },
    { family_id: "other", active_coordinate: false, reason_code: "unsupported_harness_version" },
    { family_id: "connector", active_coordinate: false, reason_code: "connector_required" },
  ];
  assert.equal(classifyPayload({ families }).state, "ok");
  assert.equal(classifyPayload({ catalog: { families }, copy_executor: "project-scoped" }).state, "ok");
  families[0].reason_code = "unsupported_harness_version";
  assert.equal(classifyPayload({ families }).state, "unsupported-version");
  assert.equal(classifyPayload(families[1]).state, "unsupported-version");
});

test("a project audit path is linked only by its observed project layer, not a filename or test label", () => {
  const receipt = { layers: [
    { rel: "<codex-home>/", adopted: "AGENTS.md" },
    { rel: "<project>/sub/", adopted: "sub/AGENTS.md" },
  ] };
  const finding = { facet: "project-content", rule_id: "hidden_unicode", path: "AGENTS.md" };
  assert.equal(findingScope(finding, receipt), "project");
  assert.equal(findingScope({ ...finding, path: "sub/AGENTS.md" }, receipt), "linked-file");
  assert.equal(findingScope({ ...finding, path: "sub/AGENTS.md", rule_id: "conflict" }, receipt), "project");
  assert.equal(findingScope({ ...finding, path: "tests/AGENTS.md" }, receipt), "project");
  assert.equal(findingScope({ facet: "model-visible" }, receipt), "receipt");
});

test("store clock and RFC3339 display the same instant without rewriting raw evidence", () => {
  assert.equal(displayTime("1789227148.805Z", "zh-CN"), displayTime("2026-09-12T15:32:28.805Z", "zh-CN"));
  assert.equal(displayTime("not-a-time", "en"), "not-a-time");
  assert.equal(displayTime(null, "en"), "—");
  assert.ok(!displayTime("1789227148.805Z", "zh-CN").includes("1789227148"));
});
