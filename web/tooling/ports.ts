import { readFileSync } from "node:fs";
import { resolve } from "node:path";

/** The work folder: the one place that holds `tools/dev/ports.json`, `messages/` and `design/`. */
export const repoRoot = resolve(import.meta.dirname, "../..");

export interface Ports {
  readonly server: number;
  readonly web: number;
  readonly webPreview: number;
  readonly e2eServer: number;
  readonly vitestUi: number;
  readonly e2eWorkersFrom: number;
  readonly e2eWorkersTo: number;
  readonly loadTestServer: number;
}

function isPort(value: unknown): value is number {
  return typeof value === "number" && Number.isInteger(value) && value > 0 && value < 65536;
}

/**
 * Reads the development ports from `tools/dev/ports.json` (ADR-0023), or from the file named by
 * `BB_PORTS_FILE`. Every tool config uses this, so a port is defined exactly once.
 */
export function readPorts(): Ports {
  const file = process.env["BB_PORTS_FILE"] ?? resolve(repoRoot, "tools/dev/ports.json");
  const parsed: unknown = JSON.parse(readFileSync(file, "utf8"));
  if (typeof parsed !== "object" || parsed === null) {
    throw new Error(`${file} must contain a JSON object of ports.`);
  }
  const port = (name: keyof Ports): number => {
    const value: unknown = Reflect.get(parsed, name);
    if (!isPort(value)) {
      throw new Error(`${file} must give a port number for "${name}".`);
    }
    return value;
  };
  return {
    server: port("server"),
    web: port("web"),
    webPreview: port("webPreview"),
    e2eServer: port("e2eServer"),
    vitestUi: port("vitestUi"),
    e2eWorkersFrom: port("e2eWorkersFrom"),
    e2eWorkersTo: port("e2eWorkersTo"),
    loadTestServer: port("loadTestServer"),
  };
}
