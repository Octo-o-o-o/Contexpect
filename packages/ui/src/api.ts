const headers = { "X-Ctxpect-Client": "desktop", "Content-Type": "application/json" };

export type Json = Record<string, unknown>;

export function asObj(v: unknown): Json {
  return v && typeof v === "object" && !Array.isArray(v) ? (v as Json) : {};
}

function throwIfError(data: unknown): unknown {
  const obj = asObj(data);
  const err = asObj(obj.error);
  if (typeof err.code === "string" && err.code.length > 0) {
    throw new Error(String(err.code) + (err.message ? `: ${String(err.message)}` : ""));
  }
  return data;
}

export async function getJson(path: string): Promise<unknown> {
  const res = await fetch(path, { headers });
  const data: unknown = await res.json();
  return throwIfError(data);
}

export async function postJson(path: string, body: unknown): Promise<unknown> {
  const res = await fetch(path, {
    method: "POST",
    headers,
    body: JSON.stringify(body),
  });
  const data: unknown = await res.json();
  return throwIfError(data);
}
