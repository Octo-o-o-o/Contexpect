// A request generation counter (C01/C40 pattern).
//
// Every fetch a page starts takes the next generation; a response is applied
// only if its generation is still the current one. A later navigation (new
// session id, new coordinate) bumps the counter, so an older request's answer
// can never overwrite the state of a newer one, however late it lands.
//
// Plain JS with a `.d.ts` sibling so `node --test` exercises the exact
// helper the UI runs.

/**
 * @returns {{ next(): number, isCurrent(generation: number): boolean, current(): number }}
 */
export function createGeneration() {
  let current = 0;
  return {
    /** Start a new generation; everything older is now stale. */
    next() {
      current += 1;
      return current;
    },
    /** Whether a response tagged with `generation` may still be applied. */
    isCurrent(generation) {
      return generation === current;
    },
    current() {
      return current;
    },
  };
}
