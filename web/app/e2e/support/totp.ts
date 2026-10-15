import { msPerSecond } from "@bugaboo/api-client";
import { createHmac } from "node:crypto";
import { setTimeout as sleep } from "node:timers/promises";

/** A code lasts this many seconds (RFC 6238). */
const STEP_SECONDS = 30;
const DIGITS = 6;
const BASE32 = "ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";

/** The bytes of a secret that a code app is given as base32 text. */
function decode(secret: string): Buffer {
  const bits = secret
    .replace(/=+$/u, "")
    .toUpperCase()
    .split("")
    .map((symbol) => BASE32.indexOf(symbol).toString(2).padStart(5, "0"))
    .join("");
  const bytes = (bits.match(/.{8}/gu) ?? []).map((byte) => Number.parseInt(byte, 2));
  return Buffer.from(bytes);
}

/** The step of the clock that `moment` (milliseconds) falls into. */
function stepOf(moment: number): number {
  return Math.floor(moment / 1000 / STEP_SECONDS);
}

/** The six-digit code that a code app shows for `step`. */
function codeAt(secret: string, step: number): string {
  const counter = Buffer.alloc(8);
  counter.writeBigUInt64BE(BigInt(step));
  const digest = createHmac("sha1", decode(secret)).update(counter).digest();
  const offset = (digest.at(-1) ?? 0) & 0x0f;
  const value = digest.readUInt32BE(offset) & 0x7fffffff;
  return String(value % 10 ** DIGITS).padStart(DIGITS, "0");
}

/** A code app for the tests. */
export interface CodeApp {
  /**
   * The next code. The server accepts each code once, for the current step and the one before and
   * after it. So the app gives the code of the step after the newest one it gave, and waits when
   * that step is not yet near enough (never more than one step).
   */
  readonly next: () => Promise<string>;
}

/** A code app that was given `secret`, as base32 text. */
export function codeApp(secret: string): CodeApp {
  let last = 0;
  return {
    next: async () => {
      const wanted = Math.max(last + 1, stepOf(Date.now()));
      // The server accepts a code up to one step ahead of its clock.
      const ready = (wanted - 1) * STEP_SECONDS * msPerSecond;
      await sleep(Math.max(0, ready - Date.now() + 100));
      last = wanted;
      return codeAt(secret, wanted);
    },
  };
}
