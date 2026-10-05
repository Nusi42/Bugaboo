import type { Schema } from "@bugaboo/api-client";
import { describe, expect, it } from "vitest";

import { certificateDays, isUnanchored, levelOf, serverNow } from "./status.ts";

type State = Schema<"ServerStatusBody">;

const NOW = Date.parse("2026-10-05T12:00:00Z");

/** A server that is fine, with a certificate for a year and a log that has an anchor. */
function healthy(): State {
  return {
    version: "0.0.0",
    started_at: "2026-10-05T08:00:00Z",
    uptime_seconds: 3600,
    database: { kind: "built_in", ok: true },
    certificate: { expires_at: "2027-10-05T12:00:00Z" },
    security_log: {
      result: "valid",
      entries: 12,
      head_seq: 12,
      head_hash: "abababababababab",
      anchored: true,
    },
  };
}

describe("ops_08 how serious the state of the server is", () => {
  it("is fine when everything is fine", () => {
    expect(levelOf(healthy(), NOW)).toBe("ok");
  });

  it("asks for a look when the certificate ends soon, or nothing vouches for the log", () => {
    const soon: State = { ...healthy(), certificate: { expires_at: "2026-10-20T12:00:00Z" } };
    expect(levelOf(soon, NOW)).toBe("warning");
    const log = { ...healthy().security_log, anchored: false };
    expect(isUnanchored(log)).toBe(true);
    expect(levelOf({ ...healthy(), security_log: log }, NOW)).toBe("warning");
    // An empty log needs no anchor.
    expect(isUnanchored({ ...log, entries: 0 })).toBe(false);
  });

  it("is a problem when the database is silent, the log is broken or the certificate has run out", () => {
    expect(levelOf({ ...healthy(), database: { kind: "built_in", ok: false } }, NOW)).toBe(
      "problem",
    );
    const broken: State = {
      ...healthy(),
      security_log: { result: "broken", broken_at: 7, reason: "gap" },
    };
    expect(levelOf(broken, NOW)).toBe("problem");
    const unreadable: State = { ...healthy(), security_log: { result: "unreadable" } };
    expect(levelOf(unreadable, NOW)).toBe("problem");
    const expired: State = { ...healthy(), certificate: { expires_at: "2026-10-01T12:00:00Z" } };
    expect(levelOf(expired, NOW)).toBe("problem");
  });

  it("takes the time of the server from when it started and how long it has run", () => {
    expect(serverNow(healthy())).toBe(Date.parse("2026-10-05T09:00:00Z"));
  });

  it("does not judge a certificate that is not shown", () => {
    const state: State = { ...healthy(), certificate: null };
    expect(certificateDays(state, NOW)).toBeNull();
    expect(levelOf(state, NOW)).toBe("ok");
  });
});
