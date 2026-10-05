import { basename, resolve } from "node:path";

import layout from "../../tools/dev/ports.json" with { type: "json" };

/** The work folder: the one place that holds `tools/dev/ports.json`, `messages/` and `design/`. */
export const repoRoot = resolve(import.meta.dirname, "../..");

/** The port names are the keys of `offsets` in `tools/dev/ports.json`; nothing here repeats them. */
export type Ports = { readonly [Name in keyof typeof layout.offsets]: number };

const portNames = Object.keys(layout.offsets);

/** The variable that names the block of ports of a folder that is not called `bugaboo.winN`. */
export const BLOCK_VARIABLE = "BUGABOO_PORT_BLOCK";

/** `bugaboo.win3` is the third work folder (ADR-0024). */
const WORK_FOLDER = /^bugaboo\.win(\d+)$/u;

function isPorts(value: Readonly<Record<string, number>>): value is Ports {
  return portNames.every((name) => Number.isInteger(value[name]));
}

function validBlock(value: number, source: string): number {
  if (!Number.isInteger(value) || value < 1 || value > layout.blocks) {
    throw new Error(
      `${source} has no block of ports: tools/dev/ports.json gives blocks 1 to ${String(layout.blocks)}. ` +
        `Set ${BLOCK_VARIABLE} to one of them.`,
    );
  }
  return value;
}

/**
 * The block of ports for a folder (ADR-0023): the one `override` names (the value of
 * `BUGABOO_PORT_BLOCK`), else the N of a work folder `bugaboo.winN`, else block 1 (CI and every
 * other folder).
 */
export function blockOf(folderName: string, override?: string): number {
  if (override !== undefined && override !== "") {
    return validBlock(Number(override), `${BLOCK_VARIABLE}=${override}`);
  }
  const match = WORK_FOLDER.exec(folderName);
  return match === null ? 1 : validBlock(Number(match[1]), `The folder ${folderName}`);
}

/** The ports of one block: `first + (block - 1) * stride`, plus the offset of each name. */
export function portsOf(block: number): Ports {
  const start = layout.first + (validBlock(block, `Block ${String(block)}`) - 1) * layout.stride;
  const ports: Readonly<Record<string, number>> = Object.fromEntries(
    Object.entries(layout.offsets).map((entry: readonly [string, number]) => [
      entry[0],
      start + entry[1],
    ]),
  );
  if (!isPorts(ports)) {
    throw new Error("tools/dev/ports.json must give a number for every port.");
  }
  return ports;
}

/**
 * The development ports of this work folder (ADR-0023). Every tool config uses this, so a port is
 * defined exactly once and two work folders never ask for the same one.
 */
export function readPorts(): Ports {
  return portsOf(blockOf(basename(repoRoot), process.env[BLOCK_VARIABLE]));
}
