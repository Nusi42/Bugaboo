import type { Schema } from "@bugaboo/api-client";
import { describe, expect, it } from "vitest";

import {
  certificateDays,
  hasWriteFailures,
  isUnanchored,
  levelOf,
  openParts,
  serverNow,
  unknownParts,
} from "./status.ts";

type State = Schema<"ServerStatusBody">;

const NOW = Date.parse("2026-10-05T12:00:00Z");

/** A server that is fine, with a certificate for a year and a log that has an anchor. */
function healthy(): State {
  return {
    version: "0.0.0",
    started_at: "2026-10-05T08:00:00Z",
    uptime_seconds: 3600,
    database: { kind: "built-in", ok: true },
    certificate: { expires_at: "2027-10-05T12:00:00Z" },
    security_log: {
      result: "valid",
      entries: 12,
      head_seq: 12,
      head_hash: "abababababababab",
      anchored: true,
      write_failures: 0,
    },
    data_folder_open: {
      folder: "private",
      database: "private",
      files: "private",
      backups: "private",
      setup_code_file: "private",
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
    expect(levelOf({ ...healthy(), database: { kind: "built-in", ok: false } }, NOW)).toBe(
      "problem",
    );
    const broken: State = {
      ...healthy(),
      security_log: { result: "broken", broken_at: 7, reason: "gap", write_failures: 0 },
    };
    expect(levelOf(broken, NOW)).toBe("problem");
    const unreadable: State = {
      ...healthy(),
      security_log: { result: "unreadable", write_failures: 0 },
    };
    const behind: State = {
      ...healthy(),
      security_log: { result: "broken", broken_at: 9, reason: "behind", write_failures: 0 },
    };
    expect(levelOf(behind, NOW)).toBe("problem");
    expect(levelOf(unreadable, NOW)).toBe("problem");
    const expired: State = { ...healthy(), certificate: { expires_at: "2026-10-01T12:00:00Z" } };
    expect(levelOf(expired, NOW)).toBe("problem");
  });
});

describe("sec_15 and ops_08 what the page makes of the server", () => {
  it("asks for a look when a security log entry could not be written", () => {
    const log = { ...healthy().security_log, write_failures: 2 };
    expect(hasWriteFailures(log)).toBe(true);
    expect(hasWriteFailures(healthy().security_log)).toBe(false);
    expect(levelOf({ ...healthy(), security_log: log }, NOW)).toBe("warning");
  });

  it("asks for a look when other accounts can open a part of the data folder", () => {
    const open: State["data_folder_open"] = {
      ...healthy().data_folder_open,
      database: "open",
      backups: "open",
    };
    expect(openParts(healthy().data_folder_open)).toEqual([]);
    expect(openParts(open)).toEqual(["database", "backups"]);
    expect(levelOf({ ...healthy(), data_folder_open: open }, NOW)).toBe("warning");
  });

  it("asks for a look, and never says all is well, when the rights could not be read", () => {
    const unknown: State["data_folder_open"] = {
      ...healthy().data_folder_open,
      files: "unknown",
    };
    expect(openParts(unknown)).toEqual([]);
    expect(unknownParts(unknown)).toEqual(["files"]);
    expect(levelOf({ ...healthy(), data_folder_open: unknown }, NOW)).toBe("warning");
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
