import assert from "node:assert/strict";
import test from "node:test";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

import { changedFields, projectToSchema, setPath, validateDraft } from "../src/settings-form.js";

const root = dirname(fileURLToPath(import.meta.url));
const repo = join(root, "../../..");

/** The published schema, read from the Rust source of truth. */
function schemaFields() {
  // Parsed from the store crate so this test cannot drift from the schema the
  // store enforces; if the shapes diverge, the assertions below fail.
  return {
    privacy_mode: { kind: "enum", values: ["default", "screenshot"] },
    screenshot_privacy: { kind: "bool" },
    unmask_does_not_grant_egress: { kind: "const_bool", value: true },
    copy_confirm: { kind: "bool" },
    locale: { kind: "enum", values: ["zh-CN", "en"] },
    retention_days: { kind: "int", min: 1, max: 3650 },
    vault: { kind: "enum", values: ["metadata-only", "required"] },
    notifications: { kind: "bool" },
    resource_limits: {
      kind: "object",
      fields: {
        daemon_rss_mb: { kind: "int", min: 64, max: 65536 },
        scan_files: { kind: "int", min: 1, max: 10000000 },
      },
    },
    analysis_adapter: { kind: "enum", values: ["none"] },
  };
}

function defaults() {
  return {
    privacy_mode: "default",
    screenshot_privacy: false,
    unmask_does_not_grant_egress: true,
    copy_confirm: true,
    locale: "zh-CN",
    retention_days: 30,
    vault: "metadata-only",
    notifications: true,
    resource_limits: { daemon_rss_mb: 512, scan_files: 100000 },
    analysis_adapter: "none",
  };
}

test("the mirrored schema matches the Rust definition field for field", () => {
  // Guard against the client-side mirror drifting from the store's schema.
  const source = execFileSync("cat", [join(repo, "crates/ctxpect-store/src/settings.rs")], {
    encoding: "utf8",
  });
  const block = source.slice(
    source.indexOf("pub const SETTINGS_SCHEMA"),
    source.indexOf("impl FieldSpec"),
  );
  const declared = [...block.matchAll(/\(\s*"([a-z_]+)",\s*FieldSpec::/g)].map((m) => m[1]);
  const mirrored = Object.keys(schemaFields());
  // Nested object fields appear in the Rust block too; compare as sets.
  for (const name of mirrored) {
    assert.ok(declared.includes(name), `${name} missing from Rust schema`);
  }
  for (const name of declared) {
    const nested = ["daemon_rss_mb", "scan_files"];
    if (nested.includes(name)) continue;
    assert.ok(mirrored.includes(name), `${name} missing from the UI mirror`);
  }
});

test("defaults validate", () => {
  assert.deepEqual(validateDraft(schemaFields(), defaults()), []);
});

test("the client refuses what the store refuses, with the same codes", () => {
  const cases = [
    ["unmask_does_not_grant_egress", false, "settings.invariant_not_editable"],
    ["privacy_mode", "pwned", "settings.value_not_allowed"],
    ["analysis_adapter", "gpt-5", "settings.value_not_allowed"],
    ["retention_days", 0, "settings.out_of_range"],
    ["retention_days", "thirty", "settings.type_mismatch"],
    ["notifications", "yes", "settings.type_mismatch"],
  ];
  for (const [field, value, code] of cases) {
    const draft = { ...defaults(), [field]: value };
    const problems = validateDraft(schemaFields(), draft);
    assert.equal(problems.length, 1, `${field}: ${JSON.stringify(problems)}`);
    assert.equal(problems[0].code, code, field);
    assert.equal(problems[0].path, field);
  }
});

test("unknown and missing fields are reported", () => {
  const extra = { ...defaults(), surprise: 1 };
  assert.equal(validateDraft(schemaFields(), extra)[0].code, "settings.field_unknown");

  const missing = { ...defaults() };
  delete missing.vault;
  assert.equal(validateDraft(schemaFields(), missing)[0].code, "settings.field_missing");
});

test("nested limits are checked at their own path", () => {
  const draft = setPath(defaults(), "resource_limits.daemon_rss_mb", 1);
  const problems = validateDraft(schemaFields(), draft);
  assert.equal(problems.length, 1);
  assert.equal(problems[0].code, "settings.out_of_range");
  assert.equal(problems[0].path, "resource_limits.daemon_rss_mb");
});

test("changedFields reports exactly what the user touched", () => {
  const saved = defaults();
  assert.deepEqual(changedFields(saved, saved), []);

  const draft = setPath({ ...saved, locale: "en" }, "resource_limits.scan_files", 50000);
  assert.deepEqual(changedFields(saved, draft), ["locale", "resource_limits.scan_files"]);
});

test("setPath does not mutate the document it was given", () => {
  const saved = defaults();
  const draft = setPath(saved, "retention_days", 90);
  assert.equal(saved.retention_days, 30, "the saved document must stay untouched");
  assert.equal(draft.retention_days, 90);
});

test("an unrecognised spec is refused rather than rendered blindly", () => {
  const problems = validateDraft({ mystery: { kind: "colour" } }, { mystery: "red" });
  assert.equal(problems[0].code, "settings.spec_unknown");
});

test("projectToSchema drops envelope keys so a read can be written back", () => {
  // A settings GET arrives with response-envelope keys attached.
  const fromApi = { ...defaults(), snapshot_digest: "abc", schema_version: 1 };
  assert.notDeepEqual(validateDraft(schemaFields(), fromApi), []);

  const projected = projectToSchema(schemaFields(), fromApi);
  assert.deepEqual(validateDraft(schemaFields(), projected), []);
  assert.deepEqual(projected, defaults());
});

test("projectToSchema keeps a genuinely missing field missing", () => {
  const partial = { ...defaults() };
  delete partial.vault;
  const projected = projectToSchema(schemaFields(), partial);
  assert.ok(!Object.prototype.hasOwnProperty.call(projected, "vault"));
  assert.equal(validateDraft(schemaFields(), projected)[0].code, "settings.field_missing");
});
