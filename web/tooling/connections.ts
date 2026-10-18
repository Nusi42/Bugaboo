/**
 * The connections a run of the browser tests leaves behind, and the limit that guards them
 * (TEST-CONNECTIONS, M4 WP-03). A connection that is closed stays in TIME_WAIT for about two
 * minutes, and one that the other side closed but this side did not stays in CLOSE_WAIT; on
 * Windows a computer that has too many of them answers `net::ERR_NO_BUFFER_SPACE`. The pure part
 * is here (reading a list of connections, judging); the reporter that asks the computer is
 * `connection-guard.ts`.
 */

/**
 * The most connections in `TIME_WAIT` and `CLOSE_WAIT` that the servers of the tests (the ports of
 * their workers) may have at the same moment during a run. A run peaked at about 1,250 after M4
 * WP-03 and at 1,989 before it (architecture §11), so a fall back to the old way of testing is
 * seen. Not to be raised to make a run pass.
 */
export const MOST_LEFTOVER_CONNECTIONS = 1_800;

/**
 * More connections in TIME_WAIT than this on the whole computer mean that other work (another work
 * folder testing, a build) is on it. It says so and fails nothing: what is left over by other
 * folders is not what the run did (M4-F22). A computer on which the tests of a folder ran alone
 * had under 100; one on which other folders tested had over 4,000.
 */
export const BUSY_COMPUTER_TIME_WAIT = 3_000;

/** The pause after a look at the computer before the next one starts (a look takes seconds). */
export const PAUSE_BETWEEN_LOOKS_MS = 1000;

/** The first port that Windows gives to the connections that programs open (its dynamic range). */
export const FIRST_DYNAMIC_PORT = 49_152;

/** What the guard counts of one worker's server. */
export interface Counts {
  /** Connections of the tests that are open (counted at the end that connects to the server). */
  readonly established: number;
  /** Closed by the side of the tests or the server and not yet let go by the system. */
  readonly timeWait: number;
  /** Closed by the other side and not yet closed here. */
  readonly closeWait: number;
}

/** One look at the computer: the counts of every worker's port and their sum. */
export interface Sample {
  readonly total: Counts;
  readonly byPort: Readonly<Record<number, Counts>>;
  /** Information only: the connections in TIME_WAIT of the whole computer, whoever owns them. */
  readonly machineTimeWait: number;
}

const NONE: Counts = { established: 0, timeWait: 0, closeWait: 0 };

/**
 * The states that the guard reads, as the systems spell them (`TIME_WAIT`, `TIME-WAIT`, `ESTAB`,
 * `TimeWait` of PowerShell). `netstat` of Windows prints them in the language of the system, which
 * is why the reporter asks PowerShell there.
 */
function stateOf(word: string): keyof Counts | undefined {
  switch (word.toUpperCase().replaceAll(/[-_]/gu, "")) {
    case "ESTABLISHED":
    case "ESTAB":
      return "established";
    case "TIMEWAIT":
      return "timeWait";
    case "CLOSEWAIT":
      return "closeWait";
    default:
      return undefined;
  }
}

const PORT_AT_END = /:(\d+)$/u;

function portOf(address: string): number | undefined {
  const found = PORT_AT_END.exec(address);
  return found?.[1] === undefined ? undefined : Number(found[1]);
}

function added(counts: Counts | undefined, state: keyof Counts): Counts {
  const before = counts ?? NONE;
  return { ...before, [state]: before[state] + 1 };
}

/**
 * Reads the list that `ss -tan` or `netstat -ant` (Linux) or the PowerShell line of the reporter (Windows) prints, and
 * counts the connections of the workers' servers, which listen on ports `from` to `to`. A line is
 * the local address, the remote address and a state, in that order. A connection counts for the
 * server whose port is one of its two ends; an open one is counted only at the end that connects,
 * so that it is not counted twice (both ends of a connection on this computer are in the list).
 */
export function countConnections(listing: string, from: number, to: number): Sample {
  const byPort: Record<number, Counts> = {};
  let total = NONE;
  let machineTimeWait = 0;
  const inRange = (port: number | undefined): port is number =>
    port !== undefined && port >= from && port <= to;
  for (const line of listing.split("\n")) {
    const words = line.trim().split(/\s+/u);
    const state = words.map((word) => stateOf(word)).find((found) => found !== undefined);
    const ends = words.filter((word) => PORT_AT_END.test(word)).map((word) => portOf(word));
    const [local, remote] = ends;
    if (state === undefined || ends.length < 2) {
      continue;
    }
    if (state === "timeWait" && local !== undefined && local >= FIRST_DYNAMIC_PORT) {
      machineTimeWait += 1;
    }
    const server = inRange(remote) ? remote : inRange(local) ? local : undefined;
    // An open connection is two lines on this computer; the one whose remote end is the server
    // is the one of the tests.
    if (server === undefined || (state === "established" && server !== remote)) {
      continue;
    }
    byPort[server] = added(byPort[server], state);
    total = added(total, state);
  }
  return { total, byPort, machineTimeWait };
}

/** What counts against the limit: connections that were closed and are not let go yet. */
export function leftOver(counts: Counts): number {
  return counts.timeWait + counts.closeWait;
}

/** Where the most connections were, and while which spec ran there. */
interface Peak {
  readonly left: number;
  readonly counts: Counts;
  readonly port: number | undefined;
  readonly spec: string;
}

/** What the run reached, kept look by look; judged at the end. */
export class ConnectionGuard {
  private peak: Peak = { left: 0, counts: NONE, port: undefined, spec: "" };
  private mostOpen = 0;
  private mostOnTheComputer = 0;
  private looks = 0;
  private failedLooks = 0;
  /** The specs that failed because the computer ran out of ports. */
  private readonly outOfPortsIn: string[] = [];

  private readonly limit: number;

  public constructor(limit = MOST_LEFTOVER_CONNECTIONS) {
    this.limit = limit;
  }

  /** Notes a look that could not be made (the command that lists the connections failed). */
  public recordFailure(): void {
    this.failedLooks += 1;
  }

  /** Takes one look; `specs` says, for each worker port, which spec its worker ran last. */
  public record(sample: Sample, specs: Readonly<Record<number, string>>): void {
    this.looks += 1;
    this.mostOpen = Math.max(this.mostOpen, sample.total.established);
    this.mostOnTheComputer = Math.max(this.mostOnTheComputer, sample.machineTimeWait);
    const left = leftOver(sample.total);
    if (left <= this.peak.left) {
      return;
    }
    let worst: number | undefined;
    for (const [name, counts] of Object.entries(sample.byPort)) {
      if (worst === undefined || leftOver(counts) > leftOver(sample.byPort[worst] ?? NONE)) {
        worst = Number(name);
      }
    }
    this.peak = {
      left,
      counts: sample.total,
      port: worst,
      spec: worst === undefined ? "" : (specs[worst] ?? ""),
    };
  }

  /** Notes that a test failed with `net::ERR_NO_BUFFER_SPACE` (the computer has no ports left). */
  public recordOutOfPorts(spec: string): void {
    this.outOfPortsIn.push(spec);
  }

  /** The line that says other work is on the computer, when it is; else nothing. */
  private busyNote(): string {
    return this.mostOnTheComputer > BUSY_COMPUTER_TIME_WAIT
      ? ` Other work is on this computer: it held ${String(this.mostOnTheComputer)} in TIME_WAIT.`
      : "";
  }

  /**
   * The sentence about a run in which a test ran out of ports, with the count of the whole computer
   * next to the count of the run, so a reviewer sees whether it was the run or the neighbours; `undefined`
   * when none did. It does not fail the run: the test that failed has done that.
   */
  public outOfPorts(): string | undefined {
    if (this.outOfPortsIn.length === 0) {
      return undefined;
    }
    return (
      `${String(this.outOfPortsIn.length)} tests failed with net::ERR_NO_BUFFER_SPACE (the first in ${this.outOfPortsIn[0] ?? ""}). ` +
      `The tests of this run left at most ${String(this.peak.left)} connections in TIME_WAIT and CLOSE_WAIT; ` +
      `the whole computer had at most ${String(this.mostOnTheComputer)} in TIME_WAIT.${this.busyNote()}`
    );
  }

  /** What was reached, in one line for the end of the run. */
  public summary(): string {
    const { counts, port, spec } = this.peak;
    const where =
      port === undefined
        ? ""
        : ` Most at the peak: the server on port ${String(port)} (${spec === "" ? "no spec yet" : spec}).`;
    return (
      `Connections of the tests: at most ${String(this.peak.left)} left over at once ` +
      `(${String(counts.timeWait)} in TIME_WAIT, ${String(counts.closeWait)} in CLOSE_WAIT), ` +
      `at most ${String(this.mostOpen)} open, in ${String(this.looks)} looks (${String(this.failedLooks)} failed); ` +
      `the whole computer had at most ${String(this.mostOnTheComputer)} in TIME_WAIT (for information); ` +
      `the limit is ${String(this.limit)}.${where}${this.busyNote()}`
    );
  }

  /** The sentence that fails the run when the peak is over the limit, else `undefined`. */
  public verdict(): string | undefined {
    if (this.looks === 0 || this.failedLooks > 0) {
      return `The connections of the tests could not be counted in ${String(this.failedLooks)} looks (${String(this.looks)} worked), so nobody knows whether the run uses up the ports of the computer. Is PowerShell (Windows) or \`ss\` (Linux) working?`;
    }
    if (this.peak.left <= this.limit) {
      return undefined;
    }
    const spec = this.peak.spec === "" ? "no spec yet" : this.peak.spec;
    const where =
      this.peak.port === undefined
        ? ""
        : ` Most of them belong to the server on port ${String(this.peak.port)}, while its worker ran ${spec}.`;
    return (
      `The browser tests left ${String(this.peak.left)} connections in TIME_WAIT and CLOSE_WAIT at once ` +
      `(${String(this.peak.counts.timeWait)} and ${String(this.peak.counts.closeWait)}); the limit is ${String(this.limit)}. ` +
      `A computer with too many of them answers net::ERR_NO_BUFFER_SPACE.${where} ` +
      "Make the spec use fewer connections (one session per person, no probes in a loop); do not raise the limit."
    );
  }
}
