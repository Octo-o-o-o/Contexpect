import assert from "node:assert/strict";
import test from "node:test";

import { createGeneration } from "../src/generation.js";

test("a response from an older generation is dropped once a newer one started", () => {
  const gen = createGeneration();
  const first = gen.next();
  assert.equal(gen.isCurrent(first), true);
  const second = gen.next();
  // The first request's answer arrives late: it must not be applied.
  assert.equal(gen.isCurrent(first), false);
  assert.equal(gen.isCurrent(second), true);
  assert.equal(gen.current(), second);
});

test("generations are strictly increasing and never reused", () => {
  const gen = createGeneration();
  const seen = new Set();
  for (let i = 0; i < 5; i += 1) {
    const g = gen.next();
    assert.ok(!seen.has(g));
    seen.add(g);
  }
  assert.equal(gen.isCurrent(0), false, "the pre-first generation is never current");
});
