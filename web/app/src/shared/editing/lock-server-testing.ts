// A stand-in for the lock on the server, for the tests of the editors: like the real one it gives
// the same editor the same token again (`Taking::decide`), and a release that is still on its way
// takes the lock away when it arrives, whoever asked for it last. Releases arrive when the test
// says so.
import type { LockOutcome } from "@bugaboo/api-client";

import { taken } from "./long-text-testing.ts";

export class LockServer {
  /** The token of the lock now, if there is one. */
  held: string | null = null;
  /** The calls that reached the server, in order. */
  readonly calls: string[] = [];
  private readonly arriving: (() => void)[] = [];

  /** Takes the lock, or gives the same editor the token it has. */
  readonly take = (): Promise<LockOutcome> => {
    this.calls.push("take");
    this.held ??= "T";
    return Promise.resolve(taken(this.held));
  };

  /** A release that reaches the server only at `arrive`. */
  readonly release = (token: string): Promise<void> => {
    this.calls.push(`release ${token}`);
    return new Promise<void>((done) => {
      this.arriving.push(() => {
        if (this.held === token) {
          this.held = null;
        }
        done();
      });
    });
  };

  /** The oldest release that is on its way reaches the server. */
  arrive(): void {
    this.arriving.shift()?.();
  }
}
