import assert from "node:assert/strict";
import test from "node:test";
import { en, zh } from "../src/i18n-tables.js";

test("zh and en cover the same keys with different values", () => {
  const zhKeys = Object.keys(zh);
  const enKeys = Object.keys(en);
  assert.deepEqual(zhKeys, enKeys);
  assert.ok(zhKeys.length > 0);
  for (const key of zhKeys) {
    assert.notEqual(zh[key], en[key], key);
    assert.match(zh[key], /[\u4e00-\u9fff]/, key);
  }
});
