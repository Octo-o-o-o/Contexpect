// Route paths parsed from routes.ts, so the contract test and the router
// cannot drift apart.
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const root = dirname(fileURLToPath(import.meta.url));
const source = readFileSync(join(root, "../src/routes.ts"), "utf8");
const block = source.slice(source.indexOf("export const ROUTES"), source.indexOf("export const NAV"));

export const ROUTE_PATHS = [...block.matchAll(/path:\s*"([^"]+)"/g)].map((match) => match[1]);
