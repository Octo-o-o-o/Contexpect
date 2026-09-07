const headers = { "X-Ctxpect-Client": "desktop", "Content-Type": "application/json" };

export type Json = Record<string, unknown>;

/**
 * A request outcome with its reason code intact.
 *
 * `kind` separates "the daemon could not be reached" from "the daemon
 * refused or failed", because C04 renders those as different states.
 */
export type ApiResult =
  | { ok: true; data: unknown }
  | { ok: false; kind: "transport" | "envelope"; code: string; message: string };

export function asObj(v: unknown): Json {
  return v && typeof v === "object" && !Array.isArray(v) ? (v as Json) : {};
}

function envelopeError(data: unknown): { code: string; message: string } | null {
  const err = asObj(asObj(data).error);
  if (typeof err.code === "string" && err.code.length > 0) {
    return { code: err.code, message: typeof err.message === "string" ? err.message : "" };
  }
  return null;
}

/**
 * Perform a request and keep the reason code.
 *
 * Nothing is thrown: a refusal is data the page has to render, and collapsing
 * it into an exception message is what lost the reason code before.
 */
export async function requestJson(path: string, init?: RequestInit): Promise<ApiResult> {
  let data: unknown;
  try {
    const res = await fetch(path, { headers, ...init });
    data = await res.json();
  } catch (error: unknown) {
    // Includes AbortError; a cancelled request is reported as such rather
    // than as an unreachable daemon.
    return {
      ok: false,
      kind: "transport",
      code:
        error instanceof DOMException && error.name === "AbortError"
          ? "api.cancelled"
          : "api.unreachable",
      message: String(error),
    };
  }
  const failure = envelopeError(data);
  if (failure) {
    return { ok: false, kind: "envelope", ...failure };
  }
  return { ok: true, data };
}

function throwIfError(result: ApiResult): unknown {
  if (result.ok) return result.data;
  throw new Error(result.code + (result.message ? `: ${result.message}` : ""));
}

export async function getJson(path: string): Promise<unknown> {
  return throwIfError(await requestJson(path));
}

export async function postJson(path: string, body: unknown): Promise<unknown> {
  return throwIfError(await requestJson(path, { method: "POST", body: JSON.stringify(body) }));
}
