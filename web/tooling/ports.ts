import { readFileSync } from "node:fs";
import { resolve } from "node:path";

import defaults from "../../tools/dev/ports.json" with { type: "json" };

/** The work folder: the one place that holds `tools/dev/ports.json`, `messages/` and `design/`. */
export const repoRoot = resolve(import.meta.dirname, "../..");

/** The port names are the keys of `tools/dev/ports.json`; nothing here repeats them. */
export type Ports = { readonly [Name in Exclude<keyof typeof defaults, `$${string}`>]: number };

const portNames = Object.keys(defaults).filter((name) => !name.startsWith("$"));

function isPort(value: unknown): value is number {
  return typeof value === "number" && Number.isInteger(value) && value > 0 && value < 65536;
}

function hasEveryPort(value: unknown): value is Ports {
  return (
    typeof value === "object" &&
    value !== null &&
    portNames.every((name) => isPort(Reflect.get(value, name)))
  );
}

/**
 * Reads the development ports from `tools/dev/ports.json` (ADR-0023), or from the file named by
 * `BB_PORTS_FILE`. Every tool config uses this, so a port is defined exactly once.
 */
export function readPorts(): Ports {
  const file = process.env["BB_PORTS_FILE"] ?? resolve(repoRoot, "tools/dev/ports.json");
  const parsed: unknown = JSON.parse(readFileSync(file, "utf8"));
  if (!hasEveryPort(parsed)) {
    throw new Error(`${file} must give a port number for each of: ${portNames.join(", ")}.`);
  }
  return parsed;
}
