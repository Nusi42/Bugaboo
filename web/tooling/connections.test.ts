import { describe, expect, it } from "vitest";

import {
  ConnectionGuard,
  countConnections,
  leftOver,
  MOST_LEFTOVER_CONNECTIONS,
  type Counts,
  type Sample,
} from "./connections.ts";

const FROM = 47410;
const TO = 47429;

const netstat = `
Active Connections

  Proto  Local Address          Foreign Address        State
  TCP    0.0.0.0:135            0.0.0.0:0              LISTENING
  TCP    127.0.0.1:47410        127.0.0.1:50001        ESTABLISHED
  TCP    127.0.0.1:50001        127.0.0.1:47410        ESTABLISHED
  TCP    127.0.0.1:50002        127.0.0.1:47410        TIME_WAIT
  TCP    127.0.0.1:50003        127.0.0.1:47411        TIME_WAIT
  TCP    127.0.0.1:47411        127.0.0.1:50004        CLOSE_WAIT
  TCP    [::1]:50005            [::1]:47411            TIME_WAIT
  TCP    127.0.0.1:50006        127.0.0.1:8080         TIME_WAIT
  TCP    127.0.0.1:47500        127.0.0.1:50007        TIME_WAIT
  UDP    0.0.0.0:5353           *:*
`;

const linux = `
State      Recv-Q Send-Q Local Address:Port  Peer Address:Port
ESTAB      0      0      127.0.0.1:50001     127.0.0.1:47410
ESTAB      0      0      127.0.0.1:47410     127.0.0.1:50001
TIME-WAIT  0      0      127.0.0.1:50002     127.0.0.1:47412
CLOSE-WAIT 1      0      [::1]:47412         [::1]:50009
LISTEN     0      128    0.0.0.0:47412       0.0.0.0:*
`;

const powershell = `
127.0.0.1:47410 127.0.0.1:50001 Established
127.0.0.1:50001 127.0.0.1:47410 Established
127.0.0.1:50002 127.0.0.1:47410 TimeWait
::1:47411 ::1:50003 CloseWait
0.0.0.0:47412 0.0.0.0:0 Listen
`;

/** A sample with these counts on one port. */
function sampleOf(port: number, counts: Counts): Sample {
  return { total: counts, byPort: { [port]: counts }, machineTimeWait: 0 };
}

const calm: Counts = { established: 4, timeWait: 20, closeWait: 0 };

describe("test_02 the connection guard counts what a run leaves behind (M4 WP-03)", () => {
  it("counts the connections of the servers of the workers from the list of netstat", () => {
    const { total, byPort } = countConnections(netstat, FROM, TO);
    // One open connection (not twice), three in TIME_WAIT at the tests' end or the server's end,
    // one in CLOSE_WAIT. The other ports and the listening and UDP lines are not the run's.
    expect(total).toEqual({ established: 1, timeWait: 3, closeWait: 1 });
    expect(byPort[47410]).toEqual({ established: 1, timeWait: 1, closeWait: 0 });
    expect(byPort[47411]).toEqual({ established: 0, timeWait: 2, closeWait: 1 });
  });

  it("counts the same from a Linux list, with its spelling of the states", () => {
    const { total, byPort } = countConnections(linux, FROM, TO);
    expect(total).toEqual({ established: 1, timeWait: 1, closeWait: 1 });
    expect(byPort[47412]).toEqual({ established: 0, timeWait: 1, closeWait: 1 });
  });

  it("counts the same from the lines that PowerShell prints, whatever the language of Windows", () => {
    const { total } = countConnections(powershell, FROM, TO);
    expect(total).toEqual({ established: 1, timeWait: 1, closeWait: 1 });
  });

  it("counts nothing for a list without a connection of the run", () => {
    expect(countConnections("", FROM, TO).total).toEqual({
      established: 0,
      timeWait: 0,
      closeWait: 0,
    });
  });
});

describe("test_03 the connection guard fails a run over its limit (M4 WP-03)", () => {
  it("passes a run that stays under the limit, and says what it reached", () => {
    const guard = new ConnectionGuard(100);
    guard.record(sampleOf(47410, calm), { 47410: "issues.spec.ts [desktop]" });
    guard.record(sampleOf(47410, { ...calm, timeWait: 99 }), { 47410: "issues.spec.ts [desktop]" });
    expect(guard.verdict()).toBeUndefined();
    expect(guard.summary()).toContain("at most 99 left over");
    expect(guard.summary()).toContain("the limit is 100");
  });

  it("fails a run over the limit with a sentence that names the peak and the spec", () => {
    const guard = new ConnectionGuard(100);
    guard.record(sampleOf(47410, calm), { 47410: "a.spec.ts [desktop]" });
    guard.record(sampleOf(47413, { established: 2, timeWait: 90, closeWait: 20 }), {
      47410: "a.spec.ts [desktop]",
      47413: "crowd.spec.ts [phone]",
    });
    guard.record(sampleOf(47410, calm), { 47410: "a.spec.ts [desktop]" });
    const sentence = guard.verdict();
    expect(sentence).toContain("110 connections");
    expect(sentence).toContain("limit is 100");
    expect(sentence).toContain("crowd.spec.ts [phone]");
    expect(sentence).toContain("port 47413");
  });
});

describe("test_07 the connection guard does not pass what it could not see (M4 WP-03)", () => {
  it("fails a run that could not look at all, instead of passing unseen", () => {
    expect(new ConnectionGuard(100).verdict()).toContain("could not be counted");
  });

  it("fails a run in which any look failed, and says how many", () => {
    const guard = new ConnectionGuard(100);
    guard.record(sampleOf(47410, calm), {});
    guard.recordFailure();
    expect(guard.verdict()).toContain("1 looks");
    expect(guard.summary()).toContain("(1 failed)");
  });

  it("tells how many connections in TIME_WAIT the whole computer had, and does not fail for it", () => {
    const { machineTimeWait } = countConnections(netstat, FROM, TO);
    // Of the lines of the list, the ones in TIME_WAIT with a local port of the dynamic range.
    expect(machineTimeWait).toBe(4);
    const guard = new ConnectionGuard(100);
    guard.record({ ...sampleOf(47410, calm), machineTimeWait: 9000 }, {});
    expect(guard.verdict()).toBeUndefined();
    expect(guard.summary()).toContain("the whole computer had at most 9000 in TIME_WAIT");
  });

  it("adds closed connections only to the left-over count", () => {
    expect(leftOver({ established: 50, timeWait: 3, closeWait: 2 })).toBe(5);
  });

  it("has a limit that is a positive whole number, set once", () => {
    expect(Number.isInteger(MOST_LEFTOVER_CONNECTIONS)).toBe(true);
    expect(MOST_LEFTOVER_CONNECTIONS).toBeGreaterThan(0);
  });
});
